use axum::{extract::State, Json};

use crate::auth::{es_correo_valido, hashear_password};
use crate::models::empresa::Empresa;

pub async fn alta_empresa(
    State(client): State<mongodb::Client>,
    Json(payload): Json<Empresa>,
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

    let coll = client.database("pymza").collection::<mongodb::bson::Document>("empresas");

    if let Ok(Some(_)) = coll.find_one(mongodb::bson::doc! { "correo": &payload.correo }, None).await {
        return Json(serde_json::json!({
            "status": "error",
            "message": "Ya existe una empresa registrada con ese correo"
        }));
    }

    let password_hash = match hashear_password(&payload.password) {
        Ok(hash) => hash,
        Err(e) => {
            eprintln!("🚨 ERROR AL HASHEAR PASSWORD: {:?}", e);
            return Json(serde_json::json!({
                "status": "error",
                "message": "Error al registrar la empresa"
            }));
        }
    };
    // ponytail: password tiene #[serde(skip_serializing)] (nunca sale en JSON),
    // así que el insert se arma con doc! para persistirla igualmente.
    let empresa_doc = mongodb::bson::doc! {
        "correo": &payload.correo,
        "password": password_hash,
        "nombre_empresa": &payload.nombre_empresa,
    };

    match coll.insert_one(empresa_doc, None).await {
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
