use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use std::net::SocketAddr;
use tokio;

mod db;
mod models;
mod services;

#[tokio::main]
async fn main() {
    // Initialize MongoDB connection
    if let Err(e) = db::connect().await {
        eprintln!("Failed to connect to MongoDB: {}", e);
        return;
    }

    // Create a basic Axum router
    let app = Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/ocr", post(process_ocr))
        .route("/api/update_status", post(update_status))  // New route for updating status
        .layer(CorsLayer::permissive());

    // Run the server on port 3000
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

async fn process_ocr() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "success",
        "document_type": "INE",
        "confidence_score": 98,
        "extracted_name": "Janeth Ramos Zamora"
    }))
}

// New handler function for updating status
async fn update_status(axum::Json(payload): axum::Json<UpdateStatusPayload>) -> axum::Json<serde_json::Value> {
    let client = db::connect().await.unwrap();
    let collection = client.database("pymza").collection::<models::solicitud::Solicitud>("solicitudes");
    let filter = doc! { "id": payload.id };
    let update = doc! { "$set": { "estado": payload.estado } };

    if let Err(e) = collection.update_one(filter, update, None).await {
        return axum::Json(serde_json::json!({
            "status": "error",
            "message": e.to_string()
        }));
    }

    axum::Json(serde_json::json!({"status": "success"}))
}

// Define the payload struct for updating status
#[derive(serde::Deserialize)]
struct UpdateStatusPayload {
    id: String,
    estado: String,
}
