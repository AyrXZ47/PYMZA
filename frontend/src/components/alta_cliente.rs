//! Alta de cliente: búsqueda en la red por CURP, alta de nuevos clientes y
//! lanzamiento del modal de plan de pagos.

use dioxus::prelude::*;

use crate::api::{
    alerta_info, archivo_a_b64_de, authed_request, confirmar_verificacion, ine_verificada,
    kyc_verificar, mensaje_kyc, mensaje_recibo, parsear_kyc, parsear_recibo, recibo_subir,
    sesion_ok, solicitar_verificacion, telefono_verificado, validar_archivo,
};
use crate::components::plan_modal::PlanModal;

/// Fase de un flujo de subida de archivo (INE o recibo): reposo → leyendo el
/// archivo → enviando al backend.
#[derive(Clone, Copy, PartialEq)]
enum Fase {
    Reposo,
    Leyendo,
    Enviando,
}

#[component]
pub fn AltaCliente(token: Signal<String>, is_authenticated: Signal<bool>) -> Element {
    let mut curp_input = use_signal(|| String::new());
    let mut search_result = use_signal(|| None::<serde_json::Value>);
    let mut search_status = use_signal(|| String::new());

    let mut show_plan_modal = use_signal(|| false);

    let mut alta_nombre = use_signal(|| String::new());
    let mut alta_direccion = use_signal(|| String::new());
    let mut alta_telefono = use_signal(|| String::new());
    let mut alta_correo = use_signal(|| String::new());

    // Verificación de teléfono por OTP (contrato ola 3).
    let mut ver_tel = use_signal(|| String::new());
    let mut ver_codigo = use_signal(|| String::new());
    let mut ver_enviado = use_signal(|| false);
    let mut ver_ocupado = use_signal(|| false);
    let mut ver_error = use_signal(|| None::<String>);
    let mut verificado = use_signal(|| false);

    // Verificación de INE por OCR (contrato ola 5): archivo guardado como
    // handle (la lectura real ocurre al presionar el botón) + semáforo.
    let mut kyc_archivo = use_signal(|| None::<dioxus_elements::FileData>);
    let mut kyc_fase = use_signal(|| Fase::Reposo);
    let mut kyc_error = use_signal(|| None::<String>);
    let mut kyc_res = use_signal(|| None::<(&'static str, String)>);

    // Score por recibos de servicios (contrato ola 5).
    let mut recibo_tipo = use_signal(|| "luz".to_string());
    let mut recibo_archivo = use_signal(|| None::<dioxus_elements::FileData>);
    let mut recibo_fase = use_signal(|| Fase::Reposo);
    let mut recibo_error = use_signal(|| None::<String>);
    let mut recibo_res = use_signal(|| None::<String>);

    // Al cargar/cambiar el cliente en el panel: prellenar el teléfono y
    // reiniciar los flujos OTP, KYC y recibos.
    use_effect(move || {
        if let Some(cliente) = search_result() {
            ver_tel.set(cliente["telefono"].as_str().unwrap_or("").to_string());
            verificado.set(telefono_verificado(&cliente));
            ver_codigo.set(String::new());
            ver_enviado.set(false);
            ver_error.set(None);
            kyc_archivo.set(None);
            kyc_fase.set(Fase::Reposo);
            kyc_error.set(None);
            kyc_res.set(None);
            recibo_tipo.set("luz".to_string());
            recibo_archivo.set(None);
            recibo_fase.set(Fase::Reposo);
            recibo_error.set(None);
            recibo_res.set(None);
        }
    });

    // Estado derivado del cliente cargado: CURP para las subidas y si la INE
    // ya quedó verificada (JSON del servidor o lograda en esta sesión).
    let curp_capturada = search_result()
        .and_then(|c| c["curp"].as_str().map(|s| s.to_string()))
        .unwrap_or_default();
    let ine_ya = search_result().is_some_and(|c| ine_verificada(&c));
    let kyc_hecho = ine_ya || kyc_res().is_some_and(|(color, _)| color == "green");
    // Cada onclick de rsx mueve sus capturas: un clon por flujo evita el
    // doble move de la CURP.
    let curp_kyc = curp_capturada.clone();
    let curp_recibo = curp_capturada.clone();

    let status = search_status();
    rsx! {
        div {
            h2 { class: "text-2xl font-bold mb-6 text-slate-900 dark:text-white", "Validación de Nuevo Cliente" }
            div { class: "flex gap-3 mb-6",
                input {
                    class: "flex-1 bg-white border border-slate-300 text-slate-900 placeholder-slate-400 rounded-lg px-4 py-3 outline-none focus:border-blue-500 transition-colors dark:bg-slate-900 dark:border-slate-600 dark:text-slate-200 dark:placeholder-slate-500",
                    placeholder: "CURP o ID del Cliente",
                    value: "{curp_input}",
                    oninput: move |e| curp_input.set(e.value()),
                }
                button {
                    class: "bg-blue-600 hover:bg-blue-700 text-white font-semibold px-6 py-3 rounded-lg transition-colors whitespace-nowrap",
                    onclick: move |_| {
                        let curp = curp_input();
                        let token_val = token();
                        search_result.set(None);
                        search_status.set(String::new());
                        show_plan_modal.set(false);
                        spawn(async move {
                            match authed_request(reqwest::Method::GET, format!("/api/clientes/{curp}"), &token_val)
                                .send()
                                .await
                            {
                                Ok(res) => {
                                    if sesion_ok(&res, is_authenticated, token) {
                                        match res.json::<serde_json::Value>().await {
                                        Ok(data) => {
                                            if data["status"] == "success" {
                                                if let Some(cliente) = data.get("cliente") {
                                                    search_result.set(Some(cliente.clone()));
                                                }
                                            } else if data["status"] == "not_found" {
                                                search_status.set("Cliente no encontrado en la red".to_string());
                                            } else {
                                                search_status.set("Error del servidor".to_string());
                                            }
                                        }
                                        Err(_) => {
                                            search_status.set("Error al procesar respuesta".to_string());
                                        }
                                        }
                                    }
                                }
                                Err(_) => {
                                    search_status.set("Error de conexión con el servidor".to_string());
                                }
                            }
                        });
                    },
                    "Buscar en Red PYMZA"
                }
            }

            if let Some(cliente) = search_result() {
                div {
                    class: "bg-green-50 border border-green-300 rounded-xl p-6 max-w-lg dark:bg-green-900/20 dark:border-green-700/50",
                    div { class: "flex items-center gap-2 mb-4",
                        div { class: "text-green-700 dark:text-green-400 font-semibold", "¡Cliente encontrado en Red PYMZA!" }
                    }
                    if let Some((motivo, empresa)) = alerta_info(&cliente) {
                        div {
                            class: "mb-4 border border-amber-400 bg-amber-50 rounded-lg p-4 dark:border-amber-500/60 dark:bg-amber-950/50",
                            div { class: "flex items-center gap-2 text-amber-700 font-semibold dark:text-amber-400",
                                svg { class: "w-5 h-5", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                                    path { d: "M12 9v4m0 4h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" }
                                }
                                "Alerta de Morosidad"
                            }
                            div { class: "mt-2 text-amber-800 dark:text-amber-200 text-sm", "{motivo}" }
                            div { class: "mt-1 text-amber-700/90 dark:text-amber-400/90 text-xs", "Reportada por: {empresa}" }
                        }
                    }
                    div { class: "grid grid-cols-2 gap-4",
                        div {
                            div { class: "text-slate-500 text-xs uppercase mb-1 dark:text-slate-400", "Nombre Completo" }
                            div { class: "text-slate-900 font-medium dark:text-white", "{cliente[\"nombre_completo\"].as_str().unwrap_or(\"—\")}" }
                        }
                        div {
                            div { class: "text-slate-500 text-xs uppercase mb-1 dark:text-slate-400", "Score Crediticio" }
                            div { class: "text-3xl font-bold text-green-600 dark:text-green-400", "{cliente[\"score\"].as_i64().unwrap_or(0)}" }
                        }
                        div {
                            div { class: "text-slate-500 text-xs uppercase mb-1 dark:text-slate-400", "Nivel de Riesgo" }
                            div { class: "text-slate-900 font-medium dark:text-white", "{cliente[\"nivel_riesgo\"].as_str().unwrap_or(\"—\")}" }
                        }
                    }
                    if verificado() || kyc_hecho {
                        div { class: "mt-6 flex items-center gap-4",
                            if verificado() {
                                span { class: "text-sm font-semibold text-green-700 dark:text-green-400", "✓ Teléfono" }
                            }
                            if kyc_hecho {
                                span { class: "text-sm font-semibold text-green-700 dark:text-green-400", "✓ INE" }
                            }
                        }
                    }
                    if !verificado() {
                        div { class: "mt-6 pt-4 border-t border-slate-200 dark:border-slate-700",
                            div { class: "text-slate-900 font-medium dark:text-white mb-2", "Verificar teléfono" }
                            if let Some(msg) = ver_error() {
                                p { class: "text-red-500 text-sm mb-2", "{msg}" }
                            }
                            div { class: "flex flex-col gap-4",
                                input {
                                    class: "bg-white border border-slate-300 text-slate-900 rounded-lg px-3 py-2 outline-none focus:border-blue-500 dark:bg-slate-800 dark:border-slate-600 dark:text-white",
                                    placeholder: "Teléfono",
                                    value: ver_tel(),
                                    oninput: move |e| {
                                        ver_tel.set(e.value());
                                        if ver_enviado() {
                                            // el desafío va ligado a curp+telefono:
                                            // si cambia el teléfono, código nuevo
                                            ver_enviado.set(false);
                                            ver_error.set(None);
                                        }
                                    },
                                }
                                if ver_enviado() {
                                    input {
                                        class: "bg-white border border-slate-300 text-slate-900 rounded-lg px-3 py-2 outline-none focus:border-blue-500 dark:bg-slate-800 dark:border-slate-600 dark:text-white",
                                        placeholder: "Código de 6 dígitos",
                                        maxlength: 6,
                                        value: ver_codigo(),
                                        oninput: move |e| ver_codigo.set(e.value()),
                                    }
                                    p { class: "text-xs text-slate-500 dark:text-slate-400", "Revisa tu WhatsApp — en dev el código aparece en el log del backend" }
                                }
                                button {
                                    class: "bg-blue-600 hover:bg-blue-700 text-white font-semibold px-3 py-2 rounded-lg",
                                    disabled: ver_ocupado(),
                                    onclick: move |_| {
                                        let curp_cliente = cliente["curp"].as_str().unwrap_or("").to_string();
                                        let token_val = token();
                                        ver_error.set(None);
                                        if ver_enviado() {
                                            let tel = ver_tel();
                                            let codigo = ver_codigo();
                                            ver_ocupado.set(true);
                                            spawn(async move {
                                                match confirmar_verificacion(&curp_cliente, &tel, &codigo, &token_val).send().await {
                                                    Ok(res) => {
                                                        if sesion_ok(&res, is_authenticated, token) {
                                                            match res.json::<serde_json::Value>().await {
                                                                Ok(data) => {
                                                                    if data["status"] == "success" {
                                                                        verificado.set(true);
                                                                        ver_enviado.set(false);
                                                                        ver_codigo.set(String::new());
                                                                    } else {
                                                                        ver_error.set(Some(data["message"].as_str().unwrap_or("Código inválido o expirado").to_string()));
                                                                    }
                                                                }
                                                                Err(_) => {
                                                                    ver_error.set(Some("Error al procesar respuesta".to_string()));
                                                                }
                                                            }
                                                        }
                                                    }
                                                    Err(_) => {
                                                        ver_error.set(Some("Error de conexión con el servidor".to_string()));
                                                    }
                                                }
                                                ver_ocupado.set(false);
                                            });
                                        } else {
                                            let tel = ver_tel();
                                            ver_ocupado.set(true);
                                            spawn(async move {
                                                match solicitar_verificacion(&curp_cliente, &tel, &token_val).send().await {
                                                    Ok(res) => {
                                                        if sesion_ok(&res, is_authenticated, token) {
                                                            match res.json::<serde_json::Value>().await {
                                                                Ok(data) => {
                                                                    if data["status"] == "success" {
                                                                        ver_enviado.set(true);
                                                                    } else {
                                                                        ver_error.set(Some(data["message"].as_str().unwrap_or("No se pudo enviar el código").to_string()));
                                                                    }
                                                                }
                                                                Err(_) => {
                                                                    ver_error.set(Some("Error al procesar respuesta".to_string()));
                                                                }
                                                            }
                                                        }
                                                    }
                                                    Err(_) => {
                                                        ver_error.set(Some("Error de conexión con el servidor".to_string()));
                                                    }
                                                }
                                                ver_ocupado.set(false);
                                            });
                                        }
                                    },
                                    {match (ver_ocupado(), ver_enviado()) {
                                        (true, false) => "Enviando...",
                                        (true, true) => "Confirmando...",
                                        (false, true) => "Confirmar",
                                        (false, false) => "Enviar código",
                                    }}
                                }
                            }
                        }
                    }
                    if !ine_ya {
                        div { class: "mt-6 pt-4 border-t border-slate-200 dark:border-slate-700",
                            div { class: "text-slate-900 font-medium dark:text-white mb-2", "Verificar INE" }
                            if let Some(msg) = kyc_error() {
                                p { class: "text-red-500 text-sm mb-2", "{msg}" }
                            }
                            if let Some((color, msg)) = kyc_res() {
                                p {
                                    class: if color == "green" { "text-green-700 dark:text-green-400 text-sm mb-2" } else { "text-amber-700 dark:text-amber-400 text-sm mb-2" },
                                    "{msg}"
                                }
                            }
                            if !kyc_hecho {
                                div { class: "flex flex-col gap-3",
                                    InputArchivo { archivo: kyc_archivo, error: kyc_error }
                                    if let Some(f) = kyc_archivo() {
                                        div { class: "text-xs text-slate-500 dark:text-slate-400", "Archivo: {f.name()}" }
                                    }
                                    button {
                                        class: "bg-blue-600 hover:bg-blue-700 text-white font-semibold px-3 py-2 rounded-lg",
                                        disabled: kyc_fase() != Fase::Reposo,
                                        onclick: move |_| {
                                            let Some(file) = kyc_archivo() else { return };
                                            let curp_cliente = curp_kyc.clone();
                                            let token_val = token();
                                            kyc_error.set(None);
                                            kyc_res.set(None);
                                            kyc_fase.set(Fase::Leyendo);
                                            spawn(async move {
                                                let b64 = match archivo_a_b64_de(&file).await {
                                                    Ok(b) => b,
                                                    Err(e) => {
                                                        kyc_fase.set(Fase::Reposo);
                                                        kyc_error.set(Some(e));
                                                        return;
                                                    }
                                                };
                                                let mime = file.content_type().unwrap_or_default();
                                                kyc_fase.set(Fase::Enviando);
                                                match kyc_verificar(&curp_cliente, &b64, &mime, &token_val).send().await {
                                                    Ok(res) => {
                                                        if sesion_ok(&res, is_authenticated, token) {
                                                            match res.json::<serde_json::Value>().await {
                                                                Ok(data) => match parsear_kyc(&data) {
                                                                    Some(kyc) => kyc_res.set(Some(mensaje_kyc(&kyc, &curp_cliente))),
                                                                    None => kyc_error.set(Some(data["message"].as_str().unwrap_or("Error del servidor").to_string())),
                                                                },
                                                                Err(_) => kyc_error.set(Some("Error al procesar respuesta".to_string())),
                                                            }
                                                        }
                                                    }
                                                    Err(_) => kyc_error.set(Some("Error de conexión con el servidor".to_string())),
                                                }
                                                kyc_fase.set(Fase::Reposo);
                                            });
                                        },
                                        {match kyc_fase() {
                                            Fase::Leyendo => "Leyendo archivo...",
                                            Fase::Enviando => "Enviando...",
                                            Fase::Reposo => "Verificar INE",
                                        }}
                                    }
                                }
                            }
                        }
                    }
                    div { class: "mt-6 pt-4 border-t border-slate-200 dark:border-slate-700",
                        div { class: "text-slate-900 font-medium dark:text-white mb-1", "Score por recibos de servicios" }
                        div { class: "text-xs text-slate-500 dark:text-slate-400 mb-3", "Cada recibo legible de servicios suma +25 al score (máximo 2)." }
                        if let Some(msg) = recibo_error() {
                            p { class: "text-red-500 text-sm mb-2", "{msg}" }
                        }
                        if let Some(msg) = recibo_res() {
                            p { class: "text-green-700 dark:text-green-400 text-sm mb-2", "{msg}" }
                        }
                        div { class: "flex flex-col gap-3",
                            select {
                                class: "bg-white border border-slate-300 text-slate-900 rounded-lg px-3 py-2 outline-none focus:border-blue-500 dark:bg-slate-800 dark:border-slate-600 dark:text-white",
                                value: "{recibo_tipo()}",
                                onchange: move |e| recibo_tipo.set(e.value()),
                                option { value: "luz", "Luz" }
                                option { value: "agua", "Agua" }
                                option { value: "telefono", "Teléfono" }
                            }
                            InputArchivo { archivo: recibo_archivo, error: recibo_error }
                            if let Some(f) = recibo_archivo() {
                                div { class: "text-xs text-slate-500 dark:text-slate-400", "Archivo: {f.name()}" }
                            }
                            button {
                                class: "bg-blue-600 hover:bg-blue-700 text-white font-semibold px-3 py-2 rounded-lg",
                                disabled: recibo_fase() != Fase::Reposo,
                                onclick: move |_| {
                                    let Some(file) = recibo_archivo() else { return };
                                    let curp_cliente = curp_recibo.clone();
                                    let tipo = recibo_tipo();
                                    let token_val = token();
                                    recibo_error.set(None);
                                    recibo_res.set(None);
                                    recibo_fase.set(Fase::Leyendo);
                                    spawn(async move {
                                        let b64 = match archivo_a_b64_de(&file).await {
                                            Ok(b) => b,
                                            Err(e) => {
                                                recibo_fase.set(Fase::Reposo);
                                                recibo_error.set(Some(e));
                                                return;
                                            }
                                        };
                                        let mime = file.content_type().unwrap_or_default();
                                        recibo_fase.set(Fase::Enviando);
                                        match recibo_subir(&curp_cliente, &b64, &mime, &tipo, &token_val).send().await {
                                            Ok(res) => {
                                                if sesion_ok(&res, is_authenticated, token) {
                                                    match res.json::<serde_json::Value>().await {
                                                        Ok(data) => match parsear_recibo(&data) {
                                                            Some(r) => recibo_res.set(Some(mensaje_recibo(&r))),
                                                            None => recibo_error.set(Some(data["message"].as_str().unwrap_or("Error del servidor").to_string())),
                                                        },
                                                        Err(_) => recibo_error.set(Some("Error al procesar respuesta".to_string())),
                                                    }
                                                }
                                            }
                                            Err(_) => recibo_error.set(Some("Error de conexión con el servidor".to_string())),
                                        }
                                        recibo_fase.set(Fase::Reposo);
                                    });
                                },
                                {match recibo_fase() {
                                    Fase::Leyendo => "Leyendo archivo...",
                                    Fase::Enviando => "Enviando...",
                                    Fase::Reposo => "Subir recibo",
                                }}
                            }
                        }
                    }
                    button {
                        class: "mt-6 bg-blue-600 hover:bg-blue-700 text-white font-semibold px-6 py-3 rounded-lg",
                        onclick: move |_| show_plan_modal.set(true),
                        "Ofrecer Plan de Pagos"
                    }
                }
            } else if !status.is_empty() {
                if status == "Cliente no encontrado en la red" {
                    div {
                        class: "bg-yellow-50 border border-yellow-300 rounded-xl p-6 max-w-lg dark:bg-yellow-900/20 dark:border-yellow-700/50",
                        h3 { class: "text-yellow-700 dark:text-yellow-400 font-semibold mb-4", "Cliente no encontrado — Alta de Cliente" }
                        div { class: "flex flex-col gap-4",
                            input {
                                class: "bg-white border border-slate-300 text-slate-900 rounded-lg px-3 py-2 outline-none focus:border-blue-500 dark:bg-slate-800 dark:border-slate-600 dark:text-white",
                                placeholder: "Nombre Completo",
                                value: alta_nombre(),
                                oninput: move |e| alta_nombre.set(e.value()),
                            }
                            input {
                                class: "bg-white border border-slate-300 text-slate-900 rounded-lg px-3 py-2 outline-none focus:border-blue-500 dark:bg-slate-800 dark:border-slate-600 dark:text-white",
                                placeholder: "Dirección",
                                value: alta_direccion(),
                                oninput: move |e| alta_direccion.set(e.value()),
                            }
                            input {
                                class: "bg-white border border-slate-300 text-slate-900 rounded-lg px-3 py-2 outline-none focus:border-blue-500 dark:bg-slate-800 dark:border-slate-600 dark:text-white",
                                placeholder: "Teléfono",
                                value: alta_telefono(),
                                oninput: move |e| alta_telefono.set(e.value()),
                            }
                            input {
                                class: "bg-white border border-slate-300 text-slate-900 rounded-lg px-3 py-2 outline-none focus:border-blue-500 dark:bg-slate-800 dark:border-slate-600 dark:text-white",
                                placeholder: "Correo (opcional)",
                                value: alta_correo(),
                                oninput: move |e| alta_correo.set(e.value()),
                            }
                            button {
                                class: "bg-green-600 hover:bg-green-700 text-white font-semibold px-6 py-3 rounded-lg",
                                onclick: move |_| {
                                    let curp = curp_input();
                                    let nombre = alta_nombre();
                                    let direccion = alta_direccion();
                                    let telefono = alta_telefono();
                                    let correo = alta_correo();
                                    let token_val = token();
                                    spawn(async move {
                                        let correo_enviar: Option<String> = if correo.is_empty() {
                                            None
                                        } else {
                                            Some(correo)
                                        };
                                        let body = serde_json::json!({
                                            "curp": curp,
                                            "nombre_completo": nombre,
                                            "direccion": direccion,
                                            "telefono": telefono,
                                            "correo": correo_enviar,
                                        });
                                        match authed_request(reqwest::Method::POST, "/api/clientes".to_string(), &token_val)
                                            .json(&body)
                                            .send()
                                            .await
                                        {
                                            Ok(res) => {
                                                if sesion_ok(&res, is_authenticated, token) {
                                                    match res.json::<serde_json::Value>().await {
                                                        Ok(data) => {
                                                            if data["status"] == "success" {
                                                                if let Some(cliente) = data.get("cliente") {
                                                                    search_result.set(Some(cliente.clone()));
                                                                    alta_nombre.set(String::new());
                                                                    alta_direccion.set(String::new());
                                                                    alta_telefono.set(String::new());
                                                                    alta_correo.set(String::new());
                                                                    search_status.set(String::new());
                                                                }
                                                            } else {
                                                                search_status.set(data["message"].as_str().unwrap_or("Error al registrar").to_string());
                                                            }
                                                        }
                                                        Err(_) => {
                                                            search_status.set("Error al procesar respuesta".to_string());
                                                        }
                                                    }
                                                }
                                            }
                                            Err(_) => {
                                                search_status.set("Error de conexión con el servidor".to_string());
                                            }
                                        }
                                    });
                                },
                                "Registrar Cliente"
                            }
                        }
                    }
                } else {
                    div {
                        class: "bg-yellow-50 border border-yellow-300 rounded-xl p-6 max-w-lg dark:bg-yellow-900/20 dark:border-yellow-700/50",
                        div { class: "text-yellow-700 dark:text-yellow-400 font-semibold mb-4", "{status}" }
                    }
                }
            }

            if show_plan_modal() {
                PlanModal { show_plan_modal, curp: curp_input(), token, is_authenticated }
            }
        }
    }
}

/// Input de archivo compartido por los paneles KYC y recibos: valida mime y
/// tamaño ANTES de guardar el handle del archivo — nada sale del navegador si
/// falla la validación client-side (los bytes se leen al presionar el botón).
#[component]
fn InputArchivo(
    mut archivo: Signal<Option<dioxus_elements::FileData>>,
    mut error: Signal<Option<String>>,
) -> Element {
    rsx! {
        input {
            class: "block w-full text-sm text-slate-500 border border-slate-300 rounded-lg cursor-pointer bg-white dark:text-slate-400 dark:border-slate-600 dark:bg-slate-800 file:mr-3 file:py-2 file:px-3 file:rounded-lg file:border-0 file:bg-blue-50 file:text-blue-700 file:font-semibold hover:file:bg-blue-100 dark:file:bg-slate-700 dark:file:text-slate-200 dark:hover:file:bg-slate-600",
            r#type: "file",
            accept: "image/png,image/jpeg,image/webp",
            onchange: move |e| {
                archivo.set(None);
                let Some(file) = e.files().first().cloned() else {
                    error.set(Some("Selecciona un archivo".to_string()));
                    return;
                };
                match validar_archivo(file.size(), file.content_type().as_deref()) {
                    Some(motivo) => error.set(Some(motivo)),
                    None => archivo.set(Some(file)),
                }
            },
        }
    }
}