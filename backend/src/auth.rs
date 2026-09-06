use std::sync::OnceLock;

use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{
        header::AUTHORIZATION,
        request::Parts,
        HeaderValue, Method, StatusCode,
    },
    Json,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

use crate::models::empresa::Empresa;

// --- JWT (ola 1: contrato `sub=correo`, `nombre=nombre_empresa`, exp 24h) ---
// ponytail: HS256 con exp 24h es lo mínimo que cumple el contrato. El techo
// (refrescos, httpOnly cookies) está documentado en el decision log del plan.

#[derive(Debug, Deserialize, Serialize)]
pub struct Claims {
    pub sub: String,
    pub nombre: String,
    pub exp: usize,
}

/// Sesión de una empresa autenticada con JWT Bearer.
#[derive(Debug, Clone)]
pub struct EmpresaSession {
    pub correo: String,
    // Superficie de contrato (brief ola 1: el extractor "expone correo/nombre");
    // ningún handler lo lee todavía, pero el claim está en el JWT.
    #[allow(dead_code)]
    pub nombre: String,
}

/// Lee `JWT_SECRET` desde env y lo inicializa UNA vez. `main()` la llama al
/// arrancar para fallar con mensaje claro si falta; con `get_or_init` ya no
/// puede fallar en tiempo de request.
pub fn jwt_secret() -> &'static str {
    static SECRET: OnceLock<String> = OnceLock::new();
    SECRET.get_or_init(|| {
        let secreto = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            panic!(
                "JWT_SECRET no está definida: añádela a backend/.env (ver .env.example). \
                 Sin secreto JWT el backend no puede firmar tokens."
            )
        });
        if secreto.trim().is_empty() {
            panic!("JWT_SECRET está vacía: define un valor no vacío en backend/.env (ver .env.example).");
        }
        secreto
    })
}

/// Emite un JWT HS256 válido por 24h con `sub=correo` y `nombre=nombre_empresa`.
pub fn emite_jwt(correo: &str, nombre: &str, secreto: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims {
        sub: correo.to_string(),
        nombre: nombre.to_string(),
        exp: (Utc::now() + Duration::hours(24)).timestamp() as usize,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secreto.as_bytes()))
}

/// Valida firma (HS256), expiración y estructura; devuelve los claims si es válido.
pub fn validar_jwt(token: &str, secreto: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secreto.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
}

fn rechazo_401() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "status": "error",
            "message": "No autorizado: token JWT ausente, inválido o expirado"
        })),
    )
}

#[async_trait]
impl<S> FromRequestParts<S> for EmpresaSession
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(rechazo_401)?;
        let token = header.strip_prefix("Bearer ").ok_or_else(rechazo_401)?;
        let claims = validar_jwt(token, jwt_secret()).map_err(|_| rechazo_401())?;
        Ok(EmpresaSession {
            correo: claims.sub,
            nombre: claims.nombre,
        })
    }
}

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
                match emite_jwt(&empresa.correo, &empresa.nombre_empresa, jwt_secret()) {
                    Ok(token) => Json(serde_json::json!({
                        "status": "success",
                        "empresa": empresa.nombre_empresa,
                        "token": token
                    })),
                    Err(e) => {
                        eprintln!("🚨 ERROR JWT: {:?}", e);
                        Json(serde_json::json!({
                            "status": "error",
                            "message": "Error al generar el token de sesión"
                        }))
                    }
                }
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

/// Parsea la env `ALLOWED_ORIGINS` (separada por comas): recorta espacios y
/// descarta entradas vacías. Función PURA, testeada.
fn parsear_origenes(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|o| !o.is_empty())
        .map(str::to_string)
        .collect()
}

pub(crate) fn cors_layer() -> CorsLayer {
    // Ola 6: orígenes desde `ALLOWED_ORIGINS` (separada por comas). Default dev
    // (frontend `dx serve` en :8080) si la env falta o queda vacía: sin cambio
    // de comportamiento en dev. Un origen con caracteres inválidos para un
    // HeaderValue (p. ej. saltos de línea) se descarta, no rompe el arranque.
    let lista = std::env::var("ALLOWED_ORIGINS")
        .ok()
        .map(|v| parsear_origenes(&v))
        .unwrap_or_default();
    let origenes = if lista.is_empty() {
        parsear_origenes("http://localhost:8080,http://127.0.0.1:8080")
    } else {
        lista
    };
    let permitidos: Vec<HeaderValue> = origenes
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();
    CorsLayer::new()
        .allow_origin(permitidos)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRETO_TEST: &str = "secreto-de-prueba-no-usar-en-produccion";

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

    #[test]
    fn jwt_roundtrip_emite_y_valida() {
        let token = emite_jwt("demo@pymza.mx", "Ferretería El Tornillo", SECRETO_TEST).unwrap();
        let claims = validar_jwt(&token, SECRETO_TEST).expect("JWT emitido debe validarse");
        assert_eq!(claims.sub, "demo@pymza.mx");
        assert_eq!(claims.nombre, "Ferretería El Tornillo");
        assert!(claims.exp > Utc::now().timestamp() as usize, "exp debe estar en el futuro");
        assert!(claims.exp <= (Utc::now() + Duration::hours(25)).timestamp() as usize, "exp ≈ 24h");
    }

    #[test]
    fn jwt_token_expirado_rechazado() {
        let expirado = Claims {
            sub: "demo@pymza.mx".into(),
            nombre: "Ferretería El Tornillo".into(),
            exp: (Utc::now() - Duration::hours(2)).timestamp() as usize,
        };
        let token = encode(
            &Header::default(),
            &expirado,
            &EncodingKey::from_secret(SECRETO_TEST.as_bytes()),
        )
        .unwrap();
        assert!(validar_jwt(&token, SECRETO_TEST).is_err());
    }

    #[test]
    fn jwt_token_malformado_rechazado() {
        assert!(validar_jwt("", SECRETO_TEST).is_err());
        assert!(validar_jwt("no-es-un-jwt", SECRETO_TEST).is_err());
        assert!(validar_jwt("a.b", SECRETO_TEST).is_err());
        // Firma manipulada: partes de header/payload válidas con firma corrupta.
        let valido = emite_jwt("demo@pymza.mx", "Ferretería El Tornillo", SECRETO_TEST).unwrap();
        let partes: Vec<&str> = valido.split('.').collect();
        let corrupto = format!("{}.{}.firma-rota", partes[0], partes[1]);
        assert!(validar_jwt(&corrupto, SECRETO_TEST).is_err());
    }

    #[test]
    fn jwt_token_con_otro_secreto_rechazado() {
        let token =
            emite_jwt("demo@pymza.mx", "Ferretería El Tornillo", "otro-secreto-distinto").unwrap();
        assert!(validar_jwt(&token, SECRETO_TEST).is_err());
    }

    #[test]
    fn parsear_origenes_separados_por_comas() {
        assert_eq!(parsear_origenes(""), Vec::<String>::new());
        assert_eq!(parsear_origenes("   "), Vec::<String>::new());
        assert_eq!(parsear_origenes(","), Vec::<String>::new());
        assert_eq!(
            parsear_origenes("http://localhost:8080,http://127.0.0.1:8080"),
            vec!["http://localhost:8080", "http://127.0.0.1:8080"]
        );
        // con espacios y vacíos intermedios
        assert_eq!(
            parsear_origenes(" https://mipyme.mx , , https://demo.pymza.mx,"),
            vec!["https://mipyme.mx", "https://demo.pymza.mx"]
        );
        // un solo origen
        assert_eq!(
            parsear_origenes("https://pymza.mx"),
            vec!["https://pymza.mx"]
        );
    }
}