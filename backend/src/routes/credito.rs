use std::collections::HashMap;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Datelike, Months, NaiveDate, Utc};
use futures::StreamExt;
use mongodb::bson::{doc, oid::ObjectId, Document};

use crate::auth::EmpresaSession;
use crate::models::cliente::Cliente;
use crate::models::credito::{
    AutorizarReq, DashboardStats, EvaluarReq, EvaluarRes, Pago, PagoInfo, PlanPago,
    RegistrarAbonoReq, RegistrarPagoReq,
};
use crate::models::empresa::Empresa;
use crate::pdf::pdf_contrato;

fn tasa_por_plazo(meses: i32) -> f64 {
    match meses {
        1 => 0.07,
        3 => 0.09,
        6 => 0.12,
        9 => 0.15,
        12 => 0.18,
        // Inalcanzable vía endpoints: `validar_plazo_y_monto` filtra los plazos
        // válidos antes de llamar a esta función.
        _ => 0.05,
    }
}

// pub(crate): la tabla del contrato PDF se regenera con la MISMA fórmula.
pub(crate) fn generar_plan_pagos(monto: f64, plazo_meses: i32, tasa: f64) -> Vec<PagoInfo> {
    let total_interes = monto * tasa;
    let total_pagar = monto + total_interes;
    let pago_mensual = total_pagar / plazo_meses as f64;
    let capital_mensual = monto / plazo_meses as f64;
    let interes_mensual = total_interes / plazo_meses as f64;

    (1..=plazo_meses).map(|mes| {
        let saldo_restante = monto - capital_mensual * mes as f64;
        PagoInfo {
            mes,
            pago: (pago_mensual * 100.0).round() / 100.0,
            interes: (interes_mensual * 100.0).round() / 100.0,
            capital: (capital_mensual * 100.0).round() / 100.0,
            saldo_restante: if saldo_restante < 0.0 { 0.0 } else { (saldo_restante * 100.0).round() / 100.0 },
        }
    }).collect()
}

/// Cuota mensual canónica del contrato: `round(monto_total*(1+tasa)/plazo, 2)`.
/// `autorizar` y el PDF usan la misma fórmula; el body no es fuente de verdad (E2).
pub(crate) fn pago_mensual_de(monto_total: f64, plazo_meses: i32) -> f64 {
    redondear2(monto_total * (1.0 + tasa_por_plazo(plazo_meses)) / plazo_meses as f64)
}

// --- Ciclo de vida del plan (ola 4): funciones PURAS, testeadas sin DB ---

/// Vencimiento de la cuota n = fecha del plan (YYYY-MM-DD) + n meses.
/// `checked_add_months` ajusta al último día del mes válido (31-ene + 1m = 28-feb).
pub(crate) fn fecha_vencimiento(fecha_plan: &str, n: i32) -> Option<NaiveDate> {
    let d = NaiveDate::parse_from_str(fecha_plan, "%Y-%m-%d").ok()?;
    if n < 0 {
        return None;
    }
    d.checked_add_months(Months::new(n as u32))
}

// --- Saldo y estado por DINERO (ola 7): funciones PURAS, testeadas sin DB ---

/// Deuda total del plan = pago_mensual × plazo (el mismo total con interés
/// que ya usa `top_deudores`).
pub(crate) fn deuda(plan: &PlanPago) -> f64 {
    plan.pago_mensual * plan.plazo_meses as f64
}

/// Saldo pendiente = max(0, deuda − cobrado), a 2 decimales. `cobrado` es la
/// suma de TODOS los pagos y abonos del plan.
pub(crate) fn saldo(plan: &PlanPago, cobrado: f64) -> f64 {
    redondear2((deuda(plan) - cobrado).max(0.0))
}

/// Cuotas cubiertas por dinero = min(plazo, floor(cobrado / pago_mensual)).
/// El epsilon evita que pagar exacto la última cuota caiga a n−1 por error de
/// punto flotante.
pub(crate) fn cuotas_cubiertas(plan: &PlanPago, cobrado: f64) -> i32 {
    if plan.pago_mensual <= 0.0 {
        return 0;
    }
    let n = (cobrado / plan.pago_mensual + 1e-9).floor() as i32;
    n.clamp(0, plan.plazo_meses)
}

/// Cuotas vencidas (vencimiento < hoy) no cubiertas por dinero:
/// min(plazo, max(0, n_vencidas − cuotas_cubiertas)).
pub(crate) fn cuotas_vencidas(plan: &PlanPago, cobrado: f64, hoy: NaiveDate) -> i32 {
    let vencidas = (1..=plan.plazo_meses)
        .filter(|n| fecha_vencimiento(&plan.fecha, *n).map_or(false, |v| v < hoy))
        .count() as i32;
    (vencidas - cuotas_cubiertas(plan, cobrado)).clamp(0, plan.plazo_meses)
}

/// Estado v2 (ola 7), por dinero y no por cuotas marcadas: Liquidado si
/// saldo ≤ 0.01; si no, Moroso con ≥1 cuota vencida sin cubrir; si no, Activo.
pub(crate) fn estado_plan(plan: &PlanPago, cobrado: f64, hoy: NaiveDate) -> &'static str {
    if saldo(plan, cobrado) <= 0.01 {
        return "Liquidado";
    }
    if cuotas_vencidas(plan, cobrado, hoy) > 0 {
        "Moroso"
    } else {
        "Activo"
    }
}

/// `cobrado` de un plan = total de sus pagos/abonos en el mapa del tenant.
fn cobrado_de(plan: &PlanPago, pagos_por_plan: &HashMap<String, PagosPlan>) -> f64 {
    plan.id
        .as_ref()
        .and_then(|id| pagos_por_plan.get(&id.to_hex()))
        .map(|pp| pp.total)
        .unwrap_or(0.0)
}

/// Cuotas de un plan que vencen dentro de `dias` días (hoy incluido) sin pago.
pub(crate) fn cuotas_por_vencer(plan: &PlanPago, cuotas_pagadas: &[i32], hoy: NaiveDate, dias: i32) -> i32 {
    (1..=plan.plazo_meses)
        .filter(|n| {
            !cuotas_pagadas.contains(n)
                && fecha_vencimiento(&plan.fecha, *n)
                    .map_or(false, |v| v >= hoy && (v - hoy).num_days() <= dias as i64)
        })
        .count() as i32
}

/// Plan serializado para la API: `_id` hex + avance y estado recalculados en
/// lectura (cuotas cubiertas por dinero, cuotas vencidas, cobrado y saldo),
/// nunca persistidos. El `estado` persistido se sobrescribe con el v2.
fn plan_json(plan: &PlanPago, cobrado: f64, hoy: NaiveDate) -> serde_json::Value {
    let mut v = serde_json::to_value(plan).unwrap_or_else(|_| serde_json::json!({}));
    // ObjectId serializa como {"$oid": hex} con serde_json; el contrato pide
    // el hex string plano (el frontend lo manda tal cual al registrar pagos).
    if let Some(id) = &plan.id {
        v["_id"] = serde_json::json!(id.to_hex());
    }
    v["estado"] = serde_json::json!(estado_plan(plan, cobrado, hoy));
    v["cuotas_pagadas"] = serde_json::json!(cuotas_cubiertas(plan, cobrado));
    v["cuotas_vencidas"] = serde_json::json!(cuotas_vencidas(plan, cobrado, hoy));
    v["cobrado"] = serde_json::json!(redondear2(cobrado));
    v["saldo"] = serde_json::json!(saldo(plan, cobrado));
    v
}

/// Pagos de un plan (agrupados por hex del ObjectId): cuotas pagadas, total
/// cobrado y monto por fecha ("YYYY-MM-DD") para la serie y los KPIs por
/// ventana (ola 8).
#[derive(Default, Clone)]
struct PagosPlan {
    cuotas: Vec<i32>,
    total: f64,
    por_fecha: HashMap<String, f64>,
}

/// Cuotas pagadas de un plan (del mapa de pagos del tenant).
fn pagadas_de(plan: &PlanPago, pagos_por_plan: &HashMap<String, PagosPlan>) -> Vec<i32> {
    plan.id
        .as_ref()
        .and_then(|id| pagos_por_plan.get(&id.to_hex()))
        .map(|pp| pp.cuotas.clone())
        .unwrap_or_default()
}

/// ponytail: un handler que carga planes + pagos del tenant y calcula en
/// memoria es suficiente para el volumen de una PYME; techo: agregaciones de
/// Mongo si el volumen escala a decenas de miles de planes.
async fn cargar_cartera(
    client: &mongodb::Client,
    correo: &str,
) -> Result<(Vec<PlanPago>, HashMap<String, PagosPlan>), mongodb::error::Error> {
    let db = client.database("pymza");
    let mut planes = Vec::new();
    let mut cursor = db
        .collection::<PlanPago>("planes_pago")
        .find(doc! { "empresa": correo }, None)
        .await?;
    while let Some(plan) = cursor.next().await {
        planes.push(plan?);
    }
    let mut pagos: HashMap<String, PagosPlan> = HashMap::new();
    let mut cursor = db.collection::<Pago>("pagos").find(doc! { "empresa": correo }, None).await?;
    while let Some(pago) = cursor.next().await {
        let pago = pago?;
        let entrada = pagos.entry(pago.plan_id.to_hex()).or_default();
        entrada.cuotas.push(pago.cuota);
        entrada.total += pago.monto;
        // Monto por fecha ("YYYY-MM-DD"): la serie/KPIs por ventana recortan aquí.
        *entrada.por_fecha.entry(pago.fecha.clone()).or_insert(0.0) += pago.monto;
    }

    // Ola 8 (E1): reconcilia el contador operativo `cobrado` — campo BSON crudo
    // del plan (no vive en `PlanPago` para no romper el literal de tests de
    // `pdf.rs`, archivo de otro dueño) — contra el ledger. Solo al alza
    // (`$max`): bajar el contador a un ledger que aún no incluye un pago en
    // vuelo clobbearía una reserva atómica concurrente. Best-effort: si falla,
    // la lectura responde igual con el ledger.
    let mut operativo: HashMap<String, f64> = HashMap::new();
    let proyeccion = mongodb::options::FindOptions::builder()
        .projection(doc! { "cobrado": 1 })
        .build();
    let mut cursor = db
        .collection::<Document>("planes_pago")
        .find(doc! { "empresa": correo }, Some(proyeccion))
        .await?;
    while let Some(d) = cursor.next().await {
        let d = d?;
        if let Ok(oid) = d.get_object_id("_id") {
            operativo.insert(oid.to_hex(), d.get_f64("cobrado").unwrap_or(0.0));
        }
    }
    let coll_planes = db.collection::<PlanPago>("planes_pago");
    for plan in planes.iter() {
        let Some(id) = &plan.id else { continue };
        let hex = id.to_hex();
        let actual = operativo.get(&hex).copied().unwrap_or(0.0);
        let ledger = pagos.get(&hex).map(|pp| pp.total).unwrap_or(0.0);
        if ledger > actual + 1e-9 {
            if let Err(e) = coll_planes
                .update_one(
                    doc! { "_id": id, "empresa": correo },
                    doc! { "$max": { "cobrado": ledger } },
                    None,
                )
                .await
            {
                eprintln!("⚠️ No se pudo reconciliar `cobrado` del plan {hex}: {e}");
            }
        }
    }
    Ok((planes, pagos))
}

/// Recalcula y persiste las stats del dashboard del tenant (shape intacta:
/// {empresa, creditos_activos, capital_prestado, proximos_cobros}). Se llama
/// en `autorizar` y al registrar cada pago, así nunca se desincronizan:
/// - creditos_activos = planes con estado Activo o Moroso
/// - capital_prestado = suma de monto_total de todos los planes del tenant
/// - proximos_cobros = cuotas que vencen en ≤30 días de planes no liquidados
async fn upsert_dashboard_stats(
    client: &mongodb::Client,
    correo: &str,
    planes: &[PlanPago],
    pagos_por_plan: &HashMap<String, PagosPlan>,
) {
    let hoy = Utc::now().date_naive();
    // Estado RECALCULADO (ola 7): un plan cubierto por abonos cuenta como
    // Liquidado aunque el campo persistido aún diga Activo.
    let creditos_activos = planes
        .iter()
        .filter(|p| {
            let e = estado_plan(p, cobrado_de(p, pagos_por_plan), hoy);
            e == "Activo" || e == "Moroso"
        })
        .count() as i32;
    let capital_prestado: f64 = planes.iter().map(|p| p.monto_total).sum();
    let proximos_cobros: i32 = planes
        .iter()
        .filter(|p| estado_plan(p, cobrado_de(p, pagos_por_plan), hoy) != "Liquidado")
        .map(|p| cuotas_por_vencer(p, &pagadas_de(p, pagos_por_plan), hoy, 30))
        .sum();

    let coll = client.database("pymza").collection::<DashboardStats>("dashboard_stats");
    if let Err(e) = coll
        .update_one(
            doc! { "empresa": correo },
            doc! { "$set": {
                "empresa": correo,
                "creditos_activos": creditos_activos,
                "capital_prestado": capital_prestado,
                "proximos_cobros": proximos_cobros,
            } },
            mongodb::options::UpdateOptions::builder().upsert(true).build(),
        )
        .await
    {
        eprintln!("🚨 ERROR AL ACTUALIZAR DASHBOARD: {:?}", e);
    }
}

fn error_status(status: StatusCode, message: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "status": "error", "message": message })))
}

/// Ola 8-fix (E2): tope de negocio de `monto_total` en MXN. Por encima,
/// `pago_mensual_de` desborda a `inf` (el `*100` de `redondear2` desborda
/// ~1e308) y Mongo persiste `pago_mensual: Infinity`, que corrompe el dashboard
/// del tenant de forma permanente. 1e12 (un billón de pesos) no lo alcanza una
/// PYME real y deja margen de sobra para el redondeo.
const MONTO_MAX_MXN: f64 = 1e12;

/// Contrato del dominio (F1/F2, auditoría ola 6; plazos ola 7; tope ola 8-fix):
/// el plazo debe ser 1, 3, 6, 9 o 12 meses y el monto positivo, finito y
/// ≤ `MONTO_MAX_MXN`. Sin esto, `generar_plan_pagos` materializa un Vec de
/// `plazo_meses` elementos (i32::MAX → ~86 GB → OOM con 1 request) y
/// `autorizar` persiste el plan envenenado. Devuelve el mensaje del 400.
fn validar_plazo_y_monto(plazo_meses: i32, monto: f64) -> Option<&'static str> {
    if !matches!(plazo_meses, 1 | 3 | 6 | 9 | 12) {
        return Some("El plazo debe ser 1, 3, 6, 9 o 12 meses");
    }
    if !monto.is_finite() || monto <= 0.0 {
        return Some("El monto debe ser mayor a 0");
    }
    if monto > MONTO_MAX_MXN {
        return Some("El monto excede el máximo permitido");
    }
    None
}

// --- Resumen de cartera (ola 4): buckets y shape exacta del contrato ---

/// Bucket de antigüedad de una cuota vencida (días desde su vencimiento, ≥1).
pub(crate) fn bucket_aging(dias: i64) -> &'static str {
    match dias {
        0..=30 => "0-30",
        31..=60 => "31-60",
        61..=90 => "61-90",
        _ => "90+",
    }
}

/// Bucket de distribución de planes por `monto_total`.
pub(crate) fn bucket_monto(monto: f64) -> &'static str {
    if monto < 1000.0 {
        "0-1k"
    } else if monto < 5000.0 {
        "1k-5k"
    } else {
        "5k+"
    }
}

fn redondear2(x: f64) -> f64 {
    let r = (x * 100.0).round() / 100.0;
    // Normaliza -0.0 → 0.0 (una suma vacía puede salir como cero negativo).
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

/// Parsea una fecha `YYYY-MM-DD` (query `?desde`/`?hasta`). Función pura.
fn parsear_fecha(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Ventana [desde, hasta] de los query params. Requiere AMBAS fechas válidas;
/// si vienen invertidas las ordena. `None` si falta o es inválida cualquiera
/// (los handlers caen al comportamiento por defecto). Función pura, testeada.
fn rango_fechas(params: &HashMap<String, String>) -> Option<(NaiveDate, NaiveDate)> {
    let desde = params.get("desde").and_then(|s| parsear_fecha(s))?;
    let hasta = params.get("hasta").and_then(|s| parsear_fecha(s))?;
    Some(if desde <= hasta { (desde, hasta) } else { (hasta, desde) })
}

/// Top deudores (curp, saldo): saldo = pago_mensual × plazo − pagos
/// registrados; descendente, máximo 10. Solo planes con saldo pendiente
/// (un plan liquidado tiene saldo ~0 y queda fuera).
fn top_deudores(
    planes: &[PlanPago],
    pagos_por_plan: &HashMap<String, PagosPlan>,
) -> Vec<(String, f64)> {
    let mut deudores: Vec<(String, f64)> = planes
        .iter()
        .filter_map(|p| {
            let pagos = p.id.as_ref().and_then(|id| pagos_por_plan.get(&id.to_hex()));
            let total_pagado = pagos.map(|pp| pp.total).unwrap_or(0.0);
            let saldo = redondear2(p.pago_mensual * p.plazo_meses as f64 - total_pagado);
            (saldo > 0.0).then(|| (p.cliente_curp.clone(), saldo))
        })
        .collect();
    deudores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    deudores.truncate(10);
    deudores
}

/// Resumen de cartera del tenant con la shape EXACTA del contrato ola 4.
/// `nombres` mapea curp → nombre_completo (join con clientes en memoria).
/// `rango` (ola 8) limita la serie `cobrado_vs_por_cobrar` a la ventana
/// [desde, hasta]; sin él se conservan los 6 meses actual+5 previos. Todas las
/// particiones usan el estado RECALCULADO (cierra O1).
///
/// ponytail: cálculo íntegro en memoria (planes + pagos del tenant ya son
/// pocos miles de registros máx); techo: agregaciones de Mongo si escala.
fn resumen_cartera(
    planes: &[PlanPago],
    pagos_por_plan: &HashMap<String, PagosPlan>,
    nombres: &HashMap<String, String>,
    hoy: NaiveDate,
    rango: Option<(NaiveDate, NaiveDate)>,
) -> serde_json::Value {
    let estado = |p: &PlanPago| estado_plan(p, cobrado_de(p, pagos_por_plan), hoy);
    let no_liquidado = |p: &PlanPago| estado(p) != "Liquidado";

    // Buckets: los meses de la ventana si viene; si no, actual + 5 previos.
    let etiquetas: Vec<String> = match rango {
        Some((desde, hasta)) => {
            let mut meses = Vec::new();
            let mut m = NaiveDate::from_ymd_opt(desde.year(), desde.month(), 1).unwrap();
            let fin = NaiveDate::from_ymd_opt(hasta.year(), hasta.month(), 1).unwrap();
            // Cap defensivo: la ventana máxima de la UI es un semestre.
            while m <= fin && meses.len() < 60 {
                meses.push(m.format("%Y-%m").to_string());
                m = m.checked_add_months(Months::new(1)).unwrap();
            }
            meses
        }
        None => {
            let primero_mes = NaiveDate::from_ymd_opt(hoy.year(), hoy.month(), 1).unwrap();
            (0..6)
                .rev()
                .map(|k| primero_mes.checked_sub_months(Months::new(k)).unwrap().format("%Y-%m").to_string())
                .collect()
        }
    };
    let idx_mes: HashMap<&str, usize> = etiquetas
        .iter()
        .enumerate()
        .map(|(i, e)| (e.as_str(), i))
        .collect();
    let (desde_str, hasta_str) = match rango {
        Some((d, h)) => (d.format("%Y-%m-%d").to_string(), h.format("%Y-%m-%d").to_string()),
        None => (String::new(), String::new()),
    };
    let en_ventana = |fecha: &str| {
        rango.is_none() || (fecha >= desde_str.as_str() && fecha <= hasta_str.as_str())
    };

    // cobrado = pagos/abonos con fecha en la ventana (todos si no hay);
    // por_cobrar = cuotas esperadas dentro de la ventana de planes no
    // liquidados (estado recalculado).
    let mut cobrado = vec![0.0; etiquetas.len()];
    for pp in pagos_por_plan.values() {
        for (fecha, monto) in &pp.por_fecha {
            if !en_ventana(fecha) {
                continue;
            }
            if let Some(mes) = fecha.get(..7) {
                if let Some(&i) = idx_mes.get(mes) {
                    cobrado[i] += monto;
                }
            }
        }
    }
    let mut por_cobrar = vec![0.0; etiquetas.len()];
    for plan in planes.iter().filter(|p| no_liquidado(p)) {
        let pagadas = pagadas_de(plan, pagos_por_plan);
        for n in 1..=plan.plazo_meses {
            if pagadas.contains(&n) {
                continue;
            }
            if let Some(v) = fecha_vencimiento(&plan.fecha, n) {
                if let Some((d, h)) = rango {
                    if v < d || v > h {
                        continue;
                    }
                }
                if let Some(&i) = idx_mes.get(v.format("%Y-%m").to_string().as_str()) {
                    por_cobrar[i] += plan.pago_mensual;
                }
            }
        }
    }

    // Ola 8: morosidad por DINERO (cartera vencida / capital colocado),
    // consistente con el KPI del dashboard; ya no cuenta planes.
    let cartera_vencida: f64 = planes
        .iter()
        .filter(|p| estado(p) == "Moroso")
        .map(|p| saldo(p, cobrado_de(p, pagos_por_plan)))
        .sum();
    let capital_colocado: f64 = planes
        .iter()
        .filter(|p| no_liquidado(p))
        .map(|p| p.monto_total)
        .sum();
    let tasa_morosidad = if capital_colocado > 0.0 {
        cartera_vencida / capital_colocado
    } else {
        0.0
    };

    // Flujo proyectado: lectura literal del contrato — horizonte 30 = cuotas
    // que vencen en ≤30 días, 60 = ≤60, 90 = ≤90 (ventanas acumulativas) de
    // planes Activo/Moroso; monto = suma del pago_mensual de esas cuotas.
    let flujo_proyectado: Vec<serde_json::Value> = [30, 60, 90]
        .iter()
        .map(|&h| {
            let monto: f64 = planes
                .iter()
                .filter(|p| matches!(estado(p), "Activo" | "Moroso"))
                .map(|p| cuotas_por_vencer(p, &pagadas_de(p, pagos_por_plan), hoy, h) as f64 * p.pago_mensual)
                .sum();
            serde_json::json!({ "horizonte": h, "monto": redondear2(monto) })
        })
        .collect();

    // Aging: saldo vencido por antigüedad de la cuota (días desde vencimiento).
    let mut aging = vec![0.0; 4];
    for plan in planes.iter().filter(|p| no_liquidado(p)) {
        let pagadas = pagadas_de(plan, pagos_por_plan);
        for n in 1..=plan.plazo_meses {
            if pagadas.contains(&n) {
                continue;
            }
            if let Some(v) = fecha_vencimiento(&plan.fecha, n) {
                if v < hoy {
                    let dias = (hoy - v).num_days();
                    let i = match bucket_aging(dias) {
                        "0-30" => 0,
                        "31-60" => 1,
                        "61-90" => 2,
                        _ => 3,
                    };
                    aging[i] += plan.pago_mensual;
                }
            }
        }
    }
    let aging_json: Vec<serde_json::Value> = ["0-30", "31-60", "61-90", "90+"]
        .iter()
        .zip(aging.iter())
        .map(|(bucket, monto)| serde_json::json!({ "bucket": bucket, "monto": redondear2(*monto) }))
        .collect();

    let top_deudores_json: Vec<serde_json::Value> = top_deudores(planes, pagos_por_plan)
        .iter()
        .map(|(curp, saldo)| {
            // Cliente borrado de la red: el curp hace de nombre (nunca vacío).
            let nombre = nombres.get(curp).cloned().unwrap_or_else(|| curp.clone());
            serde_json::json!({ "cliente_curp": curp, "nombre": nombre, "saldo": saldo })
        })
        .collect();

    let mut dist = vec![0i32; 3];
    for plan in planes.iter() {
        let i = match bucket_monto(plan.monto_total) {
            "0-1k" => 0,
            "1k-5k" => 1,
            _ => 2,
        };
        dist[i] += 1;
    }
    let distribucion_json: Vec<serde_json::Value> = ["0-1k", "1k-5k", "5k+"]
        .iter()
        .zip(dist.iter())
        .map(|(bucket, n)| serde_json::json!({ "bucket": bucket, "n": n }))
        .collect();

    let cobrado_vs_por_cobrar: Vec<serde_json::Value> = etiquetas
        .iter()
        .enumerate()
        .map(|(i, mes)| {
            serde_json::json!({
                "mes": mes,
                "cobrado": redondear2(cobrado[i]),
                "por_cobrar": redondear2(por_cobrar[i]),
            })
        })
        .collect();

    serde_json::json!({
        "cobrado_vs_por_cobrar": cobrado_vs_por_cobrar,
        "tasa_morosidad": tasa_morosidad,
        "flujo_proyectado": flujo_proyectado,
        "aging": aging_json,
        "top_deudores": top_deudores_json,
        "distribucion_montos": distribucion_json,
    })
}

/// Resumen de cartera para las gráficas del dashboard (ola 4), del tenant del
/// token. Ola 8: acepta `?desde&hasta` para la serie `cobrado_vs_por_cobrar`.
pub async fn obtener_resumen(
    State(client): State<mongodb::Client>,
    sesion: EmpresaSession,
    Query(params): Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let (planes, pagos_por_plan) = match cargar_cartera(&client, &sesion.correo).await {
        Ok(cartera) => cartera,
        Err(e) => {
            eprintln!("🚨 ERROR MONGODB: {:?}", e);
            return Json(serde_json::json!({ "status": "error" }));
        }
    };
    // Join con clientes SOLO para los máx 10 deudores: lookup por curp
    // (ponytail — cargar toda la red para 10 nombres sería peor).
    let mut nombres: HashMap<String, String> = HashMap::new();
    let coll_clientes = client.database("pymza").collection::<Cliente>("clientes");
    for (curp, _) in top_deudores(&planes, &pagos_por_plan) {
        let nombre = match coll_clientes.find_one(doc! { "curp": &curp }, None).await {
            Ok(Some(c)) => c.nombre_completo,
            Ok(None) => curp.clone(), // cliente borrado: el curp hace de nombre
            Err(e) => {
                eprintln!("🚨 ERROR MONGODB (clientes): {:?}", e);
                curp.clone()
            }
        };
        nombres.insert(curp, nombre);
    }
    let resumen = resumen_cartera(
        &planes,
        &pagos_por_plan,
        &nombres,
        Utc::now().date_naive(),
        rango_fechas(&params),
    );
    Json(serde_json::json!({ "status": "success", "resumen": resumen }))
}

pub async fn evaluar_credito(
    State(client): State<mongodb::Client>,
    _sesion: EmpresaSession,
    Json(payload): Json<EvaluarReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Validación del dominio ANTES de tocar la base: un plazo gigante no debe
    // llegar a `generar_plan_pagos` (OOM, F1).
    if let Some(msg) = validar_plazo_y_monto(payload.plazo_meses, payload.monto) {
        return Err(error_status(StatusCode::BAD_REQUEST, msg));
    }
    let coll_clientes = client.database("pymza").collection::<Cliente>("clientes");

    match coll_clientes.find_one(
        mongodb::bson::doc! { "curp": &payload.curp },
        None
    ).await {
        Ok(Some(cliente)) => {
            let tasa = tasa_por_plazo(payload.plazo_meses);
            let total_interes = payload.monto * tasa;
            let total_pagar = payload.monto + total_interes;
            let pago_mensual = (total_pagar / payload.plazo_meses as f64 * 100.0).round() / 100.0;

            let capacidad_pago = if cliente.score > 700 { 5000.0 } else { 2000.0 };
            let estado = if pago_mensual <= capacidad_pago { "Aprobado" } else { "Rechazado" };

            let consideraciones = if estado == "Aprobado" {
                format!(
                    "Crédito APROBADO.\nMonto solicitado: ${:.2}\nPlazo: {} meses\nTasa de interés: {:.0}%\nTotal a pagar: ${:.2}\nPago mensual: ${:.2}\n\nEl cliente tiene capacidad de pago suficiente.",
                    payload.monto, payload.plazo_meses, tasa * 100.0, total_pagar, pago_mensual
                )
            } else {
                format!(
                    "Crédito RECHAZADO.\nEl pago mensual de ${:.2} excede la capacidad recomendada (${:.2}) según el Score del cliente ({}).",
                    pago_mensual, capacidad_pago, cliente.score
                )
            };

            let plan_pagos = generar_plan_pagos(payload.monto, payload.plazo_meses, tasa);

            Ok(Json(serde_json::json!(EvaluarRes {
                status: "success".to_string(),
                estado: estado.to_string(),
                pago_mensual,
                tasa_interes: tasa,
                plan_pagos,
                consideraciones,
            })))
        },
        Ok(None) => Ok(Json(serde_json::json!({
            "status": "error",
            "message": "Cliente no encontrado"
        }))),
        Err(e) => {
            eprintln!("🚨 ERROR MONGODB: {:?}", e);
            Ok(Json(serde_json::json!({
                "status": "error",
                "message": "Error en la base de datos"
            })))
        }
    }
}

pub async fn autorizar_credito(
    State(client): State<mongodb::Client>,
    sesion: EmpresaSession,
    Json(payload): Json<AutorizarReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Validación del dominio ANTES de insertar: un plan envenenado se
    // persistiría y congelaría la cartera/contrato desde estado (F2).
    if let Some(msg) = validar_plazo_y_monto(payload.plazo_meses, payload.monto_total) {
        return Err(error_status(StatusCode::BAD_REQUEST, msg));
    }
    // Ola 8 (E2): los montos NO se confían al body — se recomputan desde
    // `monto_total` + `plazo` (el body solo queda por compatibilidad).
    let tasa = tasa_por_plazo(payload.plazo_meses);
    let pago_mensual = pago_mensual_de(payload.monto_total, payload.plazo_meses);
    // Ola 8-fix (E2): red de seguridad — jamás persistir `pago_mensual` no
    // finito (un `Infinity` en Mongo congelaba la cartera y el contrato).
    if !pago_mensual.is_finite() {
        return Err(error_status(
            StatusCode::BAD_REQUEST,
            "El monto genera un pago mensual no finito",
        ));
    }
    let plan_pago = PlanPago {
        id: None, // Mongo lo genera al insertar
        empresa: sesion.correo.clone(),
        cliente_curp: payload.cliente_curp.clone(),
        producto: payload.producto.clone(),
        monto_total: payload.monto_total,
        plazo_meses: payload.plazo_meses,
        pago_mensual,
        tasa_interes: tasa,
        estado: "Activo".to_string(),
        fecha: chrono::Local::now().format("%Y-%m-%d").to_string(),
    };

    let coll_planes = client.database("pymza").collection::<PlanPago>("planes_pago");
    // El frontend necesita el _id del plan para registrar pagos (ola 4).
    let inserted_id = match coll_planes.insert_one(plan_pago, None).await {
        Ok(res) => res.inserted_id,
        Err(e) => {
            eprintln!("🚨 ERROR AL GUARDAR PLAN DE PAGO: {:?}", e);
            return Err(error_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error al guardar el plan de pago",
            ));
        }
    };
    let plan_id = match inserted_id {
        mongodb::bson::Bson::ObjectId(oid) => oid.to_hex(),
        _ => String::new(), // no debería ocurrir: insert sin _id explícito genera ObjectId
    };

    // Las stats se recalculan desde la cartera real (los $inc se desincronizan
    // al liquidar/morosear planes).
    if let Ok((planes, pagos_por_plan)) = cargar_cartera(&client, &sesion.correo).await {
        upsert_dashboard_stats(&client, &sesion.correo, &planes, &pagos_por_plan).await;
    }

    Ok(Json(serde_json::json!({"status": "success", "plan_id": plan_id})))
}

pub async fn obtener_creditos(
    State(client): State<mongodb::Client>,
    sesion: EmpresaSession,
) -> Json<serde_json::Value> {
    let (planes, pagos_por_plan) = match cargar_cartera(&client, &sesion.correo).await {
        Ok(cartera) => cartera,
        Err(e) => {
            eprintln!("🚨 ERROR MONGODB: {:?}", e);
            return Json(serde_json::json!({ "status": "error" }));
        }
    };
    let hoy = Utc::now().date_naive();

    // Join con `clientes` en UNA query: `$in` sobre los CURPs del tenant (no
    // N lookups). Si el cliente ya no está en la red, el CURP hace de nombre.
    let mut curps: Vec<String> = planes.iter().map(|p| p.cliente_curp.clone()).collect();
    curps.sort();
    curps.dedup();
    let mut nombres: HashMap<String, String> = HashMap::new();
    if !curps.is_empty() {
        let coll_clientes = client.database("pymza").collection::<Cliente>("clientes");
        match coll_clientes.find(doc! { "curp": { "$in": &curps } }, None).await {
            Ok(mut cursor) => {
                while let Some(cliente) = cursor.next().await {
                    match cliente {
                        Ok(c) => {
                            nombres.insert(c.curp, c.nombre_completo);
                        }
                        Err(e) => eprintln!("🚨 ERROR MONGODB (clientes): {:?}", e),
                    }
                }
            }
            Err(e) => eprintln!("🚨 ERROR MONGODB (clientes): {:?}", e),
        }
    }

    let creditos: Vec<serde_json::Value> = planes
        .iter()
        .map(|plan| {
            let cobrado = cobrado_de(plan, &pagos_por_plan);
            let mut v = plan_json(plan, cobrado, hoy);
            v["nombre"] = serde_json::json!(
                nombres
                    .get(&plan.cliente_curp)
                    .cloned()
                    .unwrap_or_else(|| plan.cliente_curp.clone())
            );
            v
        })
        .collect();
    Json(serde_json::json!({ "status": "success", "creditos": creditos }))
}

/// Ola 8 (E1): longitud máxima de la `nota` de un abono (O2 del auditor).
const NOTA_MAX: usize = 280;

/// Acota la nota a `NOTA_MAX` chars (no bytes, para no partir un carácter).
fn acotar_nota(nota: Option<&str>) -> Option<String> {
    nota.map(|n| n.chars().take(NOTA_MAX).collect())
}

/// Ola 8 (E1): reserva `monto` en el contador `cobrado` del plan de forma
/// atómica, con guard `cobrado + monto <= pago_mensual * plazo`. Sin
/// transacciones (mongod standalone). `Some(true)` reservó; `Some(false)` el
/// guard no matcheó (saldo agotado o carrera perdida); `None` error de Mongo.
async fn reservar_cobrado(
    client: &mongodb::Client,
    correo: &str,
    plan_id: &ObjectId,
    monto: f64,
) -> Option<bool> {
    let filtro = doc! {
        "_id": plan_id,
        "empresa": correo,
        "$expr": { "$lte": [
            { "$add": [ { "$ifNull": ["$cobrado", 0.0] }, monto ] },
            { "$multiply": ["$pago_mensual", "$plazo_meses"] },
        ] },
    };
    match client
        .database("pymza")
        .collection::<PlanPago>("planes_pago")
        .find_one_and_update(filtro, doc! { "$inc": { "cobrado": monto } }, None)
        .await
    {
        Ok(Some(_)) => Some(true),
        Ok(None) => Some(false),
        Err(e) => {
            eprintln!("🚨 ERROR AL RESERVAR COBRADO: {:?}", e);
            None
        }
    }
}

/// Deshace una reserva (best-effort) cuando el insert del `Pago` falla.
async fn revertir_reserva(client: &mongodb::Client, plan_id: &ObjectId, monto: f64) {
    if let Err(e) = client
        .database("pymza")
        .collection::<PlanPago>("planes_pago")
        .update_one(doc! { "_id": plan_id }, doc! { "$inc": { "cobrado": -monto } }, None)
        .await
    {
        eprintln!("🚨 ERROR AL REVERTIR RESERVA DE COBRADO: {:?}", e);
    }
}

/// Registra el pago de una cuota (ola 4). Validaciones en orden: plan existe y
/// es del tenant (404), cuota en 1..=plazo (400), cuota no pagada (400), monto
/// igual a pago_mensual con tolerancia de 1 centavo (400). Después inserta,
/// recalcula el estado del plan y devuelve el plan actualizado.
pub async fn registrar_pago(
    State(client): State<mongodb::Client>,
    sesion: EmpresaSession,
    Json(payload): Json<RegistrarPagoReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // cargar_cartera solo trae planes del tenant: un plan_id ajeno o inválido
    // no aparece en la lista → 404 único para "no existe" y "no es tuyo".
    let (mut planes, mut pagos_por_plan) = match cargar_cartera(&client, &sesion.correo).await {
        Ok(cartera) => cartera,
        Err(e) => {
            eprintln!("🚨 ERROR MONGODB: {:?}", e);
            return Err(error_status(StatusCode::INTERNAL_SERVER_ERROR, "Error interno"));
        }
    };
    let Some(idx) = planes.iter().position(|p| {
        p.id.as_ref().map_or(false, |id| id.to_hex() == payload.plan_id)
    }) else {
        return Err(error_status(StatusCode::NOT_FOUND, "Plan no encontrado"));
    };
    let plan = planes[idx].clone();
    let plan_hex = plan.id.as_ref().map(|id| id.to_hex()).unwrap_or_default();

    if payload.cuota < 1 || payload.cuota > plan.plazo_meses {
        return Err(error_status(
            StatusCode::BAD_REQUEST,
            &format!("Cuota fuera de rango: debe estar entre 1 y {}", plan.plazo_meses),
        ));
    }
    let cuotas = pagos_por_plan
        .get(&plan_hex)
        .map(|pp| pp.cuotas.clone())
        .unwrap_or_default();
    if cuotas.contains(&payload.cuota) {
        return Err(error_status(StatusCode::BAD_REQUEST, "Cuota ya registrada"));
    }
    if (payload.monto - plan.pago_mensual).abs() > 0.01 {
        return Err(error_status(
            StatusCode::BAD_REQUEST,
            &format!("El monto debe ser igual al pago mensual del plan (${:.2})", plan.pago_mensual),
        ));
    }
    let Some(oid) = plan.id else {
        return Err(error_status(StatusCode::NOT_FOUND, "Plan no encontrado"));
    };

    // Ola 8 (E1): reserva atómica antes del insert (cierra la carrera de pagos
    // concurrentes que rebasaban la deuda).
    match reservar_cobrado(&client, &sesion.correo, &oid, payload.monto).await {
        Some(true) => {}
        Some(false) => {
            return Err(error_status(StatusCode::BAD_REQUEST, "El plan ya está liquidado"))
        }
        None => return Err(error_status(StatusCode::INTERNAL_SERVER_ERROR, "Error interno")),
    }

    let pago = Pago {
        plan_id: plan.id.clone().unwrap_or_default(),
        empresa: sesion.correo.clone(),
        cliente_curp: plan.cliente_curp.clone(),
        cuota: payload.cuota,
        monto: payload.monto,
        fecha: Utc::now().format("%Y-%m-%d").to_string(),
        tipo: "cuota".to_string(),
        nota: None,
    };
    if let Err(e) = client.database("pymza").collection::<Pago>("pagos").insert_one(pago, None).await {
        eprintln!("🚨 ERROR AL GUARDAR PAGO: {:?}", e);
        revertir_reserva(&client, &oid, payload.monto).await;
        return Err(error_status(StatusCode::INTERNAL_SERVER_ERROR, "Error al registrar el pago"));
    }

    // `cobrado` incluye el pago recién insertado; el estado se recalcula por
    // dinero (ola 7) y se persiste solo si cambió.
    let hoy = Utc::now().date_naive();
    let cobrado_nuevo = cobrado_de(&plan, &pagos_por_plan) + payload.monto;
    let estado_nuevo = estado_plan(&plan, cobrado_nuevo, hoy);
    if estado_nuevo != plan.estado {
        planes[idx].estado = estado_nuevo.to_string();
        if let Some(oid) = plan.id {
            if let Err(e) = client
                .database("pymza")
                .collection::<PlanPago>("planes_pago")
                .update_one(doc! { "_id": oid }, doc! { "$set": { "estado": estado_nuevo } }, None)
                .await
            {
                eprintln!("🚨 ERROR AL ACTUALIZAR ESTADO DEL PLAN: {:?}", e);
            }
        }
    }

    let entrada = pagos_por_plan.entry(plan_hex).or_default();
    entrada.cuotas.push(payload.cuota);
    entrada.total += payload.monto;
    upsert_dashboard_stats(&client, &sesion.correo, &planes, &pagos_por_plan).await;

    Ok(Json(serde_json::json!({
        "status": "success",
        "plan": plan_json(&planes[idx], cobrado_nuevo, hoy),
    })))
}

/// Registra un abono parcial (ola 7): baja el saldo SIN marcar la cuota como
/// pagada. Validaciones en orden: plan del tenant (404); `monto` finito > 0
/// (400); plan ya Liquidado (400); `monto > saldo + 0.01` (400). Inserta
/// `Pago{tipo:"abono", cuota:0}` y devuelve el plan con saldo/cobrado.
pub async fn registrar_abono(
    State(client): State<mongodb::Client>,
    sesion: EmpresaSession,
    Json(payload): Json<RegistrarAbonoReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // cargar_cartera solo trae planes del tenant: un plan_id ajeno o inválido
    // no aparece en la lista → 404 único para "no existe" y "no es tuyo".
    let (mut planes, mut pagos_por_plan) = match cargar_cartera(&client, &sesion.correo).await {
        Ok(cartera) => cartera,
        Err(e) => {
            eprintln!("🚨 ERROR MONGODB: {:?}", e);
            return Err(error_status(StatusCode::INTERNAL_SERVER_ERROR, "Error interno"));
        }
    };
    let Some(idx) = planes.iter().position(|p| {
        p.id.as_ref().map_or(false, |id| id.to_hex() == payload.plan_id)
    }) else {
        return Err(error_status(StatusCode::NOT_FOUND, "Plan no encontrado"));
    };
    let plan = planes[idx].clone();
    let plan_hex = plan.id.as_ref().map(|id| id.to_hex()).unwrap_or_default();

    if !payload.monto.is_finite() || payload.monto <= 0.0 {
        return Err(error_status(StatusCode::BAD_REQUEST, "El monto debe ser mayor a 0"));
    }
    let cobrado_previo = cobrado_de(&plan, &pagos_por_plan);
    let saldo_actual = saldo(&plan, cobrado_previo);
    if saldo_actual <= 0.01 {
        return Err(error_status(StatusCode::BAD_REQUEST, "El plan ya está liquidado"));
    }
    if payload.monto > saldo_actual + 0.01 {
        return Err(error_status(StatusCode::BAD_REQUEST, "El abono excede el saldo pendiente"));
    }
    let Some(oid) = plan.id else {
        return Err(error_status(StatusCode::NOT_FOUND, "Plan no encontrado"));
    };

    // Ola 8 (E1): reserva atómica antes del insert. Si otra petición consumió
    // el saldo entre la lectura y aquí, el guard no matchea → 400.
    match reservar_cobrado(&client, &sesion.correo, &oid, payload.monto).await {
        Some(true) => {}
        Some(false) => {
            return Err(error_status(StatusCode::BAD_REQUEST, "El abono excede el saldo pendiente"))
        }
        None => return Err(error_status(StatusCode::INTERNAL_SERVER_ERROR, "Error interno")),
    }

    let pago = Pago {
        plan_id: plan.id.clone().unwrap_or_default(),
        empresa: sesion.correo.clone(),
        cliente_curp: plan.cliente_curp.clone(),
        cuota: 0,
        monto: payload.monto,
        fecha: Utc::now().format("%Y-%m-%d").to_string(),
        tipo: "abono".to_string(),
        nota: acotar_nota(payload.nota.as_deref()),
    };
    if let Err(e) = client.database("pymza").collection::<Pago>("pagos").insert_one(pago, None).await {
        eprintln!("🚨 ERROR AL GUARDAR ABONO: {:?}", e);
        revertir_reserva(&client, &oid, payload.monto).await;
        return Err(error_status(StatusCode::INTERNAL_SERVER_ERROR, "Error al registrar el abono"));
    }

    let hoy = Utc::now().date_naive();
    let cobrado_nuevo = cobrado_previo + payload.monto;
    let estado_nuevo = estado_plan(&plan, cobrado_nuevo, hoy);
    if estado_nuevo != plan.estado {
        planes[idx].estado = estado_nuevo.to_string();
        if let Some(oid) = plan.id {
            if let Err(e) = client
                .database("pymza")
                .collection::<PlanPago>("planes_pago")
                .update_one(doc! { "_id": oid }, doc! { "$set": { "estado": estado_nuevo } }, None)
                .await
            {
                eprintln!("🚨 ERROR AL ACTUALIZAR ESTADO DEL PLAN: {:?}", e);
            }
        }
    }

    let entrada = pagos_por_plan.entry(plan_hex).or_default();
    entrada.total += payload.monto;
    upsert_dashboard_stats(&client, &sesion.correo, &planes, &pagos_por_plan).await;

    Ok(Json(serde_json::json!({
        "status": "success",
        "plan": plan_json(&planes[idx], cobrado_nuevo, hoy),
    })))
}

/// KPIs del dashboard calculados en vivo desde la cartera (ola 8). Conserva
/// los 3 campos viejos y agrega los 5 del contrato; `cobrado_periodo` respeta
/// la ventana (sin ventana = histórico). Dinero en MXN, 2 decimales.
fn stats_dashboard(
    empresa: &str,
    planes: &[PlanPago],
    pagos_por_plan: &HashMap<String, PagosPlan>,
    rango: Option<(NaiveDate, NaiveDate)>,
    hoy: NaiveDate,
) -> serde_json::Value {
    let estado = |p: &PlanPago| estado_plan(p, cobrado_de(p, pagos_por_plan), hoy);
    let no_liquidado = |p: &PlanPago| estado(p) != "Liquidado";

    let capital_prestado: f64 = planes.iter().map(|p| p.monto_total).sum();
    let creditos_activos = planes
        .iter()
        .filter(|p| matches!(estado(p), "Activo" | "Moroso"))
        .count() as i32;
    let proximos_cobros: i32 = planes
        .iter()
        .filter(|p| no_liquidado(p))
        .map(|p| cuotas_por_vencer(p, &pagadas_de(p, pagos_por_plan), hoy, 30))
        .sum();

    let capital_colocado: f64 = planes
        .iter()
        .filter(|p| no_liquidado(p))
        .map(|p| p.monto_total)
        .sum();
    let por_cobrar_neto: f64 = planes
        .iter()
        .filter(|p| no_liquidado(p))
        .map(|p| saldo(p, cobrado_de(p, pagos_por_plan)))
        .sum();
    let cartera_vencida: f64 = planes
        .iter()
        .filter(|p| estado(p) == "Moroso")
        .map(|p| saldo(p, cobrado_de(p, pagos_por_plan)))
        .sum();
    let tasa_morosidad = if capital_colocado > 0.0 {
        cartera_vencida / capital_colocado
    } else {
        0.0
    };

    let cobrado_periodo: f64 = match rango {
        Some((desde, hasta)) => {
            let (ds, hs) = (
                desde.format("%Y-%m-%d").to_string(),
                hasta.format("%Y-%m-%d").to_string(),
            );
            pagos_por_plan
                .values()
                .flat_map(|pp| pp.por_fecha.iter())
                .filter(|(fecha, _)| fecha.as_str() >= ds.as_str() && fecha.as_str() <= hs.as_str())
                .map(|(_, monto)| *monto)
                .sum()
        }
        None => pagos_por_plan.values().map(|pp| pp.total).sum(),
    };

    serde_json::json!({
        "empresa": empresa,
        "creditos_activos": creditos_activos,
        "capital_prestado": redondear2(capital_prestado),
        "proximos_cobros": proximos_cobros,
        "capital_colocado": redondear2(capital_colocado),
        "cobrado_periodo": redondear2(cobrado_periodo),
        "por_cobrar_neto": redondear2(por_cobrar_neto),
        "cartera_vencida": redondear2(cartera_vencida),
        "tasa_morosidad": tasa_morosidad,
    })
}

/// GET /api/dashboard (ola 8): KPIs en vivo desde la cartera del tenant.
/// Acepta `?desde&hasta` para `cobrado_periodo`; sin ventana usa el histórico.
pub async fn obtener_dashboard(
    State(client): State<mongodb::Client>,
    sesion: EmpresaSession,
    Query(params): Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let (planes, pagos_por_plan) = match cargar_cartera(&client, &sesion.correo).await {
        Ok(cartera) => cartera,
        Err(e) => {
            eprintln!("🚨 ERROR MONGODB: {:?}", e);
            return Json(serde_json::json!({ "status": "error" }));
        }
    };
    let stats = stats_dashboard(
        &sesion.correo,
        &planes,
        &pagos_por_plan,
        rango_fechas(&params),
        Utc::now().date_naive(),
    );
    Json(serde_json::json!({ "status": "success", "stats": stats }))
}

/// GET /api/creditos/:plan_id/contrato (ola 6): genera y devuelve el PDF del
/// contrato del plan. Validaciones: plan_id hex → 400; existe y es del tenant
/// del token → si no, 404 (el plan ajeno ni aparece con el filtro de empresa,
/// mismo 404 único que registrar_pago). El PDF se regenera bajo demanda, nunca
/// se almacena.
pub async fn descargar_contrato(
    State(client): State<mongodb::Client>,
    sesion: EmpresaSession,
    Path(plan_id): Path<String>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let error_db = |e: mongodb::error::Error| {
        eprintln!("🚨 ERROR MONGODB (contrato): {e:?}");
        error_status(StatusCode::INTERNAL_SERVER_ERROR, "Error interno")
    };

    let Ok(oid) = ObjectId::parse_str(&plan_id) else {
        return Err(error_status(
            StatusCode::BAD_REQUEST,
            "plan_id inválido: se espera el hex del ObjectId",
        ));
    };
    let db = client.database("pymza");
    let plan = db
        .collection::<PlanPago>("planes_pago")
        .find_one(doc! { "_id": oid, "empresa": &sesion.correo }, None)
        .await
        .map_err(error_db)?
        .ok_or_else(|| error_status(StatusCode::NOT_FOUND, "Plan no encontrado"))?;

    // Nombre de la empresa (para el PDF); si el registro no está, el correo
    // hace de nombre — el contrato sigue legible y no se rompe por cosmética.
    let empresa = db
        .collection::<Empresa>("empresas")
        .find_one(doc! { "correo": &sesion.correo }, None)
        .await
        .map_err(error_db)?;
    let nombre_empresa = empresa
        .map(|e| e.nombre_empresa)
        .unwrap_or_else(|| sesion.correo.clone());

    // Nombre del cliente; si ya no existe en la red, el CURP hace de nombre
    // (patrón de resumen_cartera).
    let cliente = db
        .collection::<Cliente>("clientes")
        .find_one(doc! { "curp": &plan.cliente_curp }, None)
        .await
        .map_err(error_db)?;
    let nombre_cliente = cliente
        .map(|c| c.nombre_completo)
        .unwrap_or_else(|| plan.cliente_curp.clone());

    // Ola 7: pagos/abonos del plan (filtrados por tenant) para la sección del
    // contrato; el cobrado/saldo de emisión sale de la misma suma.
    let mut cursor = db
        .collection::<Pago>("pagos")
        .find(doc! { "plan_id": oid, "empresa": &sesion.correo }, None)
        .await
        .map_err(error_db)?;
    let mut pagos = Vec::new();
    while let Some(pago) = cursor.next().await {
        pagos.push(pago.map_err(error_db)?);
    }
    pagos.sort_by(|a, b| a.fecha.cmp(&b.fecha));
    let cobrado: f64 = pagos.iter().map(|p| p.monto).sum();
    let saldo_actual = saldo(&plan, cobrado);

    let pdf = pdf_contrato(
        &nombre_empresa,
        &sesion.correo,
        &nombre_cliente,
        &plan.cliente_curp,
        &plan,
        &pagos,
        redondear2(cobrado),
        saldo_actual,
        &Utc::now().format("%Y-%m-%d").to_string(),
    );

    let mut res = Response::new(Body::from(pdf));
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/pdf"),
    );
    // El CURP es ascii; si el curp guardado trae basura, filename genérico.
    let disposition = HeaderValue::from_str(&format!(
        "attachment; filename=\"contrato-{}.pdf\"",
        plan.cliente_curp
    ))
    .unwrap_or_else(|_| HeaderValue::from_static("attachment; filename=\"contrato.pdf\""));
    res.headers_mut().insert(header::CONTENT_DISPOSITION, disposition);
    // axum::Response es http::Response<UnsyncBoxBody>: IntoResponse hace el boxing.
    Ok(res.into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::bson::oid::ObjectId;

    #[test]
    fn tasa_por_plazo_devuelve_la_tasa_esperada() {
        assert_eq!(tasa_por_plazo(1), 0.07);
        assert_eq!(tasa_por_plazo(3), 0.09);
        assert_eq!(tasa_por_plazo(6), 0.12);
        assert_eq!(tasa_por_plazo(9), 0.15);
        assert_eq!(tasa_por_plazo(12), 0.18);
        // fuera del contrato: inalcanzable vía endpoints (validar filtra antes)
        assert_eq!(tasa_por_plazo(4), 0.05);
    }

    #[test]
    fn generar_plan_pagos_mantiene_invariantes() {
        let monto = 10000.0;
        let plazo = 3;
        let plan = generar_plan_pagos(monto, plazo, tasa_por_plazo(plazo));

        assert_eq!(plan.len(), plazo as usize);
        assert_eq!(plan.first().unwrap().mes, 1);
        assert_eq!(plan.last().unwrap().mes, plazo);

        let suma_capital: f64 = plan.iter().map(|p| p.capital).sum();
        // ponytail: cada capital mensual se redondea al centavo, así que la suma
        // puede desviarse del monto hasta 0.005 por mes; el techo escala con el plazo.
        assert!(
            (suma_capital - monto).abs() <= 0.005 * plazo as f64 + 0.001,
            "suma capital {} no coincide con monto {}", suma_capital, monto
        );

        assert_eq!(plan.last().unwrap().saldo_restante, 0.0);

        for p in &plan {
            assert!(p.saldo_restante >= 0.0, "saldo negativo en mes {}", p.mes);
            for campo in [p.pago, p.interes, p.capital, p.saldo_restante] {
                assert!(
                    ((campo * 100.0) - (campo * 100.0).round()).abs() < 1e-6,
                    "campo sin redondear a 2 decimales en mes {}: {campo}",
                    p.mes
                );
            }
        }
    }

    #[test]
    fn generar_plan_pagos_suma_capital_exacta_sin_redondeo() {
        let plan = generar_plan_pagos(12000.0, 3, 0.03);
        let suma_capital: f64 = plan.iter().map(|p| p.capital).sum();
        assert_eq!(suma_capital, 12000.0);
    }

    fn plan_ejemplo() -> PlanPago {
        PlanPago {
            id: None,
            empresa: "demo@pymza.mx".into(),
            cliente_curp: "GARM980412HDFNRL05".into(),
            producto: "Crédito comercial".into(),
            monto_total: 10600.0,
            plazo_meses: 6,
            pago_mensual: 1766.67,
            tasa_interes: 0.06,
            estado: "Activo".into(),
            fecha: "2026-01-01".into(),
        }
    }

    #[test]
    fn fecha_vencimiento_suma_meses_con_clamp_de_fin_de_mes() {
        let f = fecha_vencimiento("2026-01-31", 1).unwrap();
        assert_eq!(f, NaiveDate::from_ymd_opt(2026, 2, 28).unwrap(), "31-ene + 1m = 28-feb");
        assert_eq!(
            fecha_vencimiento("2026-01-31", 3).unwrap(),
            NaiveDate::from_ymd_opt(2026, 4, 30).unwrap()
        );
        assert_eq!(
            fecha_vencimiento("2026-01-31", 12).unwrap(),
            NaiveDate::from_ymd_opt(2027, 1, 31).unwrap()
        );
        assert_eq!(
            fecha_vencimiento("2026-01-01", 0).unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        );
        assert!(fecha_vencimiento("no-fecha", 1).is_none());
        assert!(fecha_vencimiento("2026-02-30", 1).is_none(), "fecha imposible");
    }

    #[test]
    fn estado_plan_pagada_a_tiempo_permanece_activo() {
        let mut plan = plan_ejemplo();
        plan.fecha = "2026-06-01".into();
        let hoy = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        // cuota 1 cubierta por dinero; cuota 2 vence 2026-08-01 (futuro)
        assert_eq!(estado_plan(&plan, plan.pago_mensual, hoy), "Activo");
        // el día del vencimiento aún NO es moroso (vencimiento < hoy estricto)
        assert_eq!(estado_plan(&plan, 0.0, hoy), "Activo");
    }

    #[test]
    fn estado_plan_cuota_atrasada_es_moroso() {
        let plan = plan_ejemplo(); // fecha 2026-01-01, plazo 6
        let hoy = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        // cuota 1 venció 2026-02-01 y cuota 2 el 2026-03-01
        assert_eq!(estado_plan(&plan, 0.0, hoy), "Moroso");
        assert_eq!(cuotas_vencidas(&plan, 0.0, hoy), 2);
        // un pago cubre la cuota más antigua: sigue moroso por la segunda
        assert_eq!(estado_plan(&plan, plan.pago_mensual, hoy), "Moroso");
        assert_eq!(cuotas_vencidas(&plan, plan.pago_mensual, hoy), 1);
        // dos pagos cubren ambas vencidas → Activo (queda saldo por delante)
        assert_eq!(estado_plan(&plan, plan.pago_mensual * 2.0, hoy), "Activo");
        assert_eq!(cuotas_vencidas(&plan, plan.pago_mensual * 2.0, hoy), 0);
    }

    #[test]
    fn estado_plan_todo_pagado_es_liquidado() {
        let plan = plan_ejemplo();
        let hoy = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let total = deuda(&plan);
        assert_eq!(estado_plan(&plan, total, hoy), "Liquidado");
        assert_eq!(cuotas_vencidas(&plan, total, hoy), 0);
        assert_eq!(saldo(&plan, total), 0.0);
    }

    #[test]
    fn saldo_es_deuda_menos_cobrado_y_nunca_negativo() {
        let plan = plan_ejemplo();
        let total = deuda(&plan);
        assert_eq!(saldo(&plan, 0.0), redondear2(total));
        assert_eq!(saldo(&plan, 500.0), redondear2(total - 500.0));
        // cobrar de más no deja saldo negativo
        assert_eq!(saldo(&plan, total + 999.0), 0.0);
    }

    #[test]
    fn cuotas_cubiertas_por_dinero_incluye_abonos() {
        let plan = plan_ejemplo(); // pago_mensual 1766.67, plazo 6
        assert_eq!(cuotas_cubiertas(&plan, 0.0), 0);
        // un abono parcial de media cuota no cubre ninguna
        assert_eq!(cuotas_cubiertas(&plan, plan.pago_mensual / 2.0), 0);
        // pago exacto + abono: dos cuotas cubiertas
        assert_eq!(cuotas_cubiertas(&plan, plan.pago_mensual * 2.0 + 100.0), 2);
        // nunca más que el plazo
        assert_eq!(cuotas_cubiertas(&plan, deuda(&plan) * 2.0), plan.plazo_meses);
    }

    #[test]
    fn estado_plan_abonos_parciales_no_marcan_cuota() {
        // Un plan se liquida SOLO con abonos: no hay cuota marcada y aun así
        // debe quedar Liquidado por dinero (ola 7).
        let plan = plan_ejemplo();
        let hoy = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let total = deuda(&plan);
        // un abono menor a una cuota no cubre las vencidas → sigue Moroso
        assert_eq!(estado_plan(&plan, plan.pago_mensual * 0.5, hoy), "Moroso");
        assert_eq!(estado_plan(&plan, total, hoy), "Liquidado");
    }

    #[test]
    fn cuotas_por_vencer_ventanas_de_30_60_90() {
        let plan = plan_ejemplo(); // fecha 2026-01-01, plazo 6
        let hoy = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        // cuota 1 vence a 31 días, cuota 2 a 59, cuota 3 a 90 (hoy incluido);
        // las ventanas son acumulativas: ≤60 incluye las dos primeras.
        assert_eq!(cuotas_por_vencer(&plan, &[], hoy, 30), 0);
        assert_eq!(cuotas_por_vencer(&plan, &[], hoy, 60), 2);
        assert_eq!(cuotas_por_vencer(&plan, &[], hoy, 90), 3);
        // las ya pagadas no cuentan
        assert_eq!(cuotas_por_vencer(&plan, &[1, 2], hoy, 90), 1);
    }

    #[test]
    fn plan_json_expone_id_avance_y_saldo() {
        let mut plan = plan_ejemplo();
        plan.id = ObjectId::parse_str("507f1f77bcf86cd799439011").ok();
        let hoy = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let cobrado = plan.pago_mensual * 2.0;
        let v = plan_json(&plan, cobrado, hoy);
        assert_eq!(v["_id"], "507f1f77bcf86cd799439011");
        assert_eq!(v["cuotas_pagadas"], 2);
        assert_eq!(v["cuotas_vencidas"], 0);
        assert_eq!(v["estado"], "Activo");
        assert_eq!(v["cobrado"], redondear2(cobrado));
        assert_eq!(v["saldo"], saldo(&plan, cobrado));
    }

    #[test]
    fn plan_json_recalcula_el_estado_persistido_rancio() {
        let mut plan = plan_ejemplo();
        plan.estado = "Liquidado".into(); // valor rancio persistido
        let hoy = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let v = plan_json(&plan, 0.0, hoy);
        assert_eq!(v["estado"], "Moroso", "el estado se recalcula en lectura");
        assert_eq!(v["saldo"], redondear2(deuda(&plan)));
    }

    #[test]
    fn buckets_aging_cubren_los_limites() {
        assert_eq!(bucket_aging(1), "0-30");
        assert_eq!(bucket_aging(30), "0-30");
        assert_eq!(bucket_aging(31), "31-60");
        assert_eq!(bucket_aging(60), "31-60");
        assert_eq!(bucket_aging(61), "61-90");
        assert_eq!(bucket_aging(90), "61-90");
        assert_eq!(bucket_aging(91), "90+");
        assert_eq!(bucket_aging(365), "90+");
    }

    #[test]
    fn buckets_monto_cubren_los_limites() {
        assert_eq!(bucket_monto(0.0), "0-1k");
        assert_eq!(bucket_monto(999.99), "0-1k");
        assert_eq!(bucket_monto(1000.0), "1k-5k");
        assert_eq!(bucket_monto(4999.99), "1k-5k");
        assert_eq!(bucket_monto(5000.0), "5k+");
    }

    #[test]
    fn top_deudores_orden_desc_y_saldo() {
        let mut plan_a = plan_ejemplo();
        plan_a.id = ObjectId::parse_str("507f1f77bcf86cd799439011").ok();
        plan_a.cliente_curp = "AAAA1".into();
        let mut plan_b = plan_ejemplo();
        plan_b.id = ObjectId::parse_str("507f1f77bcf86cd799439012").ok();
        plan_b.cliente_curp = "BBBB2".into();

        let mut pagos = HashMap::new();
        pagos.insert(
            plan_a.id.as_ref().unwrap().to_hex(),
            PagosPlan { cuotas: vec![1], total: 1766.67, por_fecha: HashMap::new() },
        );
        // plan_b sin pagos → saldo completo: es el mayor deudor

        let deudores = top_deudores(&[plan_a, plan_b], &pagos);
        assert_eq!(deudores.len(), 2);
        assert_eq!(deudores[0].0, "BBBB2");
        assert_eq!(deudores[0].1, redondear2(1766.67 * 6.0));
        assert_eq!(deudores[1].1, redondear2(1766.67 * 6.0 - 1766.67));
    }

    #[test]
    fn top_deudores_maximo_10() {
        let planes: Vec<PlanPago> = (0..12u8)
            .map(|i| {
                let mut p = plan_ejemplo();
                p.id = Some(ObjectId::from_bytes([i; 12]));
                p.cliente_curp = format!("CURP{i:02}");
                p
            })
            .collect();
        assert_eq!(top_deudores(&planes, &HashMap::new()).len(), 10);
    }

    #[test]
    fn resumen_cartera_shape_exacta_con_cartera_vacia() {
        let r = resumen_cartera(&[], &HashMap::new(), &HashMap::new(), NaiveDate::from_ymd_opt(2026, 9, 4).unwrap(), None);
        assert_eq!(r["cobrado_vs_por_cobrar"].as_array().unwrap().len(), 6);
        assert_eq!(r["cobrado_vs_por_cobrar"][5]["mes"], "2026-09", "último = mes actual");
        assert_eq!(r["cobrado_vs_por_cobrar"][0]["mes"], "2026-04", "primero = mes actual − 5");
        assert_eq!(r["cobrado_vs_por_cobrar"][0]["cobrado"], 0.0);
        assert_eq!(r["cobrado_vs_por_cobrar"][0]["por_cobrar"], 0.0);
        assert_eq!(r["tasa_morosidad"], 0.0);
        assert_eq!(r["flujo_proyectado"].as_array().unwrap().len(), 3);
        assert_eq!(r["flujo_proyectado"][0]["horizonte"], 30);
        assert_eq!(r["flujo_proyectado"][2]["horizonte"], 90);
        assert_eq!(r["aging"].as_array().unwrap().len(), 4);
        assert_eq!(r["aging"][0]["bucket"], "0-30");
        assert_eq!(r["aging"][3]["bucket"], "90+");
        assert!(r["top_deudores"].as_array().unwrap().is_empty());
        assert_eq!(r["distribucion_montos"].as_array().unwrap().len(), 3);
        assert_eq!(r["distribucion_montos"][0]["bucket"], "0-1k");
        assert_eq!(r["distribucion_montos"][2]["bucket"], "5k+");
        // keys exactas de cada elemento (shape del contrato)
        assert!(r["aging"][0].as_object().unwrap().keys().all(|k| ["bucket", "monto"].contains(&k.as_str())));
        assert!(r["distribucion_montos"][0].as_object().unwrap().keys().all(|k| ["bucket", "n"].contains(&k.as_str())));
        assert!(r["cobrado_vs_por_cobrar"][0]
            .as_object()
            .unwrap()
            .keys()
            .all(|k| ["mes", "cobrado", "por_cobrar"].contains(&k.as_str())));
    }

    #[test]
    fn resumen_cartera_calcula_con_datos() {
        let mut plan = plan_ejemplo(); // 2026-01-01, plazo 6, pago 1766.67
        plan.id = ObjectId::parse_str("507f1f77bcf86cd799439011").ok();
        plan.estado = "Moroso".into(); // cuota 2 vencida al 2026-03-15

        let mut pagos = HashMap::new();
        let mut por_fecha = HashMap::new();
        por_fecha.insert("2026-02-10".to_string(), 1766.67);
        pagos.insert(
            plan.id.as_ref().unwrap().to_hex(),
            PagosPlan { cuotas: vec![1], total: 1766.67, por_fecha },
        );

        let mut nombres = HashMap::new();
        nombres.insert(plan.cliente_curp.clone(), "María García".to_string());

        let hoy = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let r = resumen_cartera(&[plan], &pagos, &nombres, hoy, None);

        // cobrado feb (cuota 1 pagada); por_cobrar marzo (cuota 2 vence 03-01)
        let meses = r["cobrado_vs_por_cobrar"].as_array().unwrap();
        let feb = meses.iter().find(|m| m["mes"] == "2026-02").unwrap();
        assert_eq!(feb["cobrado"], 1766.67);
        assert_eq!(feb["por_cobrar"], 0.0);
        let mar = meses.iter().find(|m| m["mes"] == "2026-03").unwrap();
        assert_eq!(mar["cobrado"], 0.0);
        assert_eq!(mar["por_cobrar"], 1766.67);
        // cuota 2 vencida hace 14 días → aging 0-30
        assert_eq!(r["aging"][0]["monto"], 1766.67);
        assert_eq!(r["aging"][1]["monto"], 0.0);
        // flujo (acumulativo): cuota 3 vence 04-01 (17d), 4 → 05-01 (47d), 5 → 06-01 (78d), 6 → 07-01 (108d)
        let flujo = r["flujo_proyectado"].as_array().unwrap();
        assert_eq!(flujo[0]["monto"], 1766.67);
        assert_eq!(flujo[1]["monto"], redondear2(1766.67 * 2.0));
        assert_eq!(flujo[2]["monto"], redondear2(1766.67 * 3.0));
        // morosidad por dinero: saldo vencido / capital colocado (monto_total), ola 8
        let tasa = r["tasa_morosidad"].as_f64().unwrap();
        let esperado = redondear2(1766.67 * 6.0 - 1766.67) / 10600.0;
        assert!((tasa - esperado).abs() < 1e-9, "tasa {tasa} != {esperado}");
        // top deudor: saldo = pago_mensual × plazo − pagado
        assert_eq!(r["top_deudores"][0]["saldo"], redondear2(1766.67 * 6.0 - 1766.67));
        assert_eq!(r["top_deudores"][0]["nombre"], "María García");
        // distribución: monto_total 10600 → 5k+
        assert_eq!(r["distribucion_montos"][2]["n"], 1);
    }

    #[test]
    fn resumen_tasa_morosidad_es_dinero_sobre_no_liquidados() {
        // plan_a moroso (sin pagos, cuotas vencidas), plan_b activo (fecha
        // futura) y un tercero liquidado por pagos (queda fuera del denominador).
        let mut plan_a = plan_ejemplo(); // fecha 2026-01-01
        plan_a.id = ObjectId::parse_str("507f1f77bcf86cd799439011").ok();
        let mut plan_b = plan_ejemplo();
        plan_b.id = ObjectId::parse_str("507f1f77bcf86cd799439012").ok();
        plan_b.fecha = "2026-03-01".into(); // sin cuotas vencidas al 03-15
        let mut liquidado = plan_ejemplo();
        liquidado.id = ObjectId::parse_str("507f1f77bcf86cd799439013").ok();

        let mut pagos = HashMap::new();
        pagos.insert(
            liquidado.id.as_ref().unwrap().to_hex(),
            PagosPlan {
                cuotas: vec![1, 2, 3, 4, 5, 6],
                total: liquidado.pago_mensual * 6.0,
                por_fecha: HashMap::new(),
            },
        );

        let hoy = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let esperado = redondear2(1766.67 * 6.0) / (2.0 * 10600.0);
        let r = resumen_cartera(&[plan_a, plan_b, liquidado], &pagos, &HashMap::new(), hoy, None);
        // vencida = saldo de plan_a; colocado = monto_total de plan_a + plan_b
        let tasa = r["tasa_morosidad"].as_f64().unwrap();
        assert!((tasa - esperado).abs() < 1e-9, "tasa money-based {tasa} != {esperado}");
    }

    // --- Ola 8: E1 (reserva/nota/ventana) y E2 (montos recalculados) ---

    #[test]
    fn pago_mensual_recalculado_ignora_el_body() {
        // E2: 100000 a 6 meses (tasa 12%) → 18666.67/mes; deuda 112000.02,
        // nunca la del body (pago_mensual 0.01 → deuda 0.06).
        let pm = pago_mensual_de(100000.0, 6);
        assert_eq!(pm, redondear2(100000.0 * 1.12 / 6.0));
        assert_eq!(pm, 18666.67);
        assert!(pm * 6.0 > 100000.0, "la deuda incluye el interés");
    }

    #[test]
    fn acotar_nota_corta_a_280_chars() {
        assert_eq!(acotar_nota(None), None);
        let larga = "x".repeat(300);
        assert_eq!(acotar_nota(Some(&larga)).unwrap().chars().count(), 280);
        assert_eq!(acotar_nota(Some("ok")).as_deref(), Some("ok"));
    }

    #[test]
    fn rango_fechas_valida_ordena_o_none() {
        let m: HashMap<String, String> = HashMap::from([
            ("desde".to_string(), "2026-09-01".to_string()),
            ("hasta".to_string(), "2026-09-30".to_string()),
        ]);
        assert_eq!(
            rango_fechas(&m),
            Some((
                NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()
            ))
        );
        // invertidas se ordenan
        let m2: HashMap<String, String> = HashMap::from([
            ("desde".to_string(), "2026-09-30".to_string()),
            ("hasta".to_string(), "2026-09-01".to_string()),
        ]);
        assert_eq!(
            rango_fechas(&m2),
            Some((
                NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()
            ))
        );
        // falta una o es inválida → None
        let m3: HashMap<String, String> =
            HashMap::from([("desde".to_string(), "2026-09-01".to_string())]);
        assert_eq!(rango_fechas(&m3), None);
        let m4: HashMap<String, String> = HashMap::from([
            ("desde".to_string(), "no-es-fecha".to_string()),
            ("hasta".to_string(), "2026-09-01".to_string()),
        ]);
        assert_eq!(rango_fechas(&m4), None);
    }

    #[test]
    fn stats_dashboard_kpis_en_vivo_y_ventana() {
        let mut plan = plan_ejemplo();
        plan.id = ObjectId::parse_str("507f1f77bcf86cd799439011").ok();
        let mut por_fecha = HashMap::new();
        por_fecha.insert("2026-02-10".to_string(), 1766.67);
        let mut pagos = HashMap::new();
        pagos.insert(
            plan.id.as_ref().unwrap().to_hex(),
            PagosPlan { cuotas: vec![1], total: 1766.67, por_fecha },
        );

        let hoy = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let feb = (
            NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap(),
        );
        let s = stats_dashboard("demo@pymza.mx", &[plan.clone()], &pagos, Some(feb), hoy);
        assert_eq!(s["empresa"], "demo@pymza.mx");
        assert_eq!(s["creditos_activos"], 1);
        assert_eq!(s["capital_prestado"], 10600.0);
        assert_eq!(s["capital_colocado"], 10600.0);
        assert_eq!(s["cobrado_periodo"], 1766.67);
        assert_eq!(s["por_cobrar_neto"], redondear2(10600.02 - 1766.67));
        assert_eq!(s["cartera_vencida"], redondear2(10600.02 - 1766.67));

        // ventana fuera del pago → cobrado_periodo 0
        let mar = (
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
        );
        let s2 = stats_dashboard("x", &[plan], &pagos, Some(mar), hoy);
        assert_eq!(s2["cobrado_periodo"], 0.0);
    }

    #[test]
    fn resumen_serie_respeta_la_ventana() {
        let mut plan = plan_ejemplo();
        plan.id = ObjectId::parse_str("507f1f77bcf86cd799439011").ok();
        let mut por_fecha = HashMap::new();
        por_fecha.insert("2026-02-10".to_string(), 100.0);
        por_fecha.insert("2026-03-20".to_string(), 200.0);
        let mut pagos = HashMap::new();
        pagos.insert(
            plan.id.as_ref().unwrap().to_hex(),
            PagosPlan { cuotas: vec![], total: 300.0, por_fecha },
        );
        let hoy = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();

        // ventana marzo: un solo bucket y solo el pago de marzo
        let rango = (
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
        );
        let r = resumen_cartera(&[plan.clone()], &pagos, &HashMap::new(), hoy, Some(rango));
        let meses = r["cobrado_vs_por_cobrar"].as_array().unwrap();
        assert_eq!(meses.len(), 1);
        assert_eq!(meses[0]["mes"], "2026-03");
        assert_eq!(meses[0]["cobrado"], 200.0);

        // sin ventana: 6 meses y la suma de ambos pagos
        let r2 = resumen_cartera(&[plan], &pagos, &HashMap::new(), hoy, None);
        let total: f64 = r2["cobrado_vs_por_cobrar"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["cobrado"].as_f64().unwrap())
            .sum();
        assert_eq!(total, 300.0);
    }

    // --- F1/F2 (auditoría ola 6) + plazos ola 7: 1/3/6/9/12 y monto > 0 finito ---

    #[test]
    fn validar_plazo_y_monto_aplica_el_contrato() {
        for plazo in [1, 3, 6, 9, 12] {
            assert_eq!(validar_plazo_y_monto(plazo, 100.0), None, "plazo válido {plazo}");
        }
        let msg = Some("El plazo debe ser 1, 3, 6, 9 o 12 meses");
        assert_eq!(validar_plazo_y_monto(1000000, 100.0), msg);
        assert_eq!(validar_plazo_y_monto(0, 100.0), msg);
        assert_eq!(validar_plazo_y_monto(4, 100.0), msg, "plazo fuera del catálogo");
        assert_eq!(validar_plazo_y_monto(6, -1.0), Some("El monto debe ser mayor a 0"));
        assert_eq!(validar_plazo_y_monto(6, 0.0), Some("El monto debe ser mayor a 0"));
        // E2 ola 8-fix: no finito sigue siendo "mayor a 0" (mismo mensaje);
        // finito pero por encima del tope → 400 propio; el tope exacto pasa.
        assert_eq!(validar_plazo_y_monto(6, f64::INFINITY), Some("El monto debe ser mayor a 0"));
        assert_eq!(validar_plazo_y_monto(6, 1e308), Some("El monto excede el máximo permitido"));
        assert_eq!(
            validar_plazo_y_monto(6, MONTO_MAX_MXN + 1.0),
            Some("El monto excede el máximo permitido")
        );
        assert_eq!(validar_plazo_y_monto(6, MONTO_MAX_MXN), None);
    }

    #[test]
    fn pago_mensual_en_el_tope_sigue_siendo_finito() {
        // E2 ola 8-fix: con el tope, la fórmula nunca desborda a Infinity.
        let pm = pago_mensual_de(MONTO_MAX_MXN, 1);
        assert!(pm.is_finite(), "pago mensual no finito en el tope: {pm}");
        assert!(pm > MONTO_MAX_MXN, "incluye el interés del 7% a 1 mes");
    }

    // Client sin servidor: la validación corre ANTES de cualquier acceso a
    // Mongo, así que el client jamás se usa y los tests corren sin DB —
    // NADA se lee ni se inserta en planes_pago (el insert está después).
    async fn client_test() -> mongodb::Client {
        let opts = mongodb::options::ClientOptions::parse("mongodb://127.0.0.1:27017")
            .await
            .unwrap();
        mongodb::Client::with_options(opts).unwrap()
    }

    fn sesion_test() -> EmpresaSession {
        EmpresaSession { correo: "test@pymza.mx".into(), nombre: "Test".into() }
    }

    #[tokio::test]
    async fn evaluar_rechaza_plazo_gigante_con_400() {
        // F1: plazo gigante → collect() de ~86 GB → OOM; ahora muere con 400
        // antes de generar el plan.
        let req = EvaluarReq { curp: "GARM980412HDFNRL05".into(), monto: 10000.0, plazo_meses: 1000000 };
        let res = evaluar_credito(State(client_test().await), sesion_test(), Json(req)).await;
        let (status, body) = res.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.0["status"], "error");
        assert_eq!(body.0["message"], "El plazo debe ser 1, 3, 6, 9 o 12 meses");
    }

    #[tokio::test]
    async fn evaluar_rechaza_monto_negativo_con_400() {
        let req = EvaluarReq { curp: "GARM980412HDFNRL05".into(), monto: -1.0, plazo_meses: 6 };
        let res = evaluar_credito(State(client_test().await), sesion_test(), Json(req)).await;
        let (status, body) = res.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.0["message"], "El monto debe ser mayor a 0");
    }

    #[tokio::test]
    async fn autorizar_rechaza_plazo_gigante_con_400() {
        // F2: el plan envenenado jamás debe llegar a planes_pago (validación
        // antes del insert → nada se inserta).
        let req = AutorizarReq {
            cliente_curp: "GARM980412HDFNRL05".into(),
            producto: "Crédito comercial".into(),
            monto_total: 10600.0,
            plazo_meses: 1000000,
            pago_mensual: 1766.67,
            tasa_interes: 0.06,
        };
        let res = autorizar_credito(State(client_test().await), sesion_test(), Json(req)).await;
        let (status, body) = res.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.0["message"], "El plazo debe ser 1, 3, 6, 9 o 12 meses");
    }

    #[tokio::test]
    async fn autorizar_rechaza_monto_negativo_con_400() {
        let req = AutorizarReq {
            cliente_curp: "GARM980412HDFNRL05".into(),
            producto: "Crédito comercial".into(),
            monto_total: -1.0,
            plazo_meses: 6,
            pago_mensual: 0.0,
            tasa_interes: 0.06,
        };
        let res = autorizar_credito(State(client_test().await), sesion_test(), Json(req)).await;
        let (status, body) = res.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.0["message"], "El monto debe ser mayor a 0");
    }
}
