use std::collections::HashMap;

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use chrono::{Months, NaiveDate, Utc};
use futures::StreamExt;
use mongodb::bson::doc;

use crate::auth::EmpresaSession;
use crate::models::cliente::Cliente;
use crate::models::credito::{
    AutorizarReq, DashboardStats, EvaluarReq, EvaluarRes, Pago, PagoInfo, PlanPago, RegistrarPagoReq,
};

fn tasa_por_plazo(meses: i32) -> f64 {
    match meses {
        3 => 0.03,
        6 => 0.06,
        9 => 0.10,
        12 => 0.15,
        _ => 0.05,
    }
}

fn generar_plan_pagos(monto: f64, plazo_meses: i32, tasa: f64) -> Vec<PagoInfo> {
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

/// Estado tras cada pago: Liquidado (todas las cuotas pagadas), Moroso
/// (≥1 cuota vencida sin pagar: vencimiento < hoy), Activo (resto).
pub(crate) fn estado_plan(plan: &PlanPago, cuotas_pagadas: &[i32], hoy: NaiveDate) -> &'static str {
    if cuotas_pagadas.len() as i32 >= plan.plazo_meses {
        return "Liquidado";
    }
    let vencida_sin_pago = (1..=plan.plazo_meses).any(|n| {
        !cuotas_pagadas.contains(&n)
            && fecha_vencimiento(&plan.fecha, n).map_or(false, |v| v < hoy)
    });
    if vencida_sin_pago { "Moroso" } else { "Activo" }
}

/// Cuotas vencidas sin pagar (vencimiento < hoy), ≥ 0.
pub(crate) fn cuotas_vencidas(plan: &PlanPago, cuotas_pagadas: &[i32], hoy: NaiveDate) -> i32 {
    (1..=plan.plazo_meses)
        .filter(|n| {
            !cuotas_pagadas.contains(n)
                && fecha_vencimiento(&plan.fecha, *n).map_or(false, |v| v < hoy)
        })
        .count() as i32
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

/// Plan serializado para la API: `_id` hex + avance calculado (cuotas_pagadas
/// y cuotas_vencidas), nunca persistido.
fn plan_json(plan: &PlanPago, cuotas_pagadas: i32, cuotas_vencidas: i32) -> serde_json::Value {
    let mut v = serde_json::to_value(plan).unwrap_or_else(|_| serde_json::json!({}));
    // ObjectId serializa como {"$oid": hex} con serde_json; el contrato pide
    // el hex string plano (el frontend lo manda tal cual al registrar pagos).
    if let Some(id) = &plan.id {
        v["_id"] = serde_json::json!(id.to_hex());
    }
    v["cuotas_pagadas"] = serde_json::json!(cuotas_pagadas);
    v["cuotas_vencidas"] = serde_json::json!(cuotas_vencidas);
    v
}

/// Pagos de un plan (agrupados por hex del ObjectId): cuotas pagadas y total cobrado.
#[derive(Default, Clone)]
struct PagosPlan {
    cuotas: Vec<i32>,
    total: f64,
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
    let creditos_activos = planes
        .iter()
        .filter(|p| p.estado == "Activo" || p.estado == "Moroso")
        .count() as i32;
    let capital_prestado: f64 = planes.iter().map(|p| p.monto_total).sum();
    let proximos_cobros: i32 = planes
        .iter()
        .filter(|p| p.estado != "Liquidado")
        .map(|p| {
            let pagadas = p
                .id
                .as_ref()
                .and_then(|id| pagos_por_plan.get(&id.to_hex()))
                .map(|pp| pp.cuotas.clone())
                .unwrap_or_default();
            cuotas_por_vencer(p, &pagadas, hoy, 30)
        })
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

pub async fn evaluar_credito(
    State(client): State<mongodb::Client>,
    _sesion: EmpresaSession,
    Json(payload): Json<EvaluarReq>
) -> Json<serde_json::Value> {
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

            Json(serde_json::json!(EvaluarRes {
                status: "success".to_string(),
                estado: estado.to_string(),
                pago_mensual,
                tasa_interes: tasa,
                plan_pagos,
                consideraciones,
            }))
        },
        Ok(None) => Json(serde_json::json!({
            "status": "error",
            "message": "Cliente no encontrado"
        })),
        Err(e) => {
            eprintln!("🚨 ERROR MONGODB: {:?}", e);
            Json(serde_json::json!({
                "status": "error",
                "message": "Error en la base de datos"
            }))
        }
    }
}

pub async fn autorizar_credito(
    State(client): State<mongodb::Client>,
    sesion: EmpresaSession,
    Json(payload): Json<AutorizarReq>
) -> Json<serde_json::Value> {
    let plan_pago = PlanPago {
        id: None, // Mongo lo genera al insertar
        empresa: sesion.correo.clone(),
        cliente_curp: payload.cliente_curp.clone(),
        producto: payload.producto.clone(),
        monto_total: payload.monto_total,
        plazo_meses: payload.plazo_meses,
        pago_mensual: payload.pago_mensual,
        tasa_interes: payload.tasa_interes,
        estado: "Activo".to_string(),
        fecha: chrono::Local::now().format("%Y-%m-%d").to_string(),
    };

    let coll_planes = client.database("pymza").collection::<PlanPago>("planes_pago");
    // El frontend necesita el _id del plan para registrar pagos (ola 4).
    let inserted_id = match coll_planes.insert_one(plan_pago, None).await {
        Ok(res) => res.inserted_id,
        Err(e) => {
            eprintln!("🚨 ERROR AL GUARDAR PLAN DE PAGO: {:?}", e);
            return Json(serde_json::json!({"status": "error", "message": "Error al guardar el plan de pago"}));
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

    Json(serde_json::json!({"status": "success", "plan_id": plan_id}))
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
    let creditos: Vec<serde_json::Value> = planes
        .iter()
        .map(|plan| {
            let pagadas = plan
                .id
                .as_ref()
                .and_then(|id| pagos_por_plan.get(&id.to_hex()))
                .map(|pp| pp.cuotas.clone())
                .unwrap_or_default();
            plan_json(plan, pagadas.len() as i32, cuotas_vencidas(plan, &pagadas, hoy))
        })
        .collect();
    Json(serde_json::json!({ "status": "success", "creditos": creditos }))
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
    let mut cuotas = pagos_por_plan
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

    let pago = Pago {
        plan_id: plan.id.clone().unwrap_or_default(),
        empresa: sesion.correo.clone(),
        cliente_curp: plan.cliente_curp.clone(),
        cuota: payload.cuota,
        monto: payload.monto,
        fecha: Utc::now().format("%Y-%m-%d").to_string(),
    };
    if let Err(e) = client.database("pymza").collection::<Pago>("pagos").insert_one(pago, None).await {
        eprintln!("🚨 ERROR AL GUARDAR PAGO: {:?}", e);
        return Err(error_status(StatusCode::INTERNAL_SERVER_ERROR, "Error al registrar el pago"));
    }
    cuotas.push(payload.cuota);

    // Recalcula el estado del plan y persiste solo si cambió.
    let hoy = Utc::now().date_naive();
    let estado_nuevo = estado_plan(&plan, &cuotas, hoy);
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
        "plan": plan_json(
            &planes[idx],
            cuotas.len() as i32,
            cuotas_vencidas(&planes[idx], &cuotas, hoy),
        ),
    })))
}

pub async fn obtener_dashboard(
    State(client): State<mongodb::Client>,
    sesion: EmpresaSession,
) -> Json<serde_json::Value> {
    let coll = client.database("pymza").collection::<DashboardStats>("dashboard_stats");

    match coll.find_one(mongodb::bson::doc! { "empresa": &sesion.correo }, None).await {
        Ok(Some(stats)) => Json(serde_json::json!({
            "status": "success",
            "stats": stats
        })),
        Ok(None) => Json(serde_json::json!({
            "status": "success",
            "stats": { "empresa": sesion.correo, "creditos_activos": 0, "capital_prestado": 0.0, "proximos_cobros": 0 }
        })),
        Err(_) => Json(serde_json::json!({"status": "error"})),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::bson::oid::ObjectId;

    #[test]
    fn tasa_por_plazo_devuelve_la_tasa_esperada() {
        assert_eq!(tasa_por_plazo(3), 0.03);
        assert_eq!(tasa_por_plazo(6), 0.06);
        assert_eq!(tasa_por_plazo(9), 0.10);
        assert_eq!(tasa_por_plazo(12), 0.15);
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
        // cuota 1 pagada; cuota 2 vence 2026-08-01 (futuro)
        assert_eq!(estado_plan(&plan, &[1], hoy), "Activo");
        // el día del vencimiento aún NO es moroso (vencimiento < hoy estricto)
        assert_eq!(estado_plan(&plan, &[], hoy), "Activo");
    }

    #[test]
    fn estado_plan_cuota_atrasada_es_moroso() {
        let plan = plan_ejemplo(); // fecha 2026-01-01, plazo 6
        let hoy = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        // cuota 1 venció 2026-02-01 y cuota 2 el 2026-03-01
        assert_eq!(estado_plan(&plan, &[], hoy), "Moroso");
        assert_eq!(cuotas_vencidas(&plan, &[], hoy), 2);
        // pagando la cuota 1 sigue moroso por la cuota 2
        assert_eq!(estado_plan(&plan, &[1], hoy), "Moroso");
        assert_eq!(cuotas_vencidas(&plan, &[1], hoy), 1);
    }

    #[test]
    fn estado_plan_todo_pagado_es_liquidado() {
        let plan = plan_ejemplo();
        let hoy = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        assert_eq!(estado_plan(&plan, &[1, 2, 3, 4, 5, 6], hoy), "Liquidado");
        assert_eq!(cuotas_vencidas(&plan, &[1, 2, 3, 4, 5, 6], hoy), 0);
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
    fn plan_json_expone_id_y_avance() {
        let mut plan = plan_ejemplo();
        plan.id = ObjectId::parse_str("507f1f77bcf86cd799439011").ok();
        let v = plan_json(&plan, 2, 1);
        assert_eq!(v["_id"], "507f1f77bcf86cd799439011");
        assert_eq!(v["cuotas_pagadas"], 2);
        assert_eq!(v["cuotas_vencidas"], 1);
        assert_eq!(v["estado"], "Activo");
    }
}
