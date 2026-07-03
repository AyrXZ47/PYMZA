mod db;
mod models;
mod services;

use axum::{
    extract::State,
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use tower_http::cors::CorsLayer;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // 1. Conectar a Mongo UNA SOLA VEZ
    let db_client = db::connect().await.expect("Fallo al conectar DB");

    // 2. Armar el Router inyectando el estado
    let app = Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/ocr", post(process_ocr))
        .route("/api/update_status", post(update_status))
        .layer(CorsLayer::permissive())
        .with_state(db_client); // ESTO ES EL POOL

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn health_handler() -> &'static str {
    "{\"status\": \"ok\", \"message\": \"Motor PYMZA en línea\"}"
}

// 3. Los handlers ahora reciben el State
async fn process_ocr(State(db_client): State<mongodb::Client>) -> axum::Json<serde_json::Value> {
    // Usamos el cliente compartido para acceder a la base de datos
    let coll = db_client.database("pymza").collection::<mongodb::bson::Document>("solicitudes");
    
    // (Simulación temporal para que no chille el compilador, luego pondremos tu modelo real)
    let _ = coll.insert_one(mongodb::bson::doc! {
        "document_type": "INE",
        "confidence_score": 98,
        "extracted_name": "Janeth Ramos Zamora"
    }, None).await;

    axum::Json(serde_json::json!({
        "status": "success",
        "document_type": "INE",
        "confidence_score": 98,
        "extracted_name": "Janeth Ramos Zamora"
    }))
}

#[derive(Deserialize)]
struct UpdateStatusPayload {
    id: String,
    estado: String,
}

async fn update_status(
    State(db_client): State<mongodb::Client>, 
    axum::Json(payload): axum::Json<UpdateStatusPayload>
) -> axum::Json<serde_json::Value> {
    
    let coll = db_client.database("pymza").collection::<mongodb::bson::Document>("solicitudes");
    
    let _ = coll.update_one(
        mongodb::bson::doc! { "id": &payload.id },
        mongodb::bson::doc! { "$set": { "estado": &payload.estado } },
        None
    ).await;

    axum::Json(serde_json::json!({"status": "success"}))
}
