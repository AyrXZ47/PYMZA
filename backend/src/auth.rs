use axum::{
    extract::State,
    http::{HeaderValue, Method},
    Json,
};
use serde::{Deserialize, Serialize};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use tower_http::cors::{Any, CorsLayer};

use crate::models::empresa::Empresa;

#[derive(Deserialize, Serialize)]
pub struct LoginPayload {
    correo: String,
    password: String,
}

pub async fn login_empresa(
    State(client): State<mongodb::Client>,
    axum::Json(payload): axum::Json<LoginPayload>
) -> Json<serde_json::Value> {
    let coll = client.database("pymza").collection::<Empresa>("empresas");

    match coll.find_one(
        mongodb::bson::doc! { "correo": &payload.correo },
        None
    ).await {
        Ok(Some(empresa)) => {
            if password_correcta(&payload.password, &empresa.password) {
                Json(serde_json::json!({
                    "status": "success",
                    "empresa": empresa.nombre_empresa,
                    "token": "token-temporal-123"
                }))
            } else {
                Json(serde_json::json!({
                    "status": "error",
                    "message": "Credenciales inválidas"
                }))
            }
        }
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

pub(crate) fn es_correo_valido(correo: &str) -> bool {
    if correo.contains(' ') {
        return false;
    }
    let mut partes = correo.split('@');
    let local = partes.next().unwrap_or("");
    let dominio = partes.next().unwrap_or("");
    let tiene_un_solo_arroba = partes.next().is_none();
    !local.is_empty() && dominio.contains('.') && tiene_un_solo_arroba
}

// ponytail: hashing sincrónico (~100ms, argon2id por defecto). Bloquea un worker
// del runtime async durante la operación; si el QPS de login/registro escala,
// mover a tokio::task::spawn_blocking.
pub(crate) fn hashear_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}

// Solo valida hashes PHC argon2. Sin fallback a texto plano: las bases con
// passwords en plaintext quedan obsoletas (re-seedar, ver commit T-11).
fn password_correcta(password: &str, hash_almacenado: &str) -> bool {
    match PasswordHash::new(hash_almacenado) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

pub(crate) fn cors_layer() -> CorsLayer {
    // ponytail: allowlist fija de dev local (frontend `dx serve` en :8080, API en
    // 127.0.0.1:3000). Para producción, sacar los orígenes a una env var
    // (p. ej. ALLOWED_ORIGINS) y construir el array desde ahí.
    CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:8080"),
            HeaderValue::from_static("http://127.0.0.1:8080"),
        ])
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correo_valido() {
        assert!(es_correo_valido("demo@pymza.mx"));
    }

    #[test]
    fn correo_invalido() {
        assert!(!es_correo_valido("sin-arroba"));
        assert!(!es_correo_valido("a@b")); // sin dominio con punto
        assert!(!es_correo_valido("@pymza.mx")); // sin parte local
        assert!(!es_correo_valido("a b@pymza.mx")); // con espacio
        assert!(!es_correo_valido("a@b@c.mx")); // más de un @
    }

    #[test]
    fn hashear_password_produce_hash_argon2id() {
        let hash = hashear_password("demo123").unwrap();
        assert!(hash.starts_with("$argon2id$"), "hash PHC argon2id esperado: {}", hash);
        assert_ne!(hash, "demo123");
    }

    #[test]
    fn password_correcta_valida_hash() {
        let hash = hashear_password("demo123").unwrap();
        assert!(password_correcta("demo123", &hash));
        assert!(!password_correcta("otra-password", &hash));
    }

    #[test]
    fn password_correcta_rechaza_plaintext_legacy() {
        // Nota de migración T-11: bases con plaintext quedan obsoletas; re-seedar.
        assert!(!password_correcta("demo123", "demo123"));
    }
}
