mod db;
mod models;
mod services;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use std::net::SocketAddr;
use tokio;

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

async fn process_ocr() -> serde_json::Value {
    serde_json::json!({
        "status": "success",
        "document_type": "INE",
        "confidence_score": 98,
        "extracted_name": "Janeth Ramos Zamora"
    })
}
