//! Registro de empresa: alta + auto-login. Sin fricción: al éxito de
//! `POST /api/empresas` se llama `POST /api/login` con las mismas credenciales
//! y la empresa entra directo a la app (decision log 2026-08-17).
//! Si el auto-login fallara (raro), se muestra un aviso con salida a Login.

use dioxus::prelude::*;

use crate::api::{http_client, token_guardar, API_BASE};
use crate::VistaPublica;

#[component]
pub fn Registro(
    mut vista_publica: Signal<VistaPublica>,
    mut is_authenticated: Signal<bool>,
    mut current_company: Signal<String>,
    mut token: Signal<String>,
) -> Element {
    let mut reg_nombre = use_signal(|| String::new());
    let mut reg_correo = use_signal(|| String::new());
    let mut reg_password = use_signal(|| String::new());
    let mut reg_error = use_signal(|| None::<String>);
    let mut auto_login_fallo = use_signal(|| false);

    rsx! {
        div {
            class: "flex items-center justify-center min-h-screen bg-slate-100 dark:bg-slate-900",
            div {
                class: "bg-white p-8 rounded-2xl shadow-2xl border border-slate-200 w-full max-w-md dark:bg-slate-800 dark:border-slate-700",
                div {
                    class: "flex flex-col items-center mb-8",
                    div {
                        class: "text-blue-600 dark:text-blue-500 font-bold text-5xl mb-2 cursor-pointer",
                        onclick: move |_| vista_publica.set(VistaPublica::Landing),
                        "PYMZA"
                    }
                    div { class: "text-slate-500 dark:text-slate-400 text-sm", "Únete a la red y otorga crédito con cobranza respaldada" }
                }
                div { class: "flex flex-col gap-3",
                    input {
                        class: "bg-white border border-slate-300 text-slate-900 placeholder-slate-400 rounded-lg px-4 py-3 outline-none focus:border-blue-500 transition-colors dark:bg-slate-700 dark:border-slate-600 dark:text-slate-200 dark:placeholder-slate-400",
                        placeholder: "Nombre de la Empresa",
                        value: reg_nombre(),
                        oninput: move |e| reg_nombre.set(e.value()),
                    }
                    input {
                        class: "bg-white border border-slate-300 text-slate-900 placeholder-slate-400 rounded-lg px-4 py-3 outline-none focus:border-blue-500 transition-colors dark:bg-slate-700 dark:border-slate-600 dark:text-slate-200 dark:placeholder-slate-400",
                        placeholder: "Correo de la Empresa",
                        value: reg_correo(),
                        oninput: move |e| reg_correo.set(e.value()),
                    }
                    input {
                        class: "bg-white border border-slate-300 text-slate-900 placeholder-slate-400 rounded-lg px-4 py-3 outline-none focus:border-blue-500 transition-colors dark:bg-slate-700 dark:border-slate-600 dark:text-slate-200 dark:placeholder-slate-400",
                        placeholder: "Contraseña (mínimo 8 caracteres)",
                        type: "password",
                        value: reg_password(),
                        oninput: move |e| reg_password.set(e.value()),
                    }
                    button {
                        class: "bg-blue-600 hover:bg-blue-700 text-white font-semibold rounded-lg px-4 py-3 transition-colors",
                        onclick: move |_| {
                            let nombre = reg_nombre();
                            let correo_reg = reg_correo();
                            let password_reg = reg_password();
                            reg_error.set(None);
                            auto_login_fallo.set(false);
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
                                                    // Auto-login con las mismas credenciales.
                                                    let login_body = serde_json::json!({
                                                        "correo": correo_reg,
                                                        "password": password_reg,
                                                    });
                                                    match http_client().post(format!("{API_BASE}/api/login"))
                                                        .json(&login_body)
                                                        .send()
                                                        .await
                                                    {
                                                        Ok(res_login) => {
                                                            match res_login.json::<serde_json::Value>().await {
                                                                Ok(data_login) => {
                                                                    if data_login["status"] == "success" {
                                                                        let token_nuevo = data_login["token"].as_str().unwrap_or("").to_string();
                                                                        token_guardar(&token_nuevo);
                                                                        current_company.set(data_login["empresa"].as_str().unwrap_or("").to_string());
                                                                        token.set(token_nuevo);
                                                                        is_authenticated.set(true);
                                                                    } else {
                                                                        auto_login_fallo.set(true);
                                                                    }
                                                                }
                                                                Err(_) => auto_login_fallo.set(true),
                                                            }
                                                        }
                                                        Err(_) => auto_login_fallo.set(true),
                                                    }
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
                        "Crear Cuenta"
                    }
                    if let Some(msg) = reg_error() {
                        p { class: "text-red-500 text-sm text-center", "{msg}" }
                    }
                    if auto_login_fallo() {
                        div {
                            class: "flex flex-col items-center gap-3 border border-amber-500/60 bg-amber-50 rounded-lg p-4 dark:bg-amber-950/50",
                            p { class: "text-amber-700 dark:text-amber-400 text-sm text-center",
                                "Empresa creada, pero no se pudo iniciar sesión automáticamente."
                            }
                            button {
                                class: "bg-blue-600 hover:bg-blue-700 text-white font-semibold px-4 py-2 rounded-lg text-sm",
                                onclick: move |_| vista_publica.set(VistaPublica::Login),
                                "Ir a Iniciar sesión"
                            }
                        }
                    }
                }
                div {
                    class: "mt-6 pt-4 border-t border-slate-200 text-center dark:border-slate-700",
                    span { class: "text-slate-500 dark:text-slate-400 text-sm", "¿Ya tienes cuenta? " }
                    button {
                        class: "text-blue-600 dark:text-blue-400 font-semibold text-sm hover:underline",
                        onclick: move |_| vista_publica.set(VistaPublica::Login),
                        "Inicia sesión"
                    }
                }
            }
        }
    }
}