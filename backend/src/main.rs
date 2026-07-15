mod db;
mod models;

use axum::{
    extract::{State, Path},
    routing::{post, get},
    Json,
    Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use std::net::SocketAddr;

#[derive(Deserialize, Serialize)]
struct UpdateStatusPayload { id: String, estado: String }

#[derive(Deserialize, Serialize)]
struct LoginPayload {
    correo: String,
    password: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // Inicialización única
    let db_client = db::connect().await.expect("Fallo crítico en conexión DB");

    let app = Router::new()
        .route("/api/update_status", post(update_status))
        .route("/api/ocr", post(process_ocr))
        .route("/api/login", post(login_empresa)) // New route for login
        .route("/api/clientes/:curp", get(buscar_cliente)) // New route for cliente search
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
            Json(serde_json::json!({
                "status": "error"
            }))
        }
    }
}
