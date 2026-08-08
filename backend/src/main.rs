mod db;
mod models;

use axum::{
    extract::State,
    http::{HeaderValue, Method},
    routing::{post, get},
    Json,
    Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use std::net::SocketAddr;
use futures::StreamExt;
use models::credito::{EvaluarReq, AutorizarReq, PlanPago, EvaluarRes, PagoInfo, DashboardStats};
use models::cliente::{Cliente, CrearClienteReq};

#[derive(Deserialize, Serialize)]
struct LoginPayload {
    correo: String,
    password: String,
}

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

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let db_client = db::connect().await.expect("Fallo crítico en conexión DB");

    let app = Router::new()
        .route("/api/ocr", post(process_ocr))
        .route("/api/login", post(login_empresa))
        .route("/api/empresas", post(alta_empresa))
        .route("/api/clientes", post(crear_cliente))
        .route("/api/clientes/:curp", get(buscar_cliente))
        .route("/api/creditos/evaluar", post(evaluar_credito))
        .route("/api/creditos/autorizar", post(autorizar_credito))
        .route("/api/creditos/:empresa", get(obtener_creditos))
        .route("/api/dashboard/:empresa", get(obtener_dashboard))
        .layer(cors_layer())
        .with_state(db_client);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Servidor PYMZA escuchando en {}", addr);
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
}

fn cors_layer() -> CorsLayer {
    // ponytail: allowlist fija de dev local (frontend `dx serve` en :8080, API en
    // 127.0.0.1:3000). Para producción, sacar los orígenes a una env var
    // (p. ej. ALLOWED_ORIGINS) y construir el array desde ahí.
    CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:8080"),
            HeaderValue::from_static("http://127.0.0.1:8080"),
        ])
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any)
}

async fn process_ocr(State(_client): State<mongodb::Client>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "success", "id": "12345"}))
}

async fn login_empresa(
    State(client): State<mongodb::Client>,
    axum::Json(payload): axum::Json<LoginPayload>
) -> Json<serde_json::Value> {
    let coll = client.database("pymza").collection::<models::empresa::Empresa>("empresas");

    match coll.find_one(
        mongodb::bson::doc! { "correo": payload.correo, "password": payload.password },
        None
    ).await {
        Ok(Some(empresa)) => Json(serde_json::json!({
            "status": "success",
            "empresa": empresa.nombre_empresa,
            "token": "token-temporal-123"
        })),
        Ok(None) => Json(serde_json::json!({
            "status": "error",
            "message": "Credenciales inválidas"
        })),
        Err(e) => {
            eprintln!("🚨 ERROR MONGODB: {:?}", e);
            Json(serde_json::json!({
                "status": "error",
                "message": "Credenciales inválidas"
            }))
        }
    }
}

fn es_correo_valido(correo: &str) -> bool {
    if correo.contains(' ') {
        return false;
    }
    let mut partes = correo.split('@');
    let local = partes.next().unwrap_or("");
    let dominio = partes.next().unwrap_or("");
    let tiene_un_solo_arroba = partes.next().is_none();
    !local.is_empty() && dominio.contains('.') && tiene_un_solo_arroba
}

async fn alta_empresa(
    State(client): State<mongodb::Client>,
    Json(payload): Json<models::empresa::Empresa>,
) -> Json<serde_json::Value> {
    if !es_correo_valido(&payload.correo) {
        return Json(serde_json::json!({
            "status": "error",
            "message": "Correo inválido"
        }));
    }
    if payload.password.chars().count() < 8 {
        return Json(serde_json::json!({
            "status": "error",
            "message": "La contraseña debe tener al menos 8 caracteres"
        }));
    }

    let coll = client.database("pymza").collection::<models::empresa::Empresa>("empresas");

    if let Ok(Some(_)) = coll.find_one(mongodb::bson::doc! { "correo": &payload.correo }, None).await {
        return Json(serde_json::json!({
            "status": "error",
            "message": "Ya existe una empresa registrada con ese correo"
        }));
    }

    match coll.insert_one(&payload, None).await {
        Ok(_) => Json(serde_json::json!({
            "status": "success",
            "empresa": {
                "correo": payload.correo,
                "nombre_empresa": payload.nombre_empresa,
            }
        })),
        Err(e) => {
            eprintln!("🚨 ERROR MONGODB: {:?}", e);
            Json(serde_json::json!({ "status": "error", "message": "Error al registrar la empresa" }))
        }
    }
}

async fn buscar_cliente(
    State(client): State<mongodb::Client>,
    axum::extract::Path(curp): axum::extract::Path<String>
) -> Json<serde_json::Value> {
    let coll = client.database("pymza").collection::<models::cliente::Cliente>("clientes");

    match coll.find_one(
        mongodb::bson::doc! { "curp": curp },
        None
    ).await {
        Ok(Some(cliente)) => Json(serde_json::json!({
            "status": "success",
            "cliente": cliente
        })),
        Ok(None) => Json(serde_json::json!({
            "status": "not_found",
            "message": "Cliente no existe en la red PYMZA"
        })),
        Err(e) => {
            eprintln!("🚨 ERROR MONGODB: {:?}", e);
            Json(serde_json::json!({"status": "error"}))
        }
    }
}

async fn crear_cliente(
    State(client): State<mongodb::Client>,
    Json(payload): Json<CrearClienteReq>,
) -> Json<serde_json::Value> {
    if !es_curp_valida(&payload.curp) {
        return Json(serde_json::json!({
            "status": "error",
            "message": "CURP inválida: debe tener estructura CURP válida (18 caracteres, mayúsculas, fecha, sexo y entidad correctos)"
        }));
    }

    let coll = client.database("pymza").collection::<Cliente>("clientes");

    if let Ok(Some(_)) = coll.find_one(mongodb::bson::doc! { "curp": &payload.curp }, None).await {
        return Json(serde_json::json!({
            "status": "error",
            "message": "Cliente ya existe en la red PYMZA"
        }));
    }

    let cliente = Cliente {
        curp: payload.curp,
        nombre_completo: payload.nombre_completo,
        score: 550,
        nivel_riesgo: "Medio".to_string(),
        historial_pagos: "Sin historial en la red".to_string(),
        direccion: payload.direccion,
        telefono: payload.telefono,
    };

    match coll.insert_one(&cliente, None).await {
        Ok(_) => Json(serde_json::json!({ "status": "success", "cliente": cliente })),
        Err(e) => {
            eprintln!("🚨 ERROR MONGODB: {:?}", e);
            Json(serde_json::json!({ "status": "error", "message": "Error al guardar el cliente" }))
        }
    }
}

async fn obtener_creditos(
    State(client): State<mongodb::Client>,
    axum::extract::Path(empresa): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let coll = client.database("pymza").collection::<PlanPago>("planes_pago");

    match coll.find(mongodb::bson::doc! { "empresa": &empresa }, None).await {
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

const ENTIDADES_CURP: [&str; 33] = [
    "AS", "BC", "BS", "CC", "CL", "CM", "CS", "CH", "DF", "DG",
    "GT", "GR", "HG", "JC", "MC", "MN", "MS", "NT", "NL", "OC",
    "PL", "QT", "QR", "SP", "SL", "SR", "TC", "TS", "TL", "VZ",
    "YN", "ZS", "NE",
];

fn es_curp_valida(curp: &str) -> bool {
    let b = curp.as_bytes();
    if b.len() != 18 {
        return false;
    }
    if !b.iter().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
        return false;
    }
    // Posiciones 1-4: letras (iniciales de apellidos y nombre)
    if !b[..4].iter().all(|c| c.is_ascii_uppercase()) {
        return false;
    }
    // Posiciones 5-10: fecha de nacimiento YYMMDD
    if !b[4..10].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let mes = (b[6] - b'0') as i32 * 10 + (b[7] - b'0') as i32;
    let dia = (b[8] - b'0') as i32 * 10 + (b[9] - b'0') as i32;
    if !(1..=12).contains(&mes) || !(1..=31).contains(&dia) {
        return false;
    }
    // Posición 11: sexo
    if b[10] != b'H' && b[10] != b'M' {
        return false;
    }
    // Posiciones 12-13: entidad federativa de nacimiento
    let entidad = std::str::from_utf8(&b[11..13]).expect("CURP ASCII");
    ENTIDADES_CURP.contains(&entidad)
}

async fn evaluar_credito(
    State(client): State<mongodb::Client>,
    Json(payload): Json<EvaluarReq>
) -> Json<serde_json::Value> {
    let coll_clientes = client.database("pymza").collection::<models::cliente::Cliente>("clientes");

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

async fn autorizar_credito(
    State(client): State<mongodb::Client>,
    Json(payload): Json<AutorizarReq>
) -> Json<serde_json::Value> {
    let plan_pago = PlanPago {
        empresa: payload.empresa.clone(),
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
    let filter = mongodb::bson::doc! { "empresa": &payload.empresa };
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

async fn obtener_dashboard(
    State(client): State<mongodb::Client>,
    axum::extract::Path(empresa): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let coll = client.database("pymza").collection::<DashboardStats>("dashboard_stats");

    match coll.find_one(mongodb::bson::doc! { "empresa": &empresa }, None).await {
        Ok(Some(stats)) => Json(serde_json::json!({
            "status": "success",
            "stats": stats
        })),
        Ok(None) => Json(serde_json::json!({
            "status": "success",
            "stats": { "empresa": empresa.clone(), "creditos_activos": 0, "capital_prestado": 0.0, "proximos_cobros": 0 }
        })),
        Err(_) => Json(serde_json::json!({"status": "error"})),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURPS_SEED: [&str; 3] = [
        "RAMJ920215MDFMZR03",
        "GAML930528HDFLNR05",
        "GARV850710MCHLRN09",
    ];

    #[test]
    fn tasa_por_plazo_devuelve_la_tasa_esperada() {
        assert_eq!(tasa_por_plazo(3), 0.03);
        assert_eq!(tasa_por_plazo(6), 0.06);
        assert_eq!(tasa_por_plazo(9), 0.10);
        assert_eq!(tasa_por_plazo(12), 0.15);
        assert_eq!(tasa_por_plazo(4), 0.05);
    }

    #[test]
    fn es_curp_valida_acepta_las_curps_del_seed() {
        for curp in CURPS_SEED {
            assert!(es_curp_valida(curp), "CURP del seed debe ser válida: {}", curp);
        }
    }

    #[test]
    fn es_curp_valida_rechaza_curps_invalidas() {
        assert!(!es_curp_valida(""));
        assert!(!es_curp_valida("RAMJ920215MDFMZR0")); // 17 chars
        assert!(!es_curp_valida("RAMJ920215MDFMZR031")); // 19 chars
        assert!(!es_curp_valida("RAMJ920215MDFMZR0!")); // char no alfanumérico
    }

    #[test]
    fn correo_valido() {
        assert!(es_correo_valido("demo@pymza.mx"));
    }

    #[test]
    fn correo_invalido() {
        assert!(!es_correo_valido("sin-arroba"));
        assert!(!es_correo_valido("a@b")); // sin dominio con punto
        assert!(!es_correo_valido("@pymza.mx")); // sin parte local
        assert!(!es_correo_valido("a b@pymza.mx")); // con espacio
        assert!(!es_correo_valido("a@b@c.mx")); // más de un @
    }

    #[test]
    fn curps_del_seed_son_validas() {
        for curp in ["RAMJ920215MDFMZR03", "GAML930528HDFLNR05", "GARV850710MCHLRN09"] {
            assert!(es_curp_valida(curp), "{} debería ser válida", curp);
        }
    }

    #[test]
    fn rechaza_minusculas() {
        assert!(!es_curp_valida("ramj920215mdfmzr03"));
    }

    #[test]
    fn rechaza_longitud_incorrecta() {
        assert!(!es_curp_valida(""));
        assert!(!es_curp_valida("RAMJ920215MDFMZR0"));
        assert!(!es_curp_valida("RAMJ920215MDFMZR030"));
    }

    #[test]
    fn rechaza_fecha_invalida() {
        assert!(!es_curp_valida("RAMJ921315MDFMZR03")); // mes 13
        assert!(!es_curp_valida("RAMJ920232MDFMZR03")); // día 32
        assert!(!es_curp_valida("RAMJ92M215MDFMZR03")); // año con letra
    }

    #[test]
    fn rechaza_sexo_invalido() {
        assert!(!es_curp_valida("RAMJ920215XDFMZR03"));
    }

    #[test]
    fn rechaza_entidad_invalida() {
        assert!(!es_curp_valida("RAMJ920215MXXMZR03"));
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
