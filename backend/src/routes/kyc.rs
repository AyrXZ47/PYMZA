//! KYC de INE y score por recibos de servicios (ola 5).
//!
//! ponytail: módulo nuevo en vez de crecer cliente.rs — los dos endpoints
//! comparten la validación del archivo subido (mime, tamaño, base64) y la
//! corrida de OCR; el techo es separarlos por endpoint si alguno crece.
//!
//! Trust boundary, en orden: mime → tamaño (por LARGO del b64, antes de
//! decodificar — evita decodificar payloads enormes) → base64 válido →
//! cliente existe (404). La empresa (tenant) sale del token, nunca del body.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use base64::Engine as _;
use chrono::Utc;
use serde::Deserialize;

use crate::auth::EmpresaSession;
use crate::models::cliente::Cliente;
use crate::ocr::{buscar_curp, buscar_monto, buscar_nombre, extraer_texto};

/// Máximo de la imagen decodificada: 2 MB.
const MAX_BYTES: usize = 2 * 1024 * 1024;

/// Nivel de riesgo por los umbrales del contrato ola 5. Función PURA.
pub fn nivel_por_score(score: i32) -> &'static str {
    if score >= 750 {
        "Bajo"
    } else if score >= 550 {
        "Medio"
    } else {
        "Alto"
    }
}

/// mimes de imagen aceptados (contrato ola 5). Función PURA.
fn mime_permitido(mime: &str) -> bool {
    matches!(mime, "image/png" | "image/jpeg" | "image/webp")
}

/// ¿El b64 (SIN decodificar) excede el máximo de 2 MB decodificados?
/// Decodificado ≈ 3/4 del largo; el borde exacto (2 MB justo) pasa. Función PURA.
fn b64_excede_maximo(largo_b64: usize) -> bool {
    largo_b64 > 4 * ((MAX_BYTES + 2) / 3)
}

/// Valida el archivo subido en el orden del contrato. Devuelve los bytes
/// decodificados o el mensaje del 400.
fn validar_archivo(b64: &str, mime: &str) -> Result<Vec<u8>, &'static str> {
    if !mime_permitido(mime) {
        return Err("Mime no permitido: solo image/png, image/jpeg o image/webp");
    }
    if b64_excede_maximo(b64.len()) {
        return Err("El archivo excede el máximo de 2 MB");
    }
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|_| "Base64 inválido")
}

#[derive(Deserialize)]
pub struct KycReq {
    pub archivo_b64: String,
    pub mime: String,
}

/// POST /api/clientes/:curp/kyc — verifica que la INE subida sea legible y
/// que su CURP coincida con el cliente; si coincide marca `ine_verificada`.
/// La imagen NO se guarda: solo el resultado.
pub async fn kyc_verificar_ine(
    State(client): State<mongodb::Client>,
    _sesion: EmpresaSession,
    axum::extract::Path(curp): axum::extract::Path<String>,
    Json(payload): Json<KycReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let bytes = match validar_archivo(&payload.archivo_b64, &payload.mime) {
        Ok(b) => b,
        Err(message) => return Err(error_status(StatusCode::BAD_REQUEST, message)),
    };

    let clientes = client.database("pymza").collection::<Cliente>("clientes");
    let Some(cliente) = clientes
        .find_one(mongodb::bson::doc! { "curp": &curp }, None)
        .await
        .map_err(error_db)?
    else {
        return Err(no_encontrado("Cliente no existe en la red PYMZA"));
    };

    let texto = match extraer_texto(&bytes, &payload.mime).await {
        Ok(t) => t,
        Err(_) => {
            return Err(error_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                "OCR no disponible en este servidor",
            ))
        }
    };

    let curp_ine = buscar_curp(&texto);
    let nombre_ine = buscar_nombre(&texto);
    let coincide = curp_ine.as_deref() == Some(curp.as_str());

    if coincide && !cliente.ine_verificada {
        clientes
            .update_one(
                mongodb::bson::doc! { "curp": &curp },
                mongodb::bson::doc! { "$set": { "ine_verificada": true } },
                None,
            )
            .await
            .map_err(error_db)?;
    }

    let mut resp = serde_json::json!({
        "status": "success",
        "curp_ine": curp_ine,
        "nombre_ine": nombre_ine,
        "coincide": coincide,
        // Estado final del cliente: se marcó ahora o ya estaba verificada.
        "ine_verificada": coincide || cliente.ine_verificada,
    });
    if !coincide {
        resp["message"] = serde_json::json!(if curp_ine.is_some() {
            "La CURP de la INE no coincide con el cliente"
        } else {
            "No se encontró una CURP legible en la imagen"
        });
    }
    Ok(Json(resp))
}

#[derive(Deserialize)]
pub struct ReciboReq {
    pub archivo_b64: String,
    pub mime: String,
    pub tipo: String,
}

/// POST /api/clientes/:curp/recibos — sube un recibo de servicios; si el OCR
/// lo lee (monto o texto ≥50 chars) suma 25 al score (máx 2 recibos por
/// cliente) y recalcula el nivel de riesgo. La imagen NO se guarda.
pub async fn recibo_subir(
    State(client): State<mongodb::Client>,
    sesion: EmpresaSession,
    axum::extract::Path(curp): axum::extract::Path<String>,
    Json(payload): Json<ReciboReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if !matches!(payload.tipo.as_str(), "luz" | "agua" | "telefono") {
        return Err(error_status(
            StatusCode::BAD_REQUEST,
            "Tipo inválido: debe ser luz, agua o telefono",
        ));
    }
    let bytes = match validar_archivo(&payload.archivo_b64, &payload.mime) {
        Ok(b) => b,
        Err(message) => return Err(error_status(StatusCode::BAD_REQUEST, message)),
    };

    let clientes = client.database("pymza").collection::<Cliente>("clientes");
    let Some(cliente) = clientes
        .find_one(mongodb::bson::doc! { "curp": &curp }, None)
        .await
        .map_err(error_db)?
    else {
        return Err(no_encontrado("Cliente no existe en la red PYMZA"));
    };

    let texto = match extraer_texto(&bytes, &payload.mime).await {
        Ok(t) => t,
        Err(_) => {
            return Err(error_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                "OCR no disponible en este servidor",
            ))
        }
    };
    let monto_leido = buscar_monto(&texto);
    let legible = monto_leido.is_some() || texto.chars().count() >= 50;

    // El tope de 2 es GLOBAL por curp (la señal de pago es de la red, no por
    // empresa); `empresa` guarda solo quién subió (tenant del token).
    let recibos = client
        .database("pymza")
        .collection::<mongodb::bson::Document>("recibos");
    let n = recibos
        .count_documents(mongodb::bson::doc! { "curp": &curp }, None)
        .await
        .map_err(error_db)?;

    let mut score = cliente.score;
    let mut nivel = cliente.nivel_riesgo.clone();
    if legible {
        if n >= 2 {
            return Err(error_status(
                StatusCode::BAD_REQUEST,
                "Máximo 2 recibos por cliente",
            ));
        }
        recibos
            .insert_one(
                mongodb::bson::doc! {
                    "curp": &curp,
                    "empresa": &sesion.correo,
                    "tipo": &payload.tipo,
                    // f64 o null: nunca se guarda la imagen, solo lo leído.
                    "monto_leido": monto_leido,
                    "fecha": Utc::now().format("%Y-%m-%d").to_string(),
                },
                None,
            )
            .await
            .map_err(error_db)?;
        score += 25;
        nivel = nivel_por_score(score).to_string();
        clientes
            .update_one(
                mongodb::bson::doc! { "curp": &curp },
                mongodb::bson::doc! { "$set": { "score": score, "nivel_riesgo": &nivel } },
                None,
            )
            .await
            .map_err(error_db)?;
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "monto_leido": monto_leido,
        "score": score,
        "nivel_riesgo": nivel,
        "recibos_contados": n + u64::from(legible),
    })))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nivel_por_score_umbrales_del_contrato() {
        assert_eq!(nivel_por_score(750), "Bajo");
        assert_eq!(nivel_por_score(550), "Medio");
        assert_eq!(nivel_por_score(549), "Alto");
        // bordes
        assert_eq!(nivel_por_score(749), "Medio");
        assert_eq!(nivel_por_score(800), "Bajo");
        assert_eq!(nivel_por_score(0), "Alto");
    }

    #[test]
    fn mime_permitido_solo_imagenes() {
        assert!(mime_permitido("image/png"));
        assert!(mime_permitido("image/jpeg"));
        assert!(mime_permitido("image/webp"));
        assert!(!mime_permitido("image/gif"));
        assert!(!mime_permitido("text/plain"));
        assert!(!mime_permitido(""));
        assert!(!mime_permitido("application/pdf"));
    }

    #[test]
    fn b64_excede_maximo_en_el_borde_exacto() {
        // 2 MB decodificados exactos caben (4 * ceil(2MB/3) caracteres b64).
        assert!(!b64_excede_maximo(4 * ((MAX_BYTES + 2) / 3)));
        assert!(b64_excede_maximo(4 * ((MAX_BYTES + 2) / 3) + 1));
        // archivos normales pasan sobrado
        assert!(!b64_excede_maximo(0));
        assert!(!b64_excede_maximo(600_000));
    }

    #[test]
    fn validar_archivo_orden_mime_tamano_b64() {
        // mime inválido primero
        assert_eq!(
            validar_archivo("AAAA", "text/plain").err(),
            Some("Mime no permitido: solo image/png, image/jpeg o image/webp")
        );
        // tamaño por largo b64 (sin decodificar): >2MB decodificados
        let largo_2mb_mas = "A".repeat(4 * ((MAX_BYTES + 2) / 3) + 1);
        assert_eq!(
            validar_archivo(&largo_2mb_mas, "image/png").err(),
            Some("El archivo excede el máximo de 2 MB")
        );
        // base64 inválido
        assert_eq!(
            validar_archivo("no-es-base64!!", "image/png").err(),
            Some("Base64 inválido")
        );
        // feliz: decodifica
        assert_eq!(validar_archivo("aGVsbG8=", "image/png").unwrap(), b"hello");
    }
}
