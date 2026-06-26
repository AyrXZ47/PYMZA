mod db;
mod models;
mod services;

use axum::{routing::get, Router};
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
        .route("/api/health", get(health_handler));

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
