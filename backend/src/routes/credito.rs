use axum::{
    extract::State,
    Json,
};
use futures::StreamExt;

use crate::auth::EmpresaSession;
use crate::models::cliente::Cliente;
use crate::models::credito::{AutorizarReq, DashboardStats, EvaluarReq, EvaluarRes, PagoInfo, PlanPago};

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
    if let Err(e) = coll_planes.insert_one(plan_pago, None).await {
        eprintln!("🚨 ERROR AL GUARDAR PLAN DE PAGO: {:?}", e);
        return Json(serde_json::json!({"status": "error", "message": "Error al guardar el plan de pago"}));
    }

    let coll_stats = client.database("pymza").collection::<DashboardStats>("dashboard_stats");
    let filter = mongodb::bson::doc! { "empresa": &sesion.correo };
    let update = mongodb::bson::doc! {
        "$inc": {
            "creditos_activos": 1,
            "capital_prestado": payload.monto_total,
            "proximos_cobros": payload.plazo_meses as i32,
        }
    };
    let opts = mongodb::options::UpdateOptions::builder().upsert(true).build();
    if let Err(e) = coll_stats.update_one(filter, update, opts).await {
        eprintln!("🚨 ERROR AL ACTUALIZAR DASHBOARD: {:?}", e);
    }

    Json(serde_json::json!({"status": "success"}))
}

pub async fn obtener_creditos(
    State(client): State<mongodb::Client>,
    sesion: EmpresaSession,
) -> Json<serde_json::Value> {
    let coll = client.database("pymza").collection::<PlanPago>("planes_pago");

    match coll.find(mongodb::bson::doc! { "empresa": &sesion.correo }, None).await {
        Ok(mut cursor) => {
            let mut creditos = Vec::new();
            while let Some(Ok(plan)) = cursor.next().await {
                creditos.push(plan);
            }
            Json(serde_json::json!({ "status": "success", "creditos": creditos }))
        }
        Err(e) => {
            eprintln!("🚨 ERROR MONGODB: {:?}", e);
            Json(serde_json::json!({ "status": "error" }))
        }
    }
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
}
