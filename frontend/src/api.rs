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

// --- Contrato API ola 4: resumen de cartera (GET /api/creditos/resumen) y
// registro de pagos (POST /api/creditos/pagos). Shape exacta en plan.md. ---

/// Mes de `cobrado_vs_por_cobrar`: pagos registrados vs cuotas esperadas.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct ResumenMes {
    pub mes: String,
    pub cobrado: f64,
    pub por_cobrar: f64,
}

/// Horizonte de `flujo_proyectado` (30/60/90 días).
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct ResumenHorizonte {
    pub horizonte: i64,
    pub monto: f64,
}

/// Bucket de `aging`: saldo vencido por antigüedad.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct ResumenBucket {
    pub bucket: String,
    pub monto: f64,
}

/// Deudor de `top_deudores` (saldo = total a pagar − pagos registrados).
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct ResumenDeudor {
    pub cliente_curp: String,
    pub nombre: String,
    pub saldo: f64,
}

/// Bucket de `distribucion_montos`: n de planes por monto_total.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct ResumenBucketN {
    pub bucket: String,
    pub n: i64,
}

/// Resumen de cartera del tenant (fuente de las 6 gráficas del dashboard).
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct Resumen {
    pub cobrado_vs_por_cobrar: Vec<ResumenMes>,
    pub tasa_morosidad: f64,
    pub flujo_proyectado: Vec<ResumenHorizonte>,
    pub aging: Vec<ResumenBucket>,
    pub top_deudores: Vec<ResumenDeudor>,
    pub distribucion_montos: Vec<ResumenBucketN>,
}

/// Parseo puro del body de `GET /api/creditos/resumen` (testeable en host).
/// Tolerante: campos faltantes → vacíos (el dashboard muestra "Sin datos aún").
pub fn parsear_resumen(data: &serde_json::Value) -> Option<Resumen> {
    if data["status"] == "success" {
        serde_json::from_value(data["resumen"].clone()).ok()
    } else {
        None
    }
}

/// Descarga el resumen de cartera del tenant. Ante un 401 mata la sesión y
/// devuelve error (el dashboard lo muestra; el logout lo maneja `sesion_ok`).
pub async fn obtener_resumen(
    token: &str,
    is_authenticated: Signal<bool>,
    token_sig: Signal<String>,
) -> Result<Resumen, String> {
    let res = authed_request(reqwest::Method::GET, "/api/creditos/resumen".to_string(), token)
        .send()
        .await
        .map_err(|e| format!("Sin conexión con el servidor: {e}"))?;
    if !sesion_ok(&res, is_authenticated, token_sig) {
        return Err("Sesión expirada".to_string());
    }
    let data: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Respuesta inválida del servidor: {e}"))?;
    parsear_resumen(&data).ok_or_else(|| "No se pudo leer el resumen de cartera".to_string())
}

/// Request a `POST /api/creditos/pagos`: registra el pago de la `cuota` de un
/// plan con `monto` (debe ser el `pago_mensual` del plan, ±1 centavo).
pub fn registrar_pago(plan_id: &str, cuota: i64, monto: f64, token: &str) -> reqwest::RequestBuilder {
    authed_request(reqwest::Method::POST, "/api/creditos/pagos".to_string(), token)
        .json(&serde_json::json!({ "plan_id": plan_id, "cuota": cuota, "monto": monto }))
}

/// Siguiente cuota impaga de un plan: `Some(n)` si existe una cuota después de
/// las ya pagadas; `None` si el plan está liquidado (o datos raros).
pub fn siguiente_cuota_impaga(plazo_meses: i64, cuotas_pagadas: i64) -> Option<i64> {
    let siguiente = cuotas_pagadas.max(0) + 1;
    (siguiente <= plazo_meses).then_some(siguiente)
}


// --- Contrato PDF (contrato API ola 6): GET /api/creditos/{plan_id}/contrato. ---

/// Request a `GET /api/creditos/{plan_id}/contrato`: devuelve el PDF del plan
/// (bytes, `Content-Type: application/pdf`) con su `Content-Disposition`.
pub fn contrato_request(plan_id: &str, token: &str) -> reqwest::RequestBuilder {
    authed_request(
        reqwest::Method::GET,
        format!("/api/creditos/{plan_id}/contrato"),
        token,
    )
}

/// Nombre de archivo del contrato (puro, testeable en host): usa el `filename`
/// del header `Content-Disposition` si viene; si no, `contrato-<curp>.pdf`.
pub fn nombre_archivo_contrato(content_disposition: Option<&str>, curp: &str) -> String {
    content_disposition
        .and_then(|h| {
            h.split(';')
                .map(str::trim)
                .find_map(|p| p.strip_prefix("filename="))
                .map(|v| v.trim_matches('"').to_string())
        })
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("contrato-{curp}.pdf"))
}

/// Descarga el PDF del contrato del plan. Ante un 401 mata la sesión (vía
/// `sesion_ok`); devuelve `(bytes, nombre de archivo)` listos para descargar.
pub async fn descargar_contrato(
    plan_id: &str,
    curp: &str,
    token: &str,
    is_authenticated: Signal<bool>,
    token_sig: Signal<String>,
) -> Result<(Vec<u8>, String), String> {
    let res = contrato_request(plan_id, token)
        .send()
        .await
        .map_err(|e| format!("Sin conexión con el servidor: {e}"))?;
    if !sesion_ok(&res, is_authenticated, token_sig) {
        return Err("Sesión expirada".to_string());
    }
    if !res.status().is_success() {
        return Err(format!(
            "No se pudo descargar el contrato (HTTP {})",
            res.status()
        ));
    }
    // Headers primero: `bytes()` consume la respuesta.
    let header = res
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let bytes = res
        .bytes()
        .await
        .map_err(|_| "No se pudo leer el contrato".to_string())?
        .to_vec();
    Ok((bytes, nombre_archivo_contrato(header.as_deref(), curp)))
}

/// JS que dispara la descarga de un PDF desde sus bytes base64: Blob →
/// object URL → `<a download>` click → limpia el object URL. El nombre va
/// como string JSON (mismo escaping que los items de storage).
fn js_descarga(b64: &str, nombre: &str) -> String {
    format!(
        "const bin = atob('{}'); const bytes = new Uint8Array(bin.length); \
         for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i); \
         const url = URL.createObjectURL(new Blob([bytes], {{ type: 'application/pdf' }})); \
         const a = document.createElement('a'); a.href = url; a.download = {}; \
         document.body.appendChild(a); a.click(); a.remove(); URL.revokeObjectURL(url);",
        b64,
        serde_json::to_string(nombre).unwrap_or_default()
    )
}

/// Dispara la descarga del archivo en el navegador (no-op en host).
// ponytail: los bytes van base64 dentro de un eval porque web-sys/js-sys no
// son deps directas y no se pueden añadir (cero deps nuevas). Techo: PDFs de
// ~1 MB+ pagarán el +33% del base64 en un string JS; si crece, migrar este
// único helper a web_sys::Blob directo.
pub fn descargar_archivo(bytes: &[u8], nombre: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = dioxus::document::eval(&js_descarga(&archivo_a_b64(bytes), nombre));
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (bytes, nombre);
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

// --- Verificación de INE (KYC) y score por recibos (contrato API ola 5). ---
// Subida = base64 en JSON (no multipart); el mismo mímee/tamaño que valida el
// backend se valida client-side para no mandar algo que ya sabemos que falla.

/// Límite de tamaño del contrato ola 5 (2 MB).
pub const TAM_MAX_ARCHIVO: u64 = 2 * 1024 * 1024;

/// Mimes de imagen que aceptan kyc y recibos (igual que el backend).
pub const MIMES_IMAGEN: [&str; 3] = ["image/png", "image/jpeg", "image/webp"];

/// Codifica bytes en base64 estándar (alfabeto con relleno, como `base64 -w0`).
pub fn archivo_a_b64(bytes: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Lee los bytes de un archivo seleccionado (en wasm: FileReader async de
/// dioxus-web) y los codifica base64. Los errores de lectura regresan como
/// texto listo para la UI.
pub async fn archivo_a_b64_de(file: &dioxus_elements::FileData) -> Result<String, String> {
    let bytes = file
        .read_bytes()
        .await
        .map_err(|_| "No se pudo leer el archivo".to_string())?;
    Ok(archivo_a_b64(&bytes))
}

/// Validación client-side de un archivo antes de enviarlo: `Some(motivo)` si
/// hay que rechazarlo, `None` si pasa (mismo orden de reglas que el backend:
/// mime primero, tamaño después).
pub fn validar_archivo(tamano: u64, mime: Option<&str>) -> Option<String> {
    match mime {
        Some(m) if MIMES_IMAGEN.contains(&m) => {}
        _ => return Some("Solo se aceptan imágenes PNG, JPEG o WebP".to_string()),
    }
    (tamano > TAM_MAX_ARCHIVO).then(|| "El archivo supera el límite de 2 MB".to_string())
}

/// ¿El cliente ya tiene la INE verificada? Clientes anteriores a la ola 5 no
/// traen el campo → no verificada (mismo default que `telefono_verificado`).
pub fn ine_verificada(cliente: &serde_json::Value) -> bool {
    cliente
        .get("ine_verificada")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Request a `POST /api/clientes/{curp}/kyc`: envía la INE en base64; el
/// backend corre OCR y responde si la CURP del documento coincide.
pub fn kyc_verificar(
    curp: &str,
    archivo_b64: &str,
    mime: &str,
    token: &str,
) -> reqwest::RequestBuilder {
    authed_request(
        reqwest::Method::POST,
        format!("/api/clientes/{curp}/kyc"),
        token,
    )
    .json(&serde_json::json!({ "archivo_b64": archivo_b64, "mime": mime }))
}

/// Request a `POST /api/clientes/{curp}/recibos`: sube un recibo de servicios
/// (`tipo`: "luz" | "agua" | "telefono") para el bonus de score (+25, máx 2).
pub fn recibo_subir(
    curp: &str,
    archivo_b64: &str,
    mime: &str,
    tipo: &str,
    token: &str,
) -> reqwest::RequestBuilder {
    authed_request(
        reqwest::Method::POST,
        format!("/api/clientes/{curp}/recibos"),
        token,
    )
    .json(&serde_json::json!({
        "archivo_b64": archivo_b64,
        "mime": mime,
        "tipo": tipo
    }))
}

/// Respuesta de `POST /api/clientes/:curp/kyc` (contrato ola 5).
/// `#[serde(default)]` tolera respuestas parciales del backend.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct KycResultado {
    pub curp_ine: Option<String>,
    pub nombre_ine: Option<String>,
    pub coincide: bool,
    pub ine_verificada: bool,
}

/// Respuesta de `POST /api/clientes/:curp/recibos` (contrato ola 5).
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct ReciboResultado {
    pub monto_leido: Option<f64>,
    pub score: i64,
    pub nivel_riesgo: String,
    pub recibos_contados: i64,
}

/// Parseo puro del body de kyc (testeable en host): solo `status: "success"`.
pub fn parsear_kyc(data: &serde_json::Value) -> Option<KycResultado> {
    if data["status"] == "success" {
        serde_json::from_value(data.clone()).ok()
    } else {
        None
    }
}

/// Parseo puro del body de recibos (testeable en host).
pub fn parsear_recibo(data: &serde_json::Value) -> Option<ReciboResultado> {
    if data["status"] == "success" {
        serde_json::from_value(data.clone()).ok()
    } else {
        None
    }
}

/// Semáforo del panel KYC (función pura, testeable): color ("green"/"amber",
/// como `semaforo_morosidad`) + mensaje según el resultado del backend y la
/// CURP capturada.
pub fn mensaje_kyc(res: &KycResultado, curp_capturada: &str) -> (&'static str, String) {
    if res.coincide {
        let leida = res.curp_ine.as_deref().unwrap_or(curp_capturada);
        ("green", format!("✓ INE verificada — CURP leída: {leida}"))
    } else if let Some(curp_ine) = res.curp_ine.as_deref() {
        (
            "amber",
            format!("La CURP de la INE ({curp_ine}) no coincide con la capturada ({curp_capturada})"),
        )
    } else {
        (
            "amber",
            "No se pudo leer la CURP del documento, prueba otra foto".to_string(),
        )
    }
}

/// Mensaje de resultado de un recibo subido: score nuevo, nivel de riesgo y
/// conteo; con nota del monto leído cuando el OCR lo encontró.
pub fn mensaje_recibo(res: &ReciboResultado) -> String {
    let base = format!(
        "Score: {} · Riesgo: {} · Recibos {}/2",
        res.score, res.nivel_riesgo, res.recibos_contados
    );
    match res.monto_leido {
        Some(monto) => format!("{base} · monto leído: {monto}"),
        None => base,
    }
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

    /// Shape EXACTA del contrato API ola 4 (plan.md § Contrato API ola 4).
    fn resumen_contrato() -> serde_json::Value {
        serde_json::json!({
            "status": "success",
            "resumen": {
                "cobrado_vs_por_cobrar": [
                    {"mes": "2026-04", "cobrado": 0.0, "por_cobrar": 0.0},
                    {"mes": "2026-05", "cobrado": 0.0, "por_cobrar": 0.0},
                    {"mes": "2026-06", "cobrado": 0.0, "por_cobrar": 0.0},
                    {"mes": "2026-07", "cobrado": 0.0, "por_cobrar": 0.0},
                    {"mes": "2026-08", "cobrado": 0.0, "por_cobrar": 0.0},
                    {"mes": "2026-09", "cobrado": 1200.0, "por_cobrar": 3600.0}
                ],
                "tasa_morosidad": 0.25,
                "flujo_proyectado": [
                    {"horizonte": 30, "monto": 1200.0},
                    {"horizonte": 60, "monto": 2400.0},
                    {"horizonte": 90, "monto": 3600.0}
                ],
                "aging": [
                    {"bucket": "0-30", "monto": 800.0},
                    {"bucket": "31-60", "monto": 400.0},
                    {"bucket": "61-90", "monto": 0.0},
                    {"bucket": "90+", "monto": 0.0}
                ],
                "top_deudores": [
                    {"cliente_curp": "GACM940101HDFRRR07", "nombre": "María Guadalupe", "saldo": 9600.0}
                ],
                "distribucion_montos": [
                    {"bucket": "0-1k", "n": 2},
                    {"bucket": "1k-5k", "n": 1},
                    {"bucket": "5k+", "n": 0}
                ]
            }
        })
    }

    #[test]
    fn parsear_resumen_con_shape_del_contrato() {
        let r = parsear_resumen(&resumen_contrato()).expect("el shape del contrato debe parsear");
        assert_eq!(r.cobrado_vs_por_cobrar.len(), 6);
        assert_eq!(r.cobrado_vs_por_cobrar[5].mes, "2026-09");
        assert_eq!(r.cobrado_vs_por_cobrar[5].cobrado, 1200.0);
        assert_eq!(r.cobrado_vs_por_cobrar[5].por_cobrar, 3600.0);
        // semáforo del contrato: 0.25 = >20% → rojo
        assert!(crate::components::charts::semaforo_morosidad(r.tasa_morosidad).contains("red"));
        assert_eq!(
            r.flujo_proyectado.iter().map(|f| f.horizonte).collect::<Vec<_>>(),
            [30, 60, 90]
        );
        assert_eq!(r.flujo_proyectado[1].monto, 2400.0);
        assert_eq!(r.aging.iter().map(|a| a.bucket.clone()).collect::<Vec<_>>(), ["0-30", "31-60", "61-90", "90+"]);
        assert_eq!(r.top_deudores[0].nombre, "María Guadalupe");
        assert_eq!(r.top_deudores[0].saldo, 9600.0);
        assert_eq!(r.distribucion_montos[2].n, 0);
    }

    #[test]
    fn parsear_resumen_status_error_es_none() {
        assert_eq!(parsear_resumen(&serde_json::json!({ "status": "error" })), None);
        assert_eq!(parsear_resumen(&serde_json::json!({})), None);
    }

    #[test]
    fn parsear_resumen_tolera_resumen_parcial() {
        let data = serde_json::json!({ "status": "success", "resumen": { "tasa_morosidad": 0.1 } });
        let r = parsear_resumen(&data).expect("resumen parcial debe parsear a vacíos");
        assert_eq!(r.tasa_morosidad, 0.1);
        assert!(r.cobrado_vs_por_cobrar.is_empty());
        assert!(r.top_deudores.is_empty());
        assert!(r.aging.is_empty());
    }

    #[test]
    fn registrar_pago_construye_post_con_plan_cuota_y_monto() {
        let request = registrar_pago("665f1a2b3c4d5e6f7a8b9c0d", 2, 1030.0, "tok").build().unwrap();
        assert_eq!(*request.method(), reqwest::Method::POST);
        assert_eq!(request.url().path(), "/api/creditos/pagos");
        let body: serde_json::Value =
            serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
        assert_eq!(
            body,
            serde_json::json!({ "plan_id": "665f1a2b3c4d5e6f7a8b9c0d", "cuota": 2, "monto": 1030.0 })
        );
    }

    #[test]
    fn siguiente_cuota_impaga_avanza_y_para_en_el_plazo() {
        assert_eq!(siguiente_cuota_impaga(12, 0), Some(1));
        assert_eq!(siguiente_cuota_impaga(12, 5), Some(6));
        assert_eq!(siguiente_cuota_impaga(12, 12), None, "liquidado");
        assert_eq!(siguiente_cuota_impaga(12, 15), None, "datos raros");
        assert_eq!(siguiente_cuota_impaga(0, 0), None);
        assert_eq!(siguiente_cuota_impaga(12, -3), Some(1), "pagadas negativas no rompen");
    }

    // --- Contrato API ola 5: KYC (INE) y score por recibos. ---

    #[test]
    fn archivo_a_b64_sigue_el_estandar_rfc4648() {
        assert_eq!(archivo_a_b64(b""), "");
        assert_eq!(archivo_a_b64(b"f"), "Zg==");
        assert_eq!(archivo_a_b64(b"fo"), "Zm8=");
        assert_eq!(archivo_a_b64(b"foo"), "Zm9v");
        assert_eq!(archivo_a_b64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn archivo_a_b64_codifica_binario_que_no_es_utf8() {
        assert_eq!(archivo_a_b64(&[0xff]), "/w==");
        assert_eq!(archivo_a_b64(&[0x00, 0xff, 0x10]), "AP8Q");
    }

    #[test]
    fn validar_archivo_acepta_imagenes_dentro_del_limite() {
        assert_eq!(validar_archivo(0, Some("image/png")), None);
        assert_eq!(validar_archivo(100, Some("image/jpeg")), None);
        assert_eq!(validar_archivo(100, Some("image/webp")), None);
        assert_eq!(
            validar_archivo(TAM_MAX_ARCHIVO, Some("image/png")),
            None,
            "2 MB exactos pasan (el contrato dice ≤ 2 MB)"
        );
    }

    #[test]
    fn validar_archivo_rechaza_mime_y_tamano_fuera_de_contrato() {
        assert_eq!(
            validar_archivo(0, None).as_deref(),
            Some("Solo se aceptan imágenes PNG, JPEG o WebP")
        );
        assert_eq!(
            validar_archivo(0, Some("application/pdf")).as_deref(),
            Some("Solo se aceptan imágenes PNG, JPEG o WebP")
        );
        assert_eq!(
            validar_archivo(TAM_MAX_ARCHIVO + 1, Some("image/png")).as_deref(),
            Some("El archivo supera el límite de 2 MB")
        );
    }

    #[test]
    fn ine_verificada_true_solo_con_el_campo_en_true() {
        assert!(ine_verificada(&serde_json::json!({ "ine_verificada": true })));
        assert!(!ine_verificada(&serde_json::json!({ "ine_verificada": false })));
        // cliente de antes de la ola 5: sin el campo → no verificada
        assert!(!ine_verificada(&serde_json::json!({ "curp": "GACM940101HDFRRR07" })));
    }

    #[test]
    fn kyc_verificar_construye_post_con_archivo_y_mime() {
        let request = kyc_verificar("GACM940101HDFRRR07", "QUJD", "image/png", "tok")
            .build()
            .unwrap();
        assert_eq!(*request.method(), reqwest::Method::POST);
        assert_eq!(request.url().path(), "/api/clientes/GACM940101HDFRRR07/kyc");
        let body: serde_json::Value =
            serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
        assert_eq!(
            body,
            serde_json::json!({ "archivo_b64": "QUJD", "mime": "image/png" })
        );
    }

    #[test]
    fn recibo_subir_construye_post_con_tipo() {
        let request = recibo_subir("GACM940101HDFRRR07", "QUJD", "image/jpeg", "luz", "tok")
            .build()
            .unwrap();
        assert_eq!(*request.method(), reqwest::Method::POST);
        assert_eq!(request.url().path(), "/api/clientes/GACM940101HDFRRR07/recibos");
        let body: serde_json::Value =
            serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
        assert_eq!(body["archivo_b64"], "QUJD");
        assert_eq!(body["mime"], "image/jpeg");
        assert_eq!(body["tipo"], "luz");
    }

    #[test]
    fn parsear_kyc_con_shape_del_contrato() {
        let data = serde_json::json!({
            "status": "success",
            "curp_ine": "GACM940101HDFRRR07",
            "nombre_ine": "MARIA GUADALUPE ACOSTA CARDENAS",
            "coincide": true,
            "ine_verificada": true
        });
        let kyc = parsear_kyc(&data).expect("el shape del contrato debe parsear");
        assert_eq!(kyc.curp_ine.as_deref(), Some("GACM940101HDFRRR07"));
        assert_eq!(
            kyc.nombre_ine.as_deref(),
            Some("MARIA GUADALUPE ACOSTA CARDENAS")
        );
        assert!(kyc.coincide);
        assert!(kyc.ine_verificada);
    }

    #[test]
    fn parsear_kyc_curp_no_legible_es_success_con_none() {
        let data = serde_json::json!({
            "status": "success",
            "curp_ine": null,
            "nombre_ine": null,
            "coincide": false,
            "ine_verificada": false
        });
        let kyc = parsear_kyc(&data).expect("curp null es una respuesta válida");
        assert_eq!(kyc.curp_ine, None);
        assert!(!kyc.coincide);
        assert!(!kyc.ine_verificada);
    }

    #[test]
    fn parsear_kyc_status_error_es_none() {
        assert_eq!(
            parsear_kyc(&serde_json::json!({
                "status": "error",
                "message": "OCR no disponible en este servidor"
            })),
            None
        );
    }

    #[test]
    fn parsear_recibo_con_shape_del_contrato() {
        let data = serde_json::json!({
            "status": "success",
            "monto_leido": 350.0,
            "score": 600,
            "nivel_riesgo": "Medio",
            "recibos_contados": 1
        });
        let r = parsear_recibo(&data).expect("el shape del contrato debe parsear");
        assert_eq!(r.monto_leido, Some(350.0));
        assert_eq!(r.score, 600);
        assert_eq!(r.nivel_riesgo, "Medio");
        assert_eq!(r.recibos_contados, 1);
    }

    #[test]
    fn parsear_recibo_monto_nulo_y_status_error() {
        let ok = parsear_recibo(&serde_json::json!({
            "status": "success",
            "monto_leido": null,
            "score": 550,
            "nivel_riesgo": "Medio",
            "recibos_contados": 2
        }));
        assert_eq!(ok.map(|r| r.monto_leido), Some(None), "recibo sin monto legible");
        assert_eq!(
            parsear_recibo(&serde_json::json!({
                "status": "error",
                "message": "Máximo 2 recibos por cliente"
            })),
            None
        );
    }

    #[test]
    fn mensaje_kyc_semaforo_de_tres_estados() {
        let coincide = KycResultado {
            curp_ine: Some("GACM940101HDFRRR07".into()),
            nombre_ine: None,
            coincide: true,
            ine_verificada: true,
        };
        let (color, msg) = mensaje_kyc(&coincide, "GACM940101HDFRRR07");
        assert_eq!(color, "green");
        assert_eq!(msg, "✓ INE verificada — CURP leída: GACM940101HDFRRR07");

        let distinta = KycResultado {
            curp_ine: Some("AAAA000000XXXXXX00".into()),
            nombre_ine: None,
            coincide: false,
            ine_verificada: false,
        };
        let (color, msg) = mensaje_kyc(&distinta, "GACM940101HDFRRR07");
        assert_eq!(color, "amber");
        assert_eq!(
            msg,
            "La CURP de la INE (AAAA000000XXXXXX00) no coincide con la capturada (GACM940101HDFRRR07)"
        );

        let ilegible = KycResultado {
            curp_ine: None,
            nombre_ine: None,
            coincide: false,
            ine_verificada: false,
        };
        let (color, msg) = mensaje_kyc(&ilegible, "GACM940101HDFRRR07");
        assert_eq!(color, "amber");
        assert_eq!(msg, "No se pudo leer la CURP del documento, prueba otra foto");
    }

    #[test]
    fn mensaje_recibo_incluye_score_nivel_y_conteo() {
        let completo = ReciboResultado {
            monto_leido: Some(350.0),
            score: 600,
            nivel_riesgo: "Medio".into(),
            recibos_contados: 1,
        };
        assert_eq!(
            mensaje_recibo(&completo),
            "Score: 600 · Riesgo: Medio · Recibos 1/2 · monto leído: 350"
        );
        let sin_monto = ReciboResultado {
            monto_leido: None,
            score: 575,
            nivel_riesgo: "Medio".into(),
            recibos_contados: 1,
        };
        assert_eq!(
            mensaje_recibo(&sin_monto),
            "Score: 575 · Riesgo: Medio · Recibos 1/2"
        );
    }

    // --- Contrato PDF (ola 6). ---

    #[test]
    fn contrato_request_construye_get_protegido_sin_body() {
        let request = contrato_request("665f1a2b3c4d5e6f7a8b9c0d", "tok").build().unwrap();
        assert_eq!(*request.method(), reqwest::Method::GET);
        assert_eq!(
            request.url().path(),
            "/api/creditos/665f1a2b3c4d5e6f7a8b9c0d/contrato"
        );
        assert_eq!(
            request
                .headers()
                .get(reqwest::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok()),
            Some("Bearer tok")
        );
        assert!(request.body().is_none(), "el GET de contrato no lleva body");
    }

    #[test]
    fn nombre_archivo_contrato_usa_el_filename_del_header() {
        assert_eq!(
            nombre_archivo_contrato(
                Some("attachment; filename=\"contrato-GACM940101HDFRRR07.pdf\""),
                "GACM940101HDFRRR07"
            ),
            "contrato-GACM940101HDFRRR07.pdf"
        );
        // filename sin comillas también parsea
        assert_eq!(
            nombre_archivo_contrato(Some("attachment; filename=contrato.pdf"), "X"),
            "contrato.pdf"
        );
    }

    #[test]
    fn nombre_archivo_contrato_sin_header_o_sin_filename_cae_a_curp() {
        assert_eq!(
            nombre_archivo_contrato(None, "GACM940101HDFRRR07"),
            "contrato-GACM940101HDFRRR07.pdf"
        );
        assert_eq!(
            nombre_archivo_contrato(Some("attachment"), "GACM940101HDFRRR07"),
            "contrato-GACM940101HDFRRR07.pdf"
        );
        assert_eq!(
            nombre_archivo_contrato(Some("attachment; filename=\"\""), "X"),
            "contrato-X.pdf",
            "filename vacío cuenta como ausente"
        );
    }

    #[test]
    fn js_descarga_embebe_base64_y_nombre_escapado() {
        let js = js_descarga("QUJD", "contrato-X.pdf");
        assert!(js.contains("atob('QUJD')"), "los bytes van embebidos");
        assert!(js.contains("application/pdf"));
        assert!(js.contains("\"contrato-X.pdf\""), "nombre como string JSON");
        assert!(js.contains("a.download"));
        assert!(js.contains("URL.revokeObjectURL"), "limpia el object URL");
    }
}