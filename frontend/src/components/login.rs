//! Login: solo correo + password + entrar. El registro vive en su propia
//! vista (`registro.rs`); enlace cruzado y logo → Landing.

use dioxus::prelude::*;

use crate::api::{http_client, token_guardar, API_BASE};
use crate::VistaPublica;

#[component]
pub fn Login(
    mut vista_publica: Signal<VistaPublica>,
    mut is_authenticated: Signal<bool>,
    mut current_company: Signal<String>,
    mut token: Signal<String>,
) -> Element {
    let mut correo = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut error_msg = use_signal(|| None::<String>);

    let intentar_login = move |correo: String, password_val: String| {
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
                                token_guardar(&token_nuevo);
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
    };

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
                    div { class: "text-slate-500 dark:text-slate-400 text-sm", "Plataforma de evaluación crediticia" }
                }
                div { class: "flex flex-col gap-4",
                    input {
                        class: "bg-white border border-slate-300 text-slate-900 placeholder-slate-400 rounded-lg px-4 py-3 outline-none focus:border-blue-500 transition-colors dark:bg-slate-700 dark:border-slate-600 dark:text-slate-200 dark:placeholder-slate-400",
                        placeholder: "Correo de la Empresa",
                        value: correo(),
                        oninput: move |e| correo.set(e.value()),
                    }
                    input {
                        class: "bg-white border border-slate-300 text-slate-900 placeholder-slate-400 rounded-lg px-4 py-3 outline-none focus:border-blue-500 transition-colors dark:bg-slate-700 dark:border-slate-600 dark:text-slate-200 dark:placeholder-slate-400",
                        placeholder: "Contraseña",
                        type: "password",
                        value: password(),
                        oninput: move |e| password.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                let correo = correo();
                                let password_val = password();
                                intentar_login(correo, password_val);
                            }
                        },
                    }
                    button {
                        class: "bg-blue-600 hover:bg-blue-700 text-white font-semibold rounded-lg px-4 py-3 transition-colors mt-2",
                        onclick: move |_| {
                            let correo = correo();
                            let password_val = password();
                            intentar_login(correo, password_val);
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
                    class: "mt-6 pt-4 border-t border-slate-200 text-center dark:border-slate-700",
                    span { class: "text-slate-500 dark:text-slate-400 text-sm", "¿No tienes cuenta? " }
                    button {
                        class: "text-blue-600 dark:text-blue-400 font-semibold text-sm hover:underline",
                        onclick: move |_| vista_publica.set(VistaPublica::Registro),
                        "Regístrate"
                    }
                }
            }
        }
    }
}