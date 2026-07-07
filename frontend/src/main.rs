use dioxus::prelude::*;
use std::sync::OnceLock;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| reqwest::Client::new())
}

#[derive(Clone, Copy, PartialEq)]
enum TabState {
    OpenBanking,
    Servicios,
    IneOcr,
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        div {
            class: "flex h-screen text-white",
            Sidebar {}
            MainArea {}
        }
    }
}

#[component]
fn Sidebar() -> Element {
    rsx! {
        div {
            class: "bg-slate-900 w-64 flex flex-col items-center justify-start p-4",
            div {
                class: "text-blue-500 font-bold text-2xl mb-8",
                "PYMZA"
            }
            ul {
                class: "flex flex-col",
                li {
                    class: "p-3 rounded-lg hover:bg-slate-800 hover:text-white transition-colors",
                    a {
                        href: "#",
                        class: "flex items-center",
                        svg { class: "w-5 h-5 mr-3" },
                        "Dashboard"
                    }
                }
                li {
                    class: "p-3 rounded-lg bg-blue-900/50 text-blue-400 hover:bg-slate-800 hover:text-white transition-colors",
                    a {
                        href: "#",
                        class: "flex items-center",
                        svg { class: "w-5 h-5 mr-3" },
                        "Evaluación"
                    }
                }
                li {
                    class: "p-3 rounded-lg hover:bg-slate-800 hover:text-white transition-colors",
                    a {
                        href: "#",
                        class: "flex items-center",
                        svg { class: "w-5 h-5 mr-3" },
                        "Clientes"
                    }
                }
                li {
                    class: "p-3 rounded-lg hover:bg-red-900/30 text-red-400 hover:text-white transition-colors",
                    a {
                        href: "#",
                        class: "flex items-center",
                        svg { class: "w-5 h-5 mr-3" },
                        "Red PYME"
                    }
                }
            }
        }
    }
}

#[component]
fn MainArea() -> Element {
    let mut active_tab = use_signal(|| TabState::OpenBanking);
    let mut ocr_result = use_signal(|| None::<String>);
    let mut status_message = use_signal(|| None::<String>);
    // NUEVO: Guardaremos el ID real de Mongo aquí
    let mut document_id = use_signal(|| String::new()); 

    rsx! {
        div {
            class: "bg-slate-800 flex-1 p-8 text-slate-200",
            // Header de Solicitud
            div {
                class: "flex justify-between items-center mb-4",
                div {
                    class: "text-2xl font-bold",
                    "SOLICITUD: Janeth Ramos Zamora"
                }
                div {
                    class: "bg-yellow-500 w-3 h-3 rounded-full mr-2 inline-block",
                }
                div {
                    class: "font-semibold text-xl",
                    "Monto Solicitado: $15,000 MXN"
                }
            }
            // Navegación de Tabs
            div {
                class: "border-b border-slate-700 mb-6",
                div {
                    class: "flex justify-between items-center",
                    div {
                        class: format!("cursor-pointer p-1 {}", if active_tab() == TabState::OpenBanking { "text-blue-400 font-semibold border-b-2 border-blue-500" } else { "text-slate-400" }),
                        onclick: move |_| active_tab.set(TabState::OpenBanking),
                        "TAB 1: Open Banking"
                    }
                    div {
                        class: format!("cursor-pointer p-1 {}", if active_tab() == TabState::Servicios { "text-blue-400 font-semibold border-b-2 border-blue-500" } else { "text-slate-400" }),
                        onclick: move |_| active_tab.set(TabState::Servicios),
                        "TAB 2: Servicios"
                    }
                    div {
                        class: format!("cursor-pointer p-1 {}", if active_tab() == TabState::IneOcr { "text-blue-400 font-semibold border-b-2 border-blue-500" } else { "text-slate-400" }),
                        onclick: move |_| active_tab.set(TabState::IneOcr),
                        "TAB 3: INE/OCR"
                    }
                }
            }
            // Grid de Contenido
            match active_tab() {
                TabState::OpenBanking => rsx! {
                    div {
                        class: "grid grid-cols-2 gap-6 mt-6",
                        div {
                            class: "bg-slate-900 p-6 rounded-xl border border-slate-700 flex flex-col items-center justify-center",
                            div {
                                class: "text-green-500 text-4xl font-bold mb-2",
                                "820"
                            }
                            div {
                                class: "text-green-500 font-semibold",
                                "Riesgo Bajo"
                            }
                        }
                        div {
                            class: "bg-slate-900 p-6 rounded-xl border border-slate-700 flex flex-col items-center justify-center",
                            ul {
                                class: "list-none",
                                li {
                                    class: "text-slate-400 mb-2",
                                    "CFE (Al día)"
                                }
                                li {
                                    class: "text-slate-400 mb-2",
                                    "Agua (Al día)"
                                }
                                li {
                                    class: "text-slate-400 mb-2",
                                    "Telcel (5 días de atraso)"
                                }
                            }
                        }
                    }
                },
                TabState::Servicios => rsx! {
                    div {
                        class: "border-2 border-dashed border-slate-600 rounded-xl p-12 mt-6 flex items-center justify-center",
                        div {
                            class: "text-slate-400 text-lg",
                            "Próximamente"
                        }
                    }
                },
                TabState::IneOcr => rsx! {
                    div {
                        class: "flex flex-col items-center justify-center w-full max-w-2xl h-64 border-2 border-dashed border-slate-600 bg-slate-900/50 rounded-2xl hover:border-blue-500 hover:bg-slate-800/50 transition-all cursor-pointer",
                        div {
                            class: "text-slate-300 text-sm mb-4",
                            "Validación de Identidad (Prevención de Fraude)"
                        }
                        div {
                            class: "flex flex-col items-center justify-center w-full h-full",
                            div {
                                class: "text-slate-400 text-lg",
                                "Arrastra el anverso de la INE aquí o haz clic para explorar"
                            }
                            div {
                                class: "text-slate-400 text-sm mt-2",
                                "Formatos soportados: JPG, PNG, PDF"
                            }
                        }
                        button {
                            class: "bg-blue-600 text-white px-4 py-2 rounded-lg hover:bg-blue-700 transition-all",
                            onclick: move |_| {
                                spawn(async move {
                                    match http_client().post("http://127.0.0.1:3000/api/ocr").send().await {
                                        Ok(response) => {
                                            if let Ok(result) = response.json::<serde_json::Value>().await {
                                                // 1. Extraemos el ID que nos dio el backend y lo guardamos
                                                if let Some(id) = result["id"].as_str() {
                                                    document_id.set(id.to_string());
                                                }
                                                
                                                // 2. Mostramos el mensaje visual (sin confidence score)
                                                ocr_result.set(Some(format!("Status: {}, Extracted Name: {}", result["status"], result["extracted_name"])));
                                            } else {
                                                status_message.set(Some("Error al leer respuesta JSON".to_string()));
                                            }
                                        },
                                        Err(e) => {
                                            status_message.set(Some(format!("Error de conexión OCR: {}", e)));
                                        }
                                    };
                                });
                            },
                            "Iniciar Escaneo OCR"
                        }
                    }
                },
            }
            
            // Action Bar
            div {
                class: "flex justify-end gap-4 mt-8",
                
                button {
                    class: "border border-blue-500 text-blue-500 px-4 py-2 rounded-lg hover:bg-blue-500 hover:text-white",
                    onclick: move |_| {
                        spawn(async move {
                            let id = document_id();
                            if id.is_empty() {
                                status_message.set(Some("No hay ID disponible. Ejecuta OCR primero.".to_string()));
                                return;
                            }
                            if let Ok(res) = http_client().post("http://127.0.0.1:3000/api/update_status")
                                .json(&serde_json::json!({
                                    "id": id,
                                    "estado": "Rechazado"
                                }))
                                .send()
                                .await {
                                match res.status().is_success() {
                                    true => status_message.set(Some("Solicitud Rechazada".to_string())),
                                    false => status_message.set(Some("Error de conexión al servidor".to_string()))
                                }
                            }
                        });
                    },
                    "Rechazar"
                }

                button {
                    class: "bg-green-500 text-white px-4 py-2 rounded-lg hover:bg-green-600",
                    onclick: move |_| {
                        spawn(async move {
                            let id = document_id();
                            if id.is_empty() {
                                status_message.set(Some("No hay ID disponible. Ejecuta OCR primero.".to_string()));
                                return;
                            }
                            if let Ok(res) = http_client().post("http://127.0.0.1:3000/api/update_status")
                                .json(&serde_json::json!({
                                    "id": id,
                                    "estado": "Aprobado"
                                }))
                                .send()
                                .await {
                                match res.status().is_success() {
                                    true => status_message.set(Some("¡Solicitud Aprobada!".to_string())),
                                    false => status_message.set(Some("Error de conexión al servidor".to_string()))
                                }
                            }
                        });
                    },
                    "Aprobar"
                }
            }

            // Render OCR Result (sin confidence score)
            if let Some(result) = ocr_result() {
                div {
                    class: "text-green-400 mt-4 p-4 bg-slate-900 rounded",
                    "{result}"
                }
            }

            // Render Status Message
            if let Some(msg) = status_message() {
                div {
                    class: "text-yellow-400 mt-2 p-4 bg-slate-900 rounded",
                    "{msg}"
                }
            }
        }
    }
}
