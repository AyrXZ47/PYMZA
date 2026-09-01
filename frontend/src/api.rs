//! Cliente HTTP compartido, formas de datos y sesión persistente.
//!
//! Contrato API ola 1 (`.workflow/plan.md`): el token es un JWT real; las rutas
//! `GET /api/creditos` y `GET /api/dashboard` ya no llevan `/{empresa}`, y los
//! bodies de `autorizar`/`reportar` ya no incluyen `empresa` (sale del token).

use dioxus::prelude::*;
use reqwest::StatusCode;
use std::sync::OnceLock;

/// URL del backend: inyectable en build/deploy (`API_BASE=... cargo build`),
/// default dev local. Railway inyectará la URL real en la ola 4.
pub const API_BASE: &str = match option_env!("API_BASE") {
    Some(url) => url,
    None => "http://127.0.0.1:3000",
};
/// Clave de localStorage donde vive el token de sesión (solo se usa en wasm).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub const TOKEN_STORAGE_KEY: &str = "pymza_token";
/// Clave de localStorage de la preferencia de tema (solo se usa en wasm).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub const THEME_STORAGE_KEY: &str = "pymza_theme";

pub fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| reqwest::Client::new())
}

// ponytail: el token persiste en localStorage (resuelve el techo de perder la
// sesión al recargar). Techo conocido: sin refresh tokens ni cookies httpOnly;
// el día que el backend emita cookies httpOnly se migra desde el servidor.
pub fn authed_request(method: reqwest::Method, path: String, token: &str) -> reqwest::RequestBuilder {
    http_client()
        .request(method, format!("{API_BASE}{path}"))
        .bearer_auth(token)
}

/// La sesión solo muere con un 401 (token ausente/inválido/expirado).
fn es_401(status: StatusCode) -> bool {
    status == StatusCode::UNAUTHORIZED
}

/// Marca logout y limpia el token ante un 401; devuelve false en ese caso.
pub fn sesion_ok(
    res: &reqwest::Response,
    mut is_authenticated: Signal<bool>,
    mut token: Signal<String>,
) -> bool {
    if es_401(res.status()) {
        is_authenticated.set(false);
        token.set(String::new());
        return false;
    }
    true
}

/// Decide si una respuesta de `GET /api/dashboard` revalida una sesión guardada
/// en localStorage. `Some(empresa)` si el token sigue válido; `None` si no hay
/// sesión restaurable.
pub fn evaluar_restauracion(status: StatusCode, data: &serde_json::Value) -> Option<String> {
    if es_401(status) {
        return None;
    }
    if data["status"] == "success" {
        let empresa = data["stats"]["empresa"].as_str().unwrap_or("").to_string();
        if empresa.is_empty() {
            None
        } else {
            Some(empresa)
        }
    } else {
        None
    }
}

// --- localStorage vía document::eval (solo web). En host son no-ops:
// fallback silencioso a sesión en memoria. ---

fn js_set_item(key: &str, value: &str) -> String {
    format!(
        "localStorage.setItem({}, {});",
        serde_json::to_string(key).unwrap_or_default(),
        serde_json::to_string(value).unwrap_or_default()
    )
}

fn js_get_item(key: &str) -> String {
    format!(
        "return localStorage.getItem({}) ?? '';",
        serde_json::to_string(key).unwrap_or_default()
    )
}

fn js_remove_item(key: &str) -> String {
    format!(
        "localStorage.removeItem({});",
        serde_json::to_string(key).unwrap_or_default()
    )
}

/// Persiste el token tras login/registro (ignora errores: fallback a memoria).
pub fn token_guardar(token: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = dioxus::document::eval(&js_set_item(TOKEN_STORAGE_KEY, token));
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = token;
}

/// Borra el token guardado al hacer logout.
pub fn token_borrar() {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = dioxus::document::eval(&js_remove_item(TOKEN_STORAGE_KEY));
    }
}

/// Lee el token guardado, si existe y no está vacío.
pub async fn token_leer() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let eval = dioxus::document::eval(&js_get_item(TOKEN_STORAGE_KEY));
        let valor = eval.await.ok()?.as_str()?.to_string();
        if valor.is_empty() {
            None
        } else {
            Some(valor)
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

// --- Tema claro/oscuro: clase `dark` en <html> + localStorage. En host son
// no-ops (fallback a dark, que es el default del look actual). ---

/// Devuelve el tema opuesto. Lógica pura (testeable en host): cualquier valor
/// que no sea "light" cuenta como dark (default robusto).
pub fn theme_invertir(actual: &str) -> &'static str {
    if actual == "light" {
        "dark"
    } else {
        "light"
    }
}

/// JS que activa/desactiva la clase `dark` en el elemento raíz.
fn theme_class_js(theme: &str) -> String {
    format!(
        "document.documentElement.classList.toggle('dark', {});",
        theme == "dark"
    )
}

/// Aplica el tema al documento (no-op en host).
pub fn theme_aplicar(theme: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = dioxus::document::eval(&theme_class_js(theme));
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = theme;
}

/// Persiste la preferencia de tema en localStorage (no-op en host).
pub fn theme_guardar(theme: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = dioxus::document::eval(&js_set_item(THEME_STORAGE_KEY, theme));
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = theme;
}

/// Lee la preferencia de tema guardada (solo wasm; en host devuelve None).
pub async fn theme_leer() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let eval = dioxus::document::eval(&js_get_item(THEME_STORAGE_KEY));
        let valor = eval.await.ok()?.as_str()?.to_string();
        if valor.is_empty() {
            None
        } else {
            Some(valor)
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

/// Estadísticas del dashboard devueltas por `GET /api/dashboard` (`stats`).
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DashboardStats {
    pub empresa: String,
    pub creditos_activos: i32,
    pub capital_prestado: f64,
    pub proximos_cobros: i32,
}

/// Fila del plan de pagos devuelta por `POST /api/creditos/evaluar`.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PagoInfo {
    pub mes: i32,
    pub pago: f64,
    pub interes: f64,
    pub capital: f64,
    pub saldo_restante: f64,
}

// --- Verificación de teléfono por OTP (contrato API ola 3). ---

/// Request a `POST /api/verificaciones/solicitar`: pide un código de 6 dígitos
/// para `{ curp, telefono }` (WhatsApp; en dev el backend lo imprime en su log).
pub fn solicitar_verificacion(curp: &str, telefono: &str, token: &str) -> reqwest::RequestBuilder {
    authed_request(
        reqwest::Method::POST,
        "/api/verificaciones/solicitar".to_string(),
        token,
    )
    .json(&serde_json::json!({ "curp": curp, "telefono": telefono }))
}

/// Request a `POST /api/verificaciones/confirmar`: valida `codigo` contra el
/// desafío vigente; el backend marca `telefono_verificado = true`.
pub fn confirmar_verificacion(
    curp: &str,
    telefono: &str,
    codigo: &str,
    token: &str,
) -> reqwest::RequestBuilder {
    authed_request(
        reqwest::Method::POST,
        "/api/verificaciones/confirmar".to_string(),
        token,
    )
    .json(&serde_json::json!({ "curp": curp, "telefono": telefono, "codigo": codigo }))
}

/// ¿El cliente ya tiene el teléfono verificado? Clientes anteriores a la ola 3
/// no traen el campo → no verificado.
pub fn telefono_verificado(cliente: &serde_json::Value) -> bool {
    cliente
        .get("telefono_verificado")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Extrae la alerta de morosidad de un cliente de la red (motivo, empresa).
pub fn alerta_info(cliente: &serde_json::Value) -> Option<(String, String)> {
    let alerta = cliente.get("alerta")?.as_object()?;
    let motivo = alerta
        .get("motivo")
        .and_then(|m| m.as_str())
        .unwrap_or("Morosidad reportada por otra empresa");
    let empresa = alerta
        .get("empresa")
        .and_then(|e| e.as_str())
        .unwrap_or("Empresa no identificada");
    Some((motivo.to_string(), empresa.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authed_request_incluye_header_authorization_bearer() {
        let request = authed_request(reqwest::Method::GET, "/api/dashboard".to_string(), "token-prueba-abc")
            .build()
            .unwrap();
        assert_eq!(
            request
                .headers()
                .get(reqwest::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok()),
            Some("Bearer token-prueba-abc")
        );
    }

    #[test]
    fn evaluar_restauracion_ok_devuelve_empresa() {
        let data = serde_json::json!({ "status": "success", "stats": { "empresa": "Abarrotes Don Pepe" } });
        assert_eq!(
            evaluar_restauracion(StatusCode::OK, &data).as_deref(),
            Some("Abarrotes Don Pepe")
        );
    }

    #[test]
    fn evaluar_restauracion_401_es_none() {
        assert_eq!(
            evaluar_restauracion(StatusCode::UNAUTHORIZED, &serde_json::json!({})),
            None
        );
    }

    #[test]
    fn evaluar_restauracion_success_sin_empresa_es_none() {
        let data = serde_json::json!({ "status": "success", "stats": {} });
        assert_eq!(evaluar_restauracion(StatusCode::OK, &data), None);
    }

    #[test]
    fn js_storage_escapa_comillas_e_inyecta_return() {
        assert_eq!(js_set_item("k", "va\"l"), r#"localStorage.setItem("k", "va\"l");"#);
        assert_eq!(js_get_item("k"), r#"return localStorage.getItem("k") ?? '';"#);
        assert_eq!(js_remove_item("k"), r#"localStorage.removeItem("k");"#);
    }

    #[test]
    fn alerta_info_con_alerta_devuelve_motivo_y_empresa() {
        let cliente = serde_json::json!({
            "curp": "GACM940101HDFRRR07",
            "alerta": { "empresa": "Abarrotes Don Pepe", "motivo": "2 pagos vencidos" }
        });
        assert_eq!(
            alerta_info(&cliente),
            Some(("2 pagos vencidos".to_string(), "Abarrotes Don Pepe".to_string()))
        );
    }

    #[test]
    fn alerta_info_sin_alerta_devuelve_none() {
        let cliente = serde_json::json!({ "curp": "GACM940101HDFRRR07" });
        assert_eq!(alerta_info(&cliente), None);
    }

    #[test]
    fn alerta_info_nula_devuelve_none() {
        let cliente = serde_json::json!({ "curp": "GACM940101HDFRRR07", "alerta": null });
        assert_eq!(alerta_info(&cliente), None);
    }

    #[test]
    fn theme_invertir_alterna_light_y_dark() {
        assert_eq!(theme_invertir("dark"), "light");
        assert_eq!(theme_invertir("light"), "dark");
    }

    #[test]
    fn theme_invertir_desconocido_cae_a_light() {
        assert_eq!(theme_invertir("cualquier-cosa"), "light");
    }

    #[test]
    fn theme_class_js_activa_dark_solo_con_dark() {
        assert_eq!(
            theme_class_js("dark"),
            "document.documentElement.classList.toggle('dark', true);"
        );
        assert_eq!(
            theme_class_js("light"),
            "document.documentElement.classList.toggle('dark', false);"
        );
    }

    #[test]
    fn solicitar_verificacion_construye_post_con_curp_y_telefono() {
        let request = solicitar_verificacion("GACM940101HDFRRR07", "5512345678", "tok")
            .build()
            .unwrap();
        assert_eq!(*request.method(), reqwest::Method::POST);
        assert_eq!(request.url().path(), "/api/verificaciones/solicitar");
        let body: serde_json::Value =
            serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
        assert_eq!(
            body,
            serde_json::json!({ "curp": "GACM940101HDFRRR07", "telefono": "5512345678" })
        );
        assert!(body.get("codigo").is_none());
    }

    #[test]
    fn confirmar_verificacion_incluye_codigo_en_el_body() {
        let request = confirmar_verificacion("GACM940101HDFRRR07", "5512345678", "654321", "tok")
            .build()
            .unwrap();
        assert_eq!(request.url().path(), "/api/verificaciones/confirmar");
        let body: serde_json::Value =
            serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
        assert_eq!(body["codigo"], "654321");
    }

    #[test]
    fn telefono_verificado_true_solo_con_el_campo_en_true() {
        assert!(telefono_verificado(&serde_json::json!({ "telefono_verificado": true })));
        assert!(!telefono_verificado(&serde_json::json!({ "telefono_verificado": false })));
        // cliente de antes de la ola 3: sin el campo → no verificado
        assert!(!telefono_verificado(&serde_json::json!({ "curp": "GACM940101HDFRRR07" })));
    }
}