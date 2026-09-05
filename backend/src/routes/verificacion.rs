//! Verificación de teléfono por OTP (ola 3). El desafío vive en la colección
//! `verificaciones` con SOLO el hash del código (nunca en claro), ligado a
//! `curp+telefono` y expira en 10 minutos.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::Deserialize;

use crate::auth::EmpresaSession;
use crate::models::cliente::Cliente;
use crate::otp::{generar_codigo, hash_codigo, sender_activo};

const EXPIRA_MINUTOS: i64 = 10;

#[derive(Deserialize)]
pub struct SolicitarVerificacionReq {
    pub curp: String,
    pub telefono: String,
}

#[derive(Deserialize)]
pub struct ConfirmarVerificacionReq {
    pub curp: String,
    pub telefono: String,
    pub codigo: String,
}

pub async fn solicitar_verificacion(
    State(client): State<mongodb::Client>,
    _sesion: EmpresaSession,
    Json(payload): Json<SolicitarVerificacionReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let codigo = generar_codigo();
    let codigo_hash = hash_codigo(&codigo);
    let expira_en = Utc::now().timestamp_millis() + EXPIRA_MINUTOS * 60 * 1000;

    let coll = client
        .database("pymza")
        .collection::<mongodb::bson::Document>("verificaciones");

    // Reemplaza el desafío previo (vigente o no) de este par curp+telefono.
    let filtro = mongodb::bson::doc! { "curp": &payload.curp, "telefono": &payload.telefono };
    coll.delete_many(filtro, None)
        .await
        .map_err(error_db)?;
    coll.insert_one(
        mongodb::bson::doc! {
            "curp": &payload.curp,
            "telefono": &payload.telefono,
            "codigo_hash": codigo_hash, // NUNCA el código en claro
            // BSON date (no i64): el índice TTL de db.rs solo expira campos date.
            "expira_en": mongodb::bson::DateTime::from_millis(expira_en),
        },
        None,
    )
    .await
    .map_err(error_db)?;

    sender_activo().enviar(&payload.telefono, &codigo).await;

    Ok(Json(serde_json::json!({ "status": "success" })))
}

pub async fn confirmar_verificacion(
    State(client): State<mongodb::Client>,
    _sesion: EmpresaSession,
    Json(payload): Json<ConfirmarVerificacionReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let clientes = client
        .database("pymza")
        .collection::<Cliente>("clientes");
    let verif = client
        .database("pymza")
        .collection::<mongodb::bson::Document>("verificaciones");
    let filtro = mongodb::bson::doc! { "curp": &payload.curp, "telefono": &payload.telefono };

    let existe_cliente = clientes
        .find_one(mongodb::bson::doc! { "curp": &payload.curp }, None)
        .await
        .map_err(error_db)?;
    if existe_cliente.is_none() {
        return Err(no_encontrado("Cliente no existe en la red PYMZA"));
    }

    let Some(desafio) = verif.find_one(filtro.clone(), None).await.map_err(error_db)? else {
        return Err(no_encontrado("No hay un código de verificación solicitado"));
    };

    // BSON date; docs legados con i64 → None → tratados como expirados y se
    // pide un código nuevo (ventana de solo 10 min, no rompe nada).
    let expira = desafio
        .get("expira_en")
        .and_then(|b| b.as_datetime())
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0);
    if expira < Utc::now().timestamp_millis() {
        // Desafío expirado: se limpia para que el cliente solicite otro.
        let _ = verif.delete_one(filtro, None).await;
        return Err(codigo_invalido());
    }

    let esperado = desafio.get_str("codigo_hash").unwrap_or("");
    if esperado != hash_codigo(&payload.codigo) {
        return Err(codigo_invalido());
    }

    // Código correcto: marca un solo campo y consume el desafío.
    clientes
        .update_one(
            mongodb::bson::doc! { "curp": &payload.curp },
            mongodb::bson::doc! { "$set": { "telefono_verificado": true } },
            None,
        )
        .await
        .map_err(error_db)?;
    verif.delete_one(filtro, None).await.map_err(error_db)?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "telefono_verificado": true
    })))
}

fn codigo_invalido() -> (StatusCode, Json<serde_json::Value>) {
    error_status(StatusCode::BAD_REQUEST, "Código inválido o expirado")
}

fn no_encontrado(message: &str) -> (StatusCode, Json<serde_json::Value>) {
    error_status(StatusCode::NOT_FOUND, message)
}

fn error_status(status: StatusCode, message: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "status": "error", "message": message })))
}

fn error_db(e: mongodb::error::Error) -> (StatusCode, Json<serde_json::Value>) {
    eprintln!("🚨 ERROR MONGODB: {e:?}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "status": "error", "message": "Error interno" })),
    )
}
