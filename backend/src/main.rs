mod auth;
mod db;
mod models;
mod routes;

use std::net::SocketAddr;

use axum::{extract::State, routing::{get, post}, Json, Router};

use auth::{cors_layer, login_empresa};
use routes::cliente::{buscar_cliente, crear_cliente, reportar_cliente};
use routes::credito::{autorizar_credito, evaluar_credito, obtener_creditos, obtener_dashboard};
use routes::empresa::alta_empresa;

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
        .route("/api/clientes/:curp/reportar", post(reportar_cliente))
        .route("/api/creditos/evaluar", post(evaluar_credito))
        .route("/api/creditos/autorizar", post(autorizar_credito))
        .route("/api/creditos/:empresa", get(obtener_creditos))
        .route("/api/dashboard/:empresa", get(obtener_dashboard))
        .layer(cors_layer())
        .with_state(db_client);

    // En Docker el contenedor debe escuchar en 0.0.0.0:3000 (compose lo pasa por env);
    // por defecto se conserva el bind local actual.
    let addr: SocketAddr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_string())
        .parse()
        .expect("BIND_ADDR inválida; se espera IP:PUERTO");
    println!("Servidor PYMZA escuchando en {}", addr);
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
}

// ponytail: placeholder OCR (respuesta fija, sin DB) que aún no justifica módulo
// propio; el techo es moverlo a routes/ocr.rs cuando exista OCR real.
async fn process_ocr(State(_client): State<mongodb::Client>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "success", "id": "12345"}))
}
