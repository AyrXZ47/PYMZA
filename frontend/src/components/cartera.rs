//! Cartera: tabla de planes de pago activos de la empresa.

use dioxus::prelude::*;

use crate::api::{authed_request, sesion_ok};

#[component]
pub fn Cartera(token: Signal<String>, is_authenticated: Signal<bool>) -> Element {
    let mut cartera_planes = use_signal(|| Vec::<serde_json::Value>::new());
    let mut cartera_loaded = use_signal(|| false);

    // Se monta/desmonta al navegar por el menú: refetchea la cartera en cada visita.
    if !cartera_loaded() {
        let token_val = token();
        cartera_loaded.set(true);
        spawn(async move {
            match authed_request(reqwest::Method::GET, "/api/creditos".to_string(), &token_val)
                .send()
                .await
            {
                Ok(res) => {
                    if sesion_ok(&res, is_authenticated, token) {
                        if let Ok(data) = res.json::<serde_json::Value>().await {
                            if data["status"] == "success" {
                                if let Some(arr) = data["creditos"].as_array() {
                                    cartera_planes.set(arr.clone());
                                }
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        });
    }

    let planes = cartera_planes();
    rsx! {
        div {
            h2 { class: "text-2xl font-bold mb-6 text-white", "Cartera de Créditos" }
            if planes.is_empty() {
                div { class: "text-slate-400", "No hay créditos activos para esta empresa." }
            } else {
                div { class: "overflow-x-auto",
                    table { class: "w-full text-sm text-left bg-slate-900 rounded-xl border border-slate-700",
                        thead {
                            tr { class: "text-slate-400 border-b border-slate-700",
                                th { class: "py-3 px-4", "Producto" }
                                th { class: "py-3 px-4", "Cliente (CURP)" }
                                th { class: "py-3 px-4", "Monto Total" }
                                th { class: "py-3 px-4", "Plazo" }
                                th { class: "py-3 px-4", "Pago Mensual" }
                                th { class: "py-3 px-4", "Interés" }
                                th { class: "py-3 px-4", "Estado" }
                                th { class: "py-3 px-4", "Fecha" }
                            }
                        }
                        tbody {
                            for p in &planes {
                                tr { class: "border-b border-slate-700/50 text-slate-300",
                                    td { class: "py-3 px-4", "{p[\"producto\"].as_str().unwrap_or(\"—\")}" }
                                    td { class: "py-3 px-4 font-mono", "{p[\"cliente_curp\"].as_str().unwrap_or(\"—\")}" }
                                    td { class: "py-3 px-4", "${p[\"monto_total\"].as_f64().unwrap_or(0.0)} MXN" }
                                    td { class: "py-3 px-4", "{p[\"plazo_meses\"].as_i64().unwrap_or(0)} meses" }
                                    td { class: "py-3 px-4", "${p[\"pago_mensual\"].as_f64().unwrap_or(0.0)} MXN" }
                                    td { class: "py-3 px-4", "{(p[\"tasa_interes\"].as_f64().unwrap_or(0.0) * 100.0) as i32}%" }
                                    td { class: "py-3 px-4",
                                        span {
                                            class: format!("px-2 py-1 rounded-full text-xs {}", if p["estado"].as_str() == Some("Activo") { "bg-green-900/50 text-green-400" } else { "bg-slate-700 text-slate-300" }),
                                            "{p[\"estado\"].as_str().unwrap_or(\"—\")}"
                                        }
                                    }
                                    td { class: "py-3 px-4", "{p[\"fecha\"].as_str().unwrap_or(\"—\")}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}