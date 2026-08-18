//! Alta de cliente: búsqueda en la red por CURP, alta de nuevos clientes y
//! lanzamiento del modal de plan de pagos.

use dioxus::prelude::*;

use crate::api::{alerta_info, authed_request, sesion_ok};
use crate::components::plan_modal::PlanModal;

#[component]
pub fn AltaCliente(token: Signal<String>, is_authenticated: Signal<bool>) -> Element {
    let mut curp_input = use_signal(|| String::new());
    let mut search_result = use_signal(|| None::<serde_json::Value>);
    let mut search_status = use_signal(|| String::new());

    let mut show_plan_modal = use_signal(|| false);

    let mut alta_nombre = use_signal(|| String::new());
    let mut alta_direccion = use_signal(|| String::new());
    let mut alta_telefono = use_signal(|| String::new());

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
                            button {
                                class: "bg-green-600 hover:bg-green-700 text-white font-semibold px-6 py-3 rounded-lg",
                                onclick: move |_| {
                                    let curp = curp_input();
                                    let nombre = alta_nombre();
                                    let direccion = alta_direccion();
                                    let telefono = alta_telefono();
                                    let token_val = token();
                                    spawn(async move {
                                        let body = serde_json::json!({
                                            "curp": curp,
                                            "nombre_completo": nombre,
                                            "direccion": direccion,
                                            "telefono": telefono,
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