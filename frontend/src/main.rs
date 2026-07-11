use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use std::sync::OnceLock;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

const SPINNER_CSS: &str = r#"
@keyframes pymza-spin { to { transform: rotate(360deg); } }
@keyframes pymza-pop {
    0% { transform: scale(0); opacity: 0; }
    60% { transform: scale(1.15); }
    100% { transform: scale(1); opacity: 1; }
}
.pymza-spinner {
    animation: pymza-spin 0.8s linear infinite;
    border: 4px solid rgba(255,255,255,0.2);
    border-top-color: #fff;
    border-radius: 50%;
    width: 48px; height: 48px;
    display: inline-block;
}
.pymza-pop {
    animation: pymza-pop 0.4s ease-out;
}
"#;

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

#[derive(Clone, PartialEq)]
enum ModalState {
    Hidden,
    Loading,
    Success(String),
    Error(String),
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let is_authenticated = use_signal(|| false);

    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        style { {SPINNER_CSS} }
        if is_authenticated() {
            div {
                class: "flex h-screen text-white",
                Sidebar {}
                MainArea {}
            }
        } else {
            Login { is_authenticated }
        }
    }
}

#[component]
fn Login(is_authenticated: Signal<bool>) -> Element {
    let mut correo = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut error_msg = use_signal(|| None::<String>);

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
                                    match reqwest::Client::new().post("http://127.0.0.1:3000/api/login")
                                        .json(&body)
                                        .send()
                                        .await
                                    {
                                        Ok(res) => {
                                            match res.json::<serde_json::Value>().await {
                                                Ok(data) => {
                                                    if data["status"] == "success" {
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
                                match reqwest::Client::new().post("http://127.0.0.1:3000/api/login")
                                    .json(&body)
                                    .send()
                                    .await
                                {
                                    Ok(res) => {
                                        match res.json::<serde_json::Value>().await {
                                            Ok(data) => {
                                                if data["status"] == "success" {
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
            }
        }
    }
}

#[component]
fn Sidebar() -> Element {
    rsx! {
        div {
            class: "bg-slate-900 w-64 flex flex-col items-center justify-start p-4",
            div { class: "text-blue-500 font-bold text-2xl mb-8", "PYMZA" }
            ul { class: "flex flex-col",
                li { class: "p-3 rounded-lg hover:bg-slate-800 hover:text-white transition-colors",
                    a { href: "#", class: "flex items-center",
                        svg { class: "w-5 h-5 mr-3" }, "Dashboard" } }
                li { class: "p-3 rounded-lg bg-blue-900/50 text-blue-400 hover:bg-slate-800 hover:text-white transition-colors",
                    a { href: "#", class: "flex items-center",
                        svg { class: "w-5 h-5 mr-3" }, "Evaluación" } }
                li { class: "p-3 rounded-lg hover:bg-slate-800 hover:text-white transition-colors",
                    a { href: "#", class: "flex items-center",
                        svg { class: "w-5 h-5 mr-3" }, "Clientes" } }
                li { class: "p-3 rounded-lg hover:bg-red-900/30 text-red-400 hover:text-white transition-colors",
                    a { href: "#", class: "flex items-center",
                        svg { class: "w-5 h-5 mr-3" }, "Red PYME" } }
            }
        }
    }
}

#[component]
fn MainArea() -> Element {
    let mut active_tab = use_signal(|| TabState::OpenBanking);
    let mut toast: Signal<Option<(String, String)>> = use_signal(|| None);
    let mut document_id = use_signal(|| String::new());
    let mut modal = use_signal(|| ModalState::Hidden);

    // Toast element (computed outside rsx! to avoid macro nesting issues)
    let toast_element = match toast() {
        Some((kind, msg)) => {
            let bg = match kind.as_str() {
                "success" => "bg-green-600",
                "error" => "bg-red-600",
                _ => "bg-blue-600",
            };
            rsx! {
                div {
                    class: "fixed top-4 right-4 z-50 {bg} text-white px-6 py-4 rounded-xl shadow-xl flex items-center gap-3 min-w-[300px]",
                    span { class: "flex-1 text-sm font-medium", "{msg}" }
                    button {
                        class: "text-white/70 hover:text-white text-lg leading-none",
                        onclick: move |_| toast.set(None),
                        "✕"
                    }
                }
            }
        },
        None => rsx! {},
    };

    // Modal element (computed outside rsx!)
    let modal_element = match modal() {
        ModalState::Hidden => rsx! {},
        ModalState::Loading => rsx! {
            div { class: "fixed inset-0 z-40 flex items-center justify-center bg-black/60",
                div { class: "bg-slate-800 rounded-2xl p-8 flex flex-col items-center gap-6 min-w-[320px] shadow-2xl border border-slate-700",
                    div { class: "pymza-spinner" }
                    div { class: "text-white text-lg font-medium", "Procesando solicitud..." }
                }
            }
        },
        ModalState::Success(msg) => rsx! {
            div { class: "fixed inset-0 z-40 flex items-center justify-center bg-black/60",
                div { class: "bg-slate-800 rounded-2xl p-8 flex flex-col items-center gap-6 min-w-[320px] shadow-2xl border border-slate-700",
                    div { class: "pymza-pop w-16 h-16 rounded-full bg-green-500 flex items-center justify-center",
                        svg { class: "w-8 h-8 text-white", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "3",
                            path { d: "M5 13l4 4L19 7" }
                        }
                    }
                    div { class: "text-white text-lg font-medium", "{msg}" }
                    button {
                        class: "bg-green-600 hover:bg-green-700 text-white px-8 py-2 rounded-lg transition-all",
                        onclick: move |_| modal.set(ModalState::Hidden),
                        "OK"
                    }
                }
            }
        },
        ModalState::Error(msg) => rsx! {
            div { class: "fixed inset-0 z-40 flex items-center justify-center bg-black/60",
                div { class: "bg-slate-800 rounded-2xl p-8 flex flex-col items-center gap-6 min-w-[320px] shadow-2xl border border-slate-700",
                    div { class: "pymza-pop w-16 h-16 rounded-full bg-red-500 flex items-center justify-center",
                        svg { class: "w-8 h-8 text-white", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "3",
                            path { d: "M6 18L18 6M6 6l12 12" }
                        }
                    }
                    div { class: "text-white text-lg font-medium", "{msg}" }
                    button {
                        class: "bg-red-600 hover:bg-red-700 text-white px-8 py-2 rounded-lg transition-all",
                        onclick: move |_| modal.set(ModalState::Hidden),
                        "OK"
                    }
                }
            }
        },
    };

    rsx! {
        div {
            class: "bg-slate-800 flex-1 p-8 text-slate-200",

            // Header
            div {
                class: "flex justify-between items-center mb-4",
                div { class: "text-2xl font-bold", "SOLICITUD: Janeth Ramos Zamora" }
                div { class: "bg-yellow-500 w-3 h-3 rounded-full mr-2 inline-block" }
                div { class: "font-semibold text-xl", "Monto Solicitado: $15,000 MXN" }
            }

            // Tabs
            div {
                class: "border-b border-slate-700 mb-6",
                div { class: "flex justify-between items-center",
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

            // Tab content
            match active_tab() {
                TabState::OpenBanking => rsx! {
                    div { class: "grid grid-cols-2 gap-6 mt-6",
                        div { class: "bg-slate-900 p-6 rounded-xl border border-slate-700 flex flex-col items-center justify-center",
                            div { class: "text-green-500 text-4xl font-bold mb-2", "820" }
                            div { class: "text-green-500 font-semibold", "Riesgo Bajo" }
                        }
                        div { class: "bg-slate-900 p-6 rounded-xl border border-slate-700 flex flex-col items-center justify-center",
                            ul { class: "list-none",
                                li { class: "text-slate-400 mb-2", "CFE (Al día)" }
                                li { class: "text-slate-400 mb-2", "Agua (Al día)" }
                                li { class: "text-slate-400 mb-2", "Telcel (5 días de atraso)" }
                            }
                        }
                    }
                },
                TabState::Servicios => rsx! {
                    div { class: "border-2 border-dashed border-slate-600 rounded-xl p-12 mt-6 flex items-center justify-center",
                        div { class: "text-slate-400 text-lg", "Próximamente" }
                    }
                },
                TabState::IneOcr => rsx! {
                    div {
                        class: "flex flex-col items-center justify-center w-full max-w-2xl h-64 border-2 border-dashed border-slate-600 bg-slate-900/50 rounded-2xl hover:border-blue-500 hover:bg-slate-800/50 transition-all cursor-pointer",
                        div { class: "text-slate-300 text-sm mb-4", "Validación de Identidad (Prevención de Fraude)" }
                        div { class: "flex flex-col items-center justify-center w-full h-full",
                            div { class: "text-slate-400 text-lg", "Arrastra el anverso de la INE aquí o haz clic para explorar" }
                            div { class: "text-slate-400 text-sm mt-2", "Formatos soportados: JPG, PNG, PDF" }
                        }
                        button {
                            class: "bg-blue-600 text-white px-4 py-2 rounded-lg hover:bg-blue-700 transition-all",
                            onclick: move |_| {
                                spawn(async move {
                                    match http_client().post("http://127.0.0.1:3000/api/ocr").send().await {
                                        Ok(response) => {
                                            if let Ok(result) = response.json::<serde_json::Value>().await {
                                                if let Some(id) = result["id"].as_str() {
                                                    document_id.set(id.to_string());
                                                }
                                                let name = result["extracted_name"].as_str().unwrap_or("desconocido");
                                                toast.set(Some(("success".to_string(), format!("OCR completado: {}", name))));
                                                spawn(async move {
                                                    TimeoutFuture::new(12000).await;
                                                    toast.set(None);
                                                });
                                            } else {
                                                toast.set(Some(("error".to_string(), "Error al leer respuesta JSON".to_string())));
                                                spawn(async move {
                                                    TimeoutFuture::new(12000).await;
                                                    toast.set(None);
                                                });
                                            }
                                        },
                                        Err(e) => {
                                            toast.set(Some(("error".to_string(), format!("Error de conexión OCR: {}", e))));
                                            spawn(async move {
                                                TimeoutFuture::new(12000).await;
                                                toast.set(None);
                                            });
                                        }
                                    };
                                });
                            },
                            "Iniciar Escaneo OCR"
                        }
                    }
                },
            }

            // Action bar
            div {
                class: "flex justify-end gap-4 mt-8",

                button {
                    class: "border border-blue-500 text-blue-500 px-4 py-2 rounded-lg hover:bg-blue-500 hover:text-white",
                    onclick: move |_| {
                        let id = document_id();
                        if id.is_empty() {
                            toast.set(Some(("error".to_string(), "No hay ID disponible. Ejecuta OCR primero.".to_string())));
                            spawn(async move {
                                TimeoutFuture::new(12000).await;
                                toast.set(None);
                            });
                            return;
                        }
                        modal.set(ModalState::Loading);
                        spawn(async move {
                            if let Ok(res) = http_client().post("http://127.0.0.1:3000/api/update_status")
                                .json(&serde_json::json!({"id": id, "estado": "Rechazado"}))
                                .send().await {
                                match res.status().is_success() {
                                    true => modal.set(ModalState::Error("Solicitud Rechazada".to_string())),
                                    false => modal.set(ModalState::Error("Error del servidor".to_string())),
                                }
                            } else {
                                modal.set(ModalState::Error("Error de conexión".to_string()));
                            }
                        });
                    },
                    "Rechazar"
                }

                button {
                    class: "bg-green-500 text-white px-4 py-2 rounded-lg hover:bg-green-600",
                    onclick: move |_| {
                        let id = document_id();
                        if id.is_empty() {
                            toast.set(Some(("error".to_string(), "No hay ID disponible. Ejecuta OCR primero.".to_string())));
                            spawn(async move {
                                TimeoutFuture::new(12000).await;
                                toast.set(None);
                            });
                            return;
                        }
                        modal.set(ModalState::Loading);
                        spawn(async move {
                            if let Ok(res) = http_client().post("http://127.0.0.1:3000/api/update_status")
                                .json(&serde_json::json!({"id": id, "estado": "Aprobado"}))
                                .send().await {
                                match res.status().is_success() {
                                    true => modal.set(ModalState::Success("¡Solicitud Aprobada!".to_string())),
                                    false => modal.set(ModalState::Error("Error del servidor".to_string())),
                                }
                            } else {
                                modal.set(ModalState::Error("Error de conexión".to_string()));
                            }
                        });
                    },
                    "Aprobar"
                }
            }

            {toast_element}
            {modal_element}
        }
    }
}
