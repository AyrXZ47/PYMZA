mod auth;
mod db;
mod models;
mod ocr;
mod otp;
mod pdf;
mod routes;

use std::net::SocketAddr;

use axum::{
    error_handling::HandleErrorLayer,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    BoxError, Json, Router,
};
use tower_governor::{errors::GovernorError, governor::GovernorConfigBuilder, GovernorLayer};

use auth::{cors_layer, jwt_secret, login_empresa, EmpresaSession};
use routes::cliente::{buscar_cliente, crear_cliente, reportar_cliente};
use routes::credito::{
    autorizar_credito, descargar_contrato, evaluar_credito, obtener_creditos, obtener_dashboard,
    obtener_resumen, registrar_pago,
};
use routes::empresa::alta_empresa;
use routes::kyc::{kyc_verificar_ine, recibo_subir};
use routes::verificacion::{confirmar_verificacion, solicitar_verificacion};

// --- Rate limit por IP (ola 6): solo rutas públicas, las protegidas ya exigen JWT ---
// ponytail: 10 req/s con ráfaga 20 cubre el brute-force de login sin tocar las
// rutas de negocio; el techo es extender al flujo OTP si se abusa.

/// Límite por env: entero > 0 dentro de `max`; cualquier cosa inválida (falta,
/// vacío, no numérico, 0 o > max) usa el default. Función PURA, testeada.
fn parsear_limite(raw: Option<&str>, defecto: u64, max: u64) -> u64 {
    raw.and_then(|r| r.trim().parse::<u64>().ok())
        .filter(|v| v > &0 && v <= &max)
        .unwrap_or(defecto)
}

/// (rps, burst) del rate limiter desde `RATE_LIMIT_RPS` / `RATE_LIMIT_BURST`
/// con defaults 10/20. Función PURA (los límites llegan ya parseados).
fn limites_rate(env_rps: Option<&str>, env_burst: Option<&str>) -> (u64, u32) {
    let rps = parsear_limite(env_rps, 10, 10_000);
    let burst = parsear_limite(env_burst, 20, 100_000);
    (rps, burst as u32)
}

/// JSON del 429 según el contrato (función PURA: testeada sin tower).
fn json_429() -> serde_json::Value {
    serde_json::json!({
        "status": "error",
        "message": "Demasiadas peticiones, intenta más tarde"
    })
}

/// Convierte el GovernorError en respuesta (HandleErrorLayer): 429 con el JSON
/// del contrato — nunca un 500 crudo ni un cuerpo plano.
async fn error_governor(error: BoxError) -> Response {
    let interno = || (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"status": "error", "message": "Error interno"})),
    )
        .into_response();
    let Some(gov) = error.downcast_ref::<GovernorError>() else {
        return interno();
    };
    match gov.clone() {
        GovernorError::TooManyRequests { headers, .. } => {
            let mut res = (StatusCode::TOO_MANY_REQUESTS, Json(json_429())).into_response();
            // headers x-ratelimit-* que trae el error (x-ratelimit-after)
            if let Some(hs) = headers {
                for (k, v) in hs {
                    if let Some(k) = k {
                        res.headers_mut().insert(k, v);
                    }
                }
            }
            res
        }
        _ => interno(),
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // Falla al arrancar con mensaje claro si falta JWT_SECRET; una vez inicializada
    // aquí (OnceLock), el extractor EmpresaSession nunca vuelve a fallar por env ausente.
    jwt_secret();

    let db_client = db::connect().await.expect("Fallo crítico en conexión DB");

    // Rutas públicas con rate limit por IP (10 req/s, ráfaga 20, por env):
    // peer IP vía ConnectInfo — el make-service de abajo la inyecta. La config
    // se crea UNA vez con Box::leak (patrón del README de tower_governor: axum
    // 0.6 exige layers clonables y 'static).
    let (rps, burst) = limites_rate(
        std::env::var("RATE_LIMIT_RPS").ok().as_deref(),
        std::env::var("RATE_LIMIT_BURST").ok().as_deref(),
    );
    let conf_governor = Box::leak(Box::new(
        GovernorConfigBuilder::default()
            .per_second(rps)
            .burst_size(burst)
            .finish()
            .expect("config de rate limit válida: defaults garantizan valores > 0"),
    ));

    let publicas = Router::new()
        // Governor + HandleError encadenados sobre cada MethodRouter (soportado
        // en axum 0.6): HandleError SIEMPRE exterior a Governor para convertir
        // sus errores (429) en respuestas del contrato sin que axum los degrada a 500.
        .route(
            "/api/login",
            post(login_empresa)
                .layer(GovernorLayer { config: conf_governor })
                .layer(HandleErrorLayer::new(error_governor)),
        )
        .route(
            "/api/empresas",
            post(alta_empresa)
                .layer(GovernorLayer { config: conf_governor })
                .layer(HandleErrorLayer::new(error_governor)),
        )
        .with_state(db_client.clone());

    let app = Router::new()
        .route("/api/ocr", post(process_ocr))
        .route("/api/clientes", post(crear_cliente))
        .route("/api/clientes/:curp", get(buscar_cliente))
        .route("/api/clientes/:curp/reportar", post(reportar_cliente))
        .route("/api/clientes/:curp/kyc", post(kyc_verificar_ine))
        .route("/api/clientes/:curp/recibos", post(recibo_subir))
        .route("/api/creditos/evaluar", post(evaluar_credito))
        .route("/api/creditos/autorizar", post(autorizar_credito))
        .route("/api/creditos/pagos", post(registrar_pago))
        .route("/api/creditos/:plan_id/contrato", get(descargar_contrato))
        .route("/api/creditos", get(obtener_creditos))
        .route("/api/creditos/resumen", get(obtener_resumen))
        .route("/api/dashboard", get(obtener_dashboard))
        .route("/api/verificaciones/solicitar", post(solicitar_verificacion))
        .route("/api/verificaciones/confirmar", post(confirmar_verificacion))
        .merge(publicas)
        .layer(cors_layer())
        // Ola 6 (cierra E1 del auditor ola 5): el límite global pasa de 2MB a 3MB
        // para que el b64 de un archivo real de hasta ~2MB (~2.7MB en base64)
        // llegue al handler, que es quien rechaza >2MB decodificados con el
        // 400 del contrato — el 413 solo aparece por encima de la red de seguridad.
        .layer(DefaultBodyLimit::max(3_000_000))
        .with_state(db_client);

    // En Docker el contenedor debe escuchar en 0.0.0.0:3000 (compose lo pasa por env);
    // por defecto se conserva el bind local actual. Con ConnectInfo: el rate
    // limiter necesita la IP del peer en las extensiones del request.
    let addr: SocketAddr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_string())
        .parse()
        .expect("BIND_ADDR inválida; se espera IP:PUERTO");
    println!("Servidor PYMZA escuchando en {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

// ponytail: placeholder OCR (respuesta fija, sin DB) que aún no justifica módulo
// propio; el techo es moverlo a routes/ocr.rs cuando exista OCR real. Requiere
// sesión JWT como las demás rutas protegidas del contrato ola 1.
async fn process_ocr(
    State(_client): State<mongodb::Client>,
    _sesion: EmpresaSession,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "success", "id": "12345"}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsear_limite_defaults_y_bordes() {
        // env ausente → default
        assert_eq!(parsear_limite(None, 10, 10_000), 10);
        // vacía, no numérica, 0, fuera de rango → default
        assert_eq!(parsear_limite(Some(""), 10, 10_000), 10);
        assert_eq!(parsear_limite(Some("  "), 10, 10_000), 10);
        assert_eq!(parsear_limite(Some("abc"), 10, 10_000), 10);
        assert_eq!(parsear_limite(Some("0"), 10, 10_000), 10);
        assert_eq!(parsear_limite(Some("100001"), 10, 10_000), 10);
        // válidos (con espacios); el borde superior del rango pasa
        assert_eq!(parsear_limite(Some("5"), 10, 10_000), 5);
        assert_eq!(parsear_limite(Some(" 500 "), 10, 10_000), 500);
        assert_eq!(parsear_limite(Some("10000"), 10, 10_000), 10000);
    }

    #[test]
    fn limites_rate_defaults_10_20_y_desde_env() {
        assert_eq!(limites_rate(None, None), (10, 20));
        assert_eq!(limites_rate(Some("3"), Some("40")), (3, 40));
        // una inválida conserva su default
        assert_eq!(limites_rate(Some("abc"), None), (10, 20));
        // burst > 65535 cabe en u32
        assert_eq!(limites_rate(Some("1"), Some("70000")), (1, 70000));
    }

    #[test]
    fn json_429_shape_del_contrato() {
        let json = json_429();
        assert_eq!(json["status"], "error");
        assert_eq!(json["message"], "Demasiadas peticiones, intenta más tarde");
        assert_eq!(json.as_object().unwrap().len(), 2, "sin campos extra");
    }
}
