//! OTP de verificación de teléfono (ola 3): código de 6 dígitos, hash SHA-256
//! para guardar en DB y envío por el `OtpSender` activo (mock en dev que
//! imprime el código en el log; WhatsApp Cloud API de Meta en producción).

use std::sync::{Arc, OnceLock};

use rand::Rng;
use sha2::{Digest, Sha256};

/// Código OTP de 6 dígitos con ceros a la izquierda ("000123").
pub fn generar_codigo() -> String {
    let n: u32 = rand::thread_rng().gen_range(0..1_000_000);
    format!("{:06}", n)
}

/// SHA-256 hex (minúsculas). La colección `verificaciones` guarda SOLO este
/// hash, nunca el código en claro.
pub fn hash_codigo(codigo: &str) -> String {
    Sha256::digest(codigo.as_bytes())
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

/// Canal de envío del código.
#[axum::async_trait]
pub trait OtpSender: Send + Sync {
    async fn enviar(&self, telefono: &str, codigo: &str);
}

/// Default en dev: el código queda en el log del backend.
pub struct MockOtpSender;

#[axum::async_trait]
impl OtpSender for MockOtpSender {
    async fn enviar(&self, telefono: &str, codigo: &str) {
        eprintln!("OTP MOCK para {telefono}: {codigo}");
    }
}

/// WhatsApp Cloud API (Meta). Solo se construye si existen las dos env.
pub struct WhatsAppOtpSender {
    token: String,
    phone_number_id: String,
}

impl WhatsAppOtpSender {
    pub fn desde_env() -> Option<Self> {
        Some(Self {
            token: env_no_vacia("WHATSAPP_TOKEN")?,
            phone_number_id: env_no_vacia("WHATSAPP_PHONE_NUMBER_ID")?,
        })
    }
}

fn env_no_vacia(key: &str) -> Option<String> {
    // Un placeholder vacío (p. ej. copiado de .env.example) no activa WhatsApp.
    std::env::var(key).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

#[axum::async_trait]
impl OtpSender for WhatsAppOtpSender {
    async fn enviar(&self, telefono: &str, codigo: &str) {
        // ponytail: Client por llamada — el volumen de OTP es ínfimo; techo:
        // reutilizar un reqwest::Client si el envío escala a miles/minuto.
        let res = reqwest::Client::new()
            .post(&format!(
                "https://graph.facebook.com/v21.0/{}/messages",
                self.phone_number_id
            ))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({
                "messaging_product": "whatsapp",
                "to": telefono,
                "type": "text",
                "text": { "body": format!("Tu código de verificación PYMZA es: {codigo}") }
            }))
            .send()
            .await;
        // Nunca panickea: si WhatsApp falla, el backend sigue y el usuario
        // puede pedir otro código.
        match res {
            Ok(r) if r.status().is_success() => {}
            Ok(r) => eprintln!("🚨 WhatsApp OTP falló (HTTP {})", r.status()),
            Err(e) => eprintln!("🚨 WhatsApp OTP falló: {e}"),
        }
    }
}

/// OtpSender activo del proceso: WhatsApp si las dos env `WHATSAPP_*` existen
/// y no están vacías; mock si no. Se construye UNA vez (mismo patrón que
/// `jwt_secret` en auth.rs).
// ponytail: global OnceLock en vez de ampliar el State del Router a un
// AppState — evita tocar el wiring de las 10 rutas existentes; techo: mover a
// AppState si hace falta inyectar el sender en tests multi-config.
pub fn sender_activo() -> Arc<dyn OtpSender + Send + Sync> {
    static SENDER: OnceLock<Arc<dyn OtpSender + Send + Sync>> = OnceLock::new();
    SENDER
        .get_or_init(|| match WhatsAppOtpSender::desde_env() {
            Some(s) => Arc::new(s),
            None => Arc::new(MockOtpSender),
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generar_codigo_produce_seis_digitos_con_ceros() {
        for _ in 0..500 {
            let codigo = generar_codigo();
            assert_eq!(codigo.len(), 6, "{codigo} debe tener 6 caracteres");
            assert!(codigo.chars().all(|d| d.is_ascii_digit()), "{codigo} debe ser numérico");
            codigo.parse::<u32>().expect("debe parsear como número");
        }
        // Con el rango 0..1_000_000 algún código pequeño aparece: si el
        // formato perdiera los ceros a la izquierda, estos asserts lo atrapan.
        assert!(generar_codigo().starts_with(|c: char| c.is_ascii_digit()));
    }

    #[test]
    fn hash_codigo_es_sha256_hex_minisculas_determinista() {
        let h = hash_codigo("123456");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(h.chars().all(|c| !c.is_ascii_uppercase()), "hex en minúsculas");
        // Vector conocido: SHA-256 de "123456"
        assert_eq!(
            h,
            "8d969eef6ecad3c29a3a629280e686cf0c3f5d5a86aff3ca12020c923adc6c92"
        );
        assert_eq!(hash_codigo("123456"), h, "determinista");
        assert_ne!(hash_codigo("654321"), h);
    }
}
