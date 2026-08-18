//! Login + alta de empresa (form embebido al fondo de la pantalla de login).

use dioxus::prelude::*;

use crate::api::{http_client, API_BASE};

#[component]
pub fn Login(
    mut is_authenticated: Signal<bool>,
    mut current_company: Signal<String>,
    mut token: Signal<String>,
) -> Element {
    let mut correo = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut error_msg = use_signal(|| None::<String>);
    let mut reg_nombre = use_signal(|| String::new());
    let mut reg_correo = use_signal(|| String::new());
    let mut reg_password = use_signal(|| String::new());
    let mut reg_error = use_signal(|| None::<String>);
    let mut reg_ok = use_signal(|| false);

    rsx! {
        div {
            class: "flex items-center justify-center h-screen bg-slate-900",
            div {
                class: "bg-slate-800 p-8 rounded-2xl shadow-2xl border border-slate-700 w-full max-w-md",
                div {
                    class: "flex flex-col items-center mb-8",
                    div { class: "text-blue-500 font-bold text-5xl mb-2", "PYMZA" }
                    div { class: "text-slate-400 text-sm", "Plataforma de evaluación crediticia" }
                }
                div { class: "flex flex-col gap-4",
                    input {
                        class: "bg-slate-700 border border-slate-600 text-slate-200 placeholder-slate-400 rounded-lg px-4 py-3 outline-none focus:border-blue-500 transition-colors",
                        placeholder: "Correo de la Empresa",
                        value: correo(),
                        oninput: move |e| correo.set(e.value()),
                    }
                    input {
                        class: "bg-slate-700 border border-slate-600 text-slate-200 placeholder-slate-400 rounded-lg px-4 py-3 outline-none focus:border-blue-500 transition-colors",
                        placeholder: "Contraseña",
                        value: password(),
                        oninput: move |e| password.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                let correo = correo();
                                let password_val = password();
                                spawn(async move {
                                    let body = serde_json::json!({
                                        "correo": correo,
                                        "password": password_val
                                    });
                                    match http_client().post(format!("{API_BASE}/api/login"))
                                        .json(&body)
                                        .send()
                                        .await
                                    {
                                        Ok(res) => {
                                            match res.json::<serde_json::Value>().await {
                                                Ok(data) => {
                                                    if data["status"] == "success" {
                                                        let token_nuevo = data["token"].as_str().unwrap_or("").to_string();
                                                        crate::api::token_guardar(&token_nuevo);
                                                        current_company.set(data["empresa"].as_str().unwrap_or("").to_string());
                                                        token.set(token_nuevo);
                                                        is_authenticated.set(true);
                                                    } else {
                                                        error_msg.set(Some("Credenciales inválidas".to_string()));
                                                    }
                                                }
                                                Err(_) => {
                                                    error_msg.set(Some("Credenciales inválidas".to_string()));
                                                }
                                            }
                                        }
                                        Err(_) => {
                                            error_msg.set(Some("Credenciales inválidas".to_string()));
                                        }
                                    }
                                });
                            }
                        },
                    }
                    button {
                        class: "bg-blue-600 hover:bg-blue-700 text-white font-semibold rounded-lg px-4 py-3 transition-colors mt-2",
                        onclick: move |_| {
                            let correo = correo();
                            let password_val = password();
                            spawn(async move {
                                let body = serde_json::json!({
                                    "correo": correo,
                                    "password": password_val
                                });
                                match http_client().post(format!("{API_BASE}/api/login"))
                                    .json(&body)
                                    .send()
                                    .await
                                {
                                    Ok(res) => {
                                        match res.json::<serde_json::Value>().await {
                                            Ok(data) => {
                                                if data["status"] == "success" {
                                                    let token_nuevo = data["token"].as_str().unwrap_or("").to_string();
                                                    crate::api::token_guardar(&token_nuevo);
                                                    current_company.set(data["empresa"].as_str().unwrap_or("").to_string());
                                                    token.set(token_nuevo);
                                                    is_authenticated.set(true);
                                                } else {
                                                    error_msg.set(Some("Credenciales inválidas".to_string()));
                                                }
                                            }
                                            Err(_) => {
                                                error_msg.set(Some("Credenciales inválidas".to_string()));
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        error_msg.set(Some("Credenciales inválidas".to_string()));
                                    }
                                }
                            });
                        },
                        "Iniciar Sesión"
                    }
                    if let Some(msg) = error_msg() {
                        p {
                            class: "text-red-500 text-sm text-center",
                            "{msg}"
                        }
                    }
                }
                div {
                    class: "mt-8 pt-6 border-t border-slate-700",
                    div {
                        class: "flex flex-col items-center mb-6",
                        div { class: "text-white font-bold text-lg", "Registrar Empresa" }
                        div { class: "text-slate-400 text-sm text-center", "Únete a la red PYMZA y otorga crédito con cobranza respaldada" }
                    }
                    div { class: "flex flex-col gap-3",
                        input {
                            class: "bg-slate-700 border border-slate-600 text-slate-200 placeholder-slate-400 rounded-lg px-4 py-3 outline-none focus:border-blue-500 transition-colors",
                            placeholder: "Nombre de la Empresa",
                            value: reg_nombre(),
                            oninput: move |e| reg_nombre.set(e.value()),
                        }
                        input {
                            class: "bg-slate-700 border border-slate-600 text-slate-200 placeholder-slate-400 rounded-lg px-4 py-3 outline-none focus:border-blue-500 transition-colors",
                            placeholder: "Correo de la Empresa",
                            value: reg_correo(),
                            oninput: move |e| reg_correo.set(e.value()),
                        }
                        input {
                            class: "bg-slate-700 border border-slate-600 text-slate-200 placeholder-slate-400 rounded-lg px-4 py-3 outline-none focus:border-blue-500 transition-colors",
                            placeholder: "Contraseña (mínimo 8 caracteres)",
                            type: "password",
                            value: reg_password(),
                            oninput: move |e| reg_password.set(e.value()),
                        }
                        button {
                            class: "bg-slate-700 hover:bg-slate-600 text-white font-semibold rounded-lg px-4 py-3 transition-colors",
                            onclick: move |_| {
                                let nombre = reg_nombre();
                                let correo_reg = reg_correo();
                                let password_reg = reg_password();
                                let prefill_correo = correo_reg.clone();
                                reg_error.set(None);
                                reg_ok.set(false);
                                spawn(async move {
                                    let body = serde_json::json!({
                                        "correo": correo_reg,
                                        "password": password_reg,
                                        "nombre_empresa": nombre,
                                    });
                                    match http_client().post(format!("{API_BASE}/api/empresas"))
                                        .json(&body)
                                        .send()
                                        .await
                                    {
                                        Ok(res) => {
                                            match res.json::<serde_json::Value>().await {
                                                Ok(data) => {
                                                    if data["status"] == "success" {
                                                        reg_ok.set(true);
                                                        correo.set(prefill_correo);
                                                        reg_nombre.set(String::new());
                                                        reg_password.set(String::new());
                                                    } else {
                                                        reg_error.set(Some(data["message"].as_str().unwrap_or("Error al registrar la empresa").to_string()));
                                                    }
                                                }
                                                Err(_) => {
                                                    reg_error.set(Some("Error al procesar la respuesta".to_string()));
                                                }
                                            }
                                        }
                                        Err(_) => {
                                            reg_error.set(Some("Error de conexión con el servidor".to_string()));
                                        }
                                    }
                                });
                            },
                            "Registrar Empresa"
                        }
                        if reg_ok() {
                            p { class: "text-green-500 text-sm text-center", "Empresa registrada correctamente. Ya puedes iniciar sesión." }
                        }
                        if let Some(msg) = reg_error() {
                            p { class: "text-red-500 text-sm text-center", "{msg}" }
                        }
                    }
                }
            }
        }
    }
}