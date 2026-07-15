use dioxus::prelude::*;
use std::sync::OnceLock;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| reqwest::Client::new())
}

#[derive(Clone, Copy, PartialEq)]
enum MenuState {
    Dashboard,
    AltaCliente,
    Cartera,
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let is_authenticated = use_signal(|| false);
    let mut current_company = use_signal(|| String::new());
    let mut active_menu = use_signal(|| MenuState::Dashboard);

    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        if is_authenticated() {
            div {
                class: "flex h-screen text-white",
                Sidebar { current_company, active_menu }
                MainArea { current_company, active_menu }
            }
        } else {
            Login { is_authenticated, current_company }
        }
    }
}

#[component]
fn Login(is_authenticated: Signal<bool>, mut current_company: Signal<String>) -> Element {
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
                                    match http_client().post("http://127.0.0.1:3000/api/login")
                                        .json(&body)
                                        .send()
                                        .await
                                    {
                                        Ok(res) => {
                                            match res.json::<serde_json::Value>().await {
                                                Ok(data) => {
                                                    if data["status"] == "success" {
                                                         current_company.set(data["empresa"].as_str().unwrap_or("").to_string());
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
                                match http_client().post("http://127.0.0.1:3000/api/login")
                                    .json(&body)
                                    .send()
                                    .await
                                {
                                    Ok(res) => {
                                        match res.json::<serde_json::Value>().await {
                                            Ok(data) => {
                                                if data["status"] == "success" {
                                                    current_company.set(data["empresa"].as_str().unwrap_or("").to_string());
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
fn Sidebar(current_company: Signal<String>, mut active_menu: Signal<MenuState>) -> Element {
    rsx! {
        div {
            class: "bg-slate-900 w-64 flex flex-col items-center justify-start p-4",
            div { class: "text-blue-500 font-bold text-2xl mb-8", "PYMZA" }
            div { class: "text-slate-400 text-xs mb-6 text-center px-2", "{current_company}" }
            ul { class: "flex flex-col w-full gap-1",
                li {
                    class: format!("p-3 rounded-lg cursor-pointer flex items-center transition-colors {}",
                        if active_menu() == MenuState::Dashboard { "bg-blue-900/50 text-blue-400" } else { "text-slate-400 hover:bg-slate-800 hover:text-white" }
                    ),
                    onclick: move |_| active_menu.set(MenuState::Dashboard),
                    svg { class: "w-5 h-5 mr-3", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                        path { d: "M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" }
                    "Dashboard"
                }
                li {
                    class: format!("p-3 rounded-lg cursor-pointer flex items-center transition-colors {}",
                        if active_menu() == MenuState::AltaCliente { "bg-blue-900/50 text-blue-400" } else { "text-slate-400 hover:bg-slate-800 hover:text-white" }
                    ),
                    onclick: move |_| active_menu.set(MenuState::AltaCliente),
                    svg { class: "w-5 h-5 mr-3", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                        path { d: "M18 9v3m0 0v3m0-3h3m-3 0h-3m-2-5a4 4 0 11-8 0 4 4 0 018 0zM3 20a6 6 0 0112 0v1H3v-1z" }
                    }
                    "Alta de Cliente"
                }
                li {
                    class: format!("p-3 rounded-lg cursor-pointer flex items-center transition-colors {}",
                        if active_menu() == MenuState::Cartera { "bg-blue-900/50 text-blue-400" } else { "text-slate-400 hover:bg-slate-800 hover:text-white" }
                    ),
                    onclick: move |_| active_menu.set(MenuState::Cartera),
                    svg { class: "w-5 h-5 mr-3", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                        path { d: "M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" }
                    }
                    "Cartera"
                }
            }
        }
    }
}

#[component]
fn MainArea(current_company: Signal<String>, active_menu: Signal<MenuState>) -> Element {
    rsx! {
        div {
            class: "bg-slate-800 flex-1 p-8 text-slate-200",

            // Welcome banner
            div {
                class: "bg-gradient-to-r from-blue-900/40 to-slate-800 border border-blue-800/50 rounded-xl p-6 mb-6 flex items-center justify-between",
                div {
                    class: "flex items-center gap-3",
                    svg { class: "w-8 h-8 text-blue-400", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                        path { d: "M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" }
                    }
                    div {
                        div { class: "text-slate-400 text-sm font-medium uppercase tracking-wider", "Panel de Control" }
                        div { class: "text-white text-2xl font-bold", "{current_company}" }
                    }
                }
                div {
                    class: "flex items-center gap-2 bg-slate-900/60 px-4 py-2 rounded-lg border border-slate-700",
                    div { class: "w-2 h-2 rounded-full bg-green-500" }
                    div { class: "text-slate-300 text-sm", "Sesión activa" }
                }
            }

            match active_menu() {
                MenuState::Dashboard => rsx! {
                    div { class: "grid grid-cols-3 gap-6",
                        div { class: "bg-slate-900 rounded-xl border border-slate-700 p-6",
                            div { class: "flex items-center gap-3 mb-4",
                                svg { class: "w-6 h-6 text-blue-400", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                                    path { d: "M17 9V7a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2m2 4h10a2 2 0 002-2v-6a2 2 0 00-2-2H9a2 2 0 00-2 2v6a2 2 0 002 2zm7-5a2 2 0 11-4 0 2 2 0 014 0z" }
                                }
                                div { class: "text-slate-400 text-sm font-medium", "Créditos Activos" }
                            }
                            div { class: "text-3xl font-bold text-white", "0" }
                        }
                        div { class: "bg-slate-900 rounded-xl border border-slate-700 p-6",
                            div { class: "flex items-center gap-3 mb-4",
                                svg { class: "w-6 h-6 text-green-400", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                                    path { d: "M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" }
                                }
                                div { class: "text-slate-400 text-sm font-medium", "Capital Prestado" }
                            }
                            div { class: "text-3xl font-bold text-white", "$0 MXN" }
                        }
                        div { class: "bg-slate-900 rounded-xl border border-slate-700 p-6",
                            div { class: "flex items-center gap-3 mb-4",
                                svg { class: "w-6 h-6 text-yellow-400", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                                    path { d: "M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" }
                                }
                                div { class: "text-slate-400 text-sm font-medium", "Próximos Cobros" }
                            }
                            div { class: "text-3xl font-bold text-white", "0" }
                        }
                    }
                },
                MenuState::AltaCliente => rsx! {
                    div {
                        div { class: "text-xl font-bold text-white mb-6", "Validación de Nuevo Cliente" }
                        div { class: "bg-slate-900 rounded-xl border border-slate-700 p-6 max-w-lg",
                            div { class: "text-slate-400 text-sm mb-4",
                                "Ingresa el CURP o ID del cliente para verificar si ya existe en el ecosistema Red PYMZA."
                            }
                            div { class: "flex gap-3",
                                input {
                                    class: "flex-1 bg-slate-800 border border-slate-600 text-slate-200 placeholder-slate-500 rounded-lg px-4 py-3 outline-none focus:border-blue-500 transition-colors",
                                    placeholder: "CURP o ID del Cliente",
                                }
                                button {
                                    class: "bg-blue-600 hover:bg-blue-700 text-white font-semibold px-6 py-3 rounded-lg transition-colors whitespace-nowrap",
                                    "Buscar en Red PYMZA"
                                }
                            }
                        }
                    }
                },
                MenuState::Cartera => rsx! {
                    div {
                        div { class: "text-xl font-bold text-white mb-6", "Cartera de Clientes" }
                        div { class: "bg-slate-900 rounded-xl border border-slate-700 overflow-hidden",
                            table { class: "w-full text-left",
                                thead {
                                    tr { class: "border-b border-slate-700 bg-slate-800/50",
                                        th { class: "px-6 py-4 text-slate-400 text-sm font-medium uppercase tracking-wider", "Nombre del Cliente" }
                                        th { class: "px-6 py-4 text-slate-400 text-sm font-medium uppercase tracking-wider", "Producto" }
                                        th { class: "px-6 py-4 text-slate-400 text-sm font-medium uppercase tracking-wider", "Monto" }
                                        th { class: "px-6 py-4 text-slate-400 text-sm font-medium uppercase tracking-wider", "Estado" }
                                        th { class: "px-6 py-4 text-slate-400 text-sm font-medium uppercase tracking-wider", "Acciones" }
                                    }
                                }
                                tbody {
                                    tr {
                                        class: "border-b border-slate-700",
                                        td { colspan: "5", class: "px-6 py-8 text-center text-slate-500", "No hay clientes registrados" }
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}
