mod db;

use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use std::net::SocketAddr;

#[derive(Deserialize, Serialize)]
struct UpdateStatusPayload { id: String, estado: String }

#[tokio::main]
async fn main() {
    // Inicialización única
    let db_client = db::connect().await.expect("Fallo crítico en conexión DB");

    let app = Router::new()
        .route("/api/update_status", post(update_status))
        .route("/api/ocr", post(process_ocr))
        .layer(CorsLayer::permissive())
        .with_state(db_client);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Servidor PYMZA escuchando en {}", addr);
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
}

async fn update_status(
    State(client): State<mongodb::Client>,
    Json(payload): Json<UpdateStatusPayload>
) -> Json<serde_json::Value> {
    let coll = client.database("pymza").collection::<mongodb::bson::Document>("solicitudes");
    
    let _ = coll.update_one(
        mongodb::bson::doc! { "id": payload.id },
        mongodb::bson::doc! { "$set": { "estado": payload.estado } },
        None
    ).await;

    Json(serde_json::json!({"status": "success"}))
}

async fn process_ocr(State(_client): State<mongodb::Client>) -> Json<serde_json::Value> {
    // Pendiente: implementar lógica real de OCR
    Json(serde_json::json!({"status": "success", "id": "12345"}))
}
