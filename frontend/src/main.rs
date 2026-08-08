use dioxus::prelude::*;
use std::sync::OnceLock;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
const API_BASE: &str = "http://127.0.0.1:3000";

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

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
struct DashboardStats {
    empresa: String,
    creditos_activos: i32,
    capital_prestado: f64,
    proximos_cobros: i32,
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
struct PagoInfo {
    mes: i32,
    pago: f64,
    interes: f64,
    capital: f64,
    saldo_restante: f64,
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let is_authenticated = use_signal(|| false);
    let current_company = use_signal(|| String::new());
    let active_menu = use_signal(|| MenuState::Dashboard);

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
                                    match http_client().post(format!("{API_BASE}/api/login"))
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
                                match http_client().post(format!("{API_BASE}/api/login"))
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
                    }
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
    let mut curp_input = use_signal(|| String::new());
    let mut search_result = use_signal(|| None::<serde_json::Value>);
    let mut search_status = use_signal(|| String::new());

    let mut show_plan_modal = use_signal(|| false);
    let mut plan_producto = use_signal(|| String::new());
    let mut plan_monto = use_signal(|| String::new());
    let mut plan_plazo = use_signal(|| "3".to_string());
    let mut modal_step = use_signal(|| 0u8);
    let mut eval_estado = use_signal(|| String::new());
    let mut eval_pago_mensual = use_signal(|| 0.0);
    let mut eval_tasa_interes = use_signal(|| 0.0);
    let mut eval_plan_pagos = use_signal(|| Vec::<PagoInfo>::new());
    let mut consideraciones = use_signal(|| String::new());
    let mut terms_accepted = use_signal(|| false);

    let mut alta_nombre = use_signal(|| String::new());
    let mut alta_direccion = use_signal(|| String::new());
    let mut alta_telefono = use_signal(|| String::new());

    let mut dashboard_stats = use_signal(|| DashboardStats::default());
    let mut stats_loaded = use_signal(|| false);

    let mut cartera_planes = use_signal(|| Vec::<serde_json::Value>::new());
    let mut cartera_loaded = use_signal(|| false);

    // Fetch dashboard stats when empresa changes or after authorization
    if !stats_loaded() && !current_company().is_empty() {
        let empresa = current_company();
        stats_loaded.set(true);
        spawn(async move {
            match http_client()
                .get(&format!("{API_BASE}/api/dashboard/{}", empresa))
                .send()
                .await
            {
                Ok(res) => {
                    if let Ok(data) = res.json::<serde_json::Value>().await {
                        if data["status"] == "success" {
                            if let Some(s) = data.get("stats") {
                                let e = s["empresa"].as_str().unwrap_or("").to_string();
                                let ca = s["creditos_activos"].as_i64().unwrap_or(0) as i32;
                                let cp = s["capital_prestado"].as_f64().unwrap_or(0.0);
                                let pc = s["proximos_cobros"].as_i64().unwrap_or(0) as i32;
                                dashboard_stats.set(DashboardStats {
                                    empresa: e, creditos_activos: ca,
                                    capital_prestado: cp, proximos_cobros: pc,
                                });
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        });
    }

    // Fetch cartera (planes activos) cuando cambia la empresa
    if !cartera_loaded() && !current_company().is_empty() {
        let empresa = current_company();
        cartera_loaded.set(true);
        spawn(async move {
            match http_client()
                .get(&format!("{API_BASE}/api/creditos/{}", empresa))
                .send()
                .await
            {
                Ok(res) => {
                    if let Ok(data) = res.json::<serde_json::Value>().await {
                        if data["status"] == "success" {
                            if let Some(arr) = data["creditos"].as_array() {
                                cartera_planes.set(arr.clone());
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        });
    }

    rsx! {
        div {
            class: "bg-slate-800 flex-1 p-8 text-slate-200",

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
                MenuState::Dashboard => {
                    let stats = dashboard_stats();
                    rsx! { div { class: "grid grid-cols-3 gap-6",
                        div { class: "bg-slate-900 rounded-xl border border-slate-700 p-6",
                            div { class: "flex items-center gap-3 mb-4",
                                svg { class: "w-6 h-6 text-blue-400", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                                    path { d: "M17 9V7a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2m2 4h10a2 2 0 002-2v-6a2 2 0 00-2-2H9a2 2 0 00-2 2v6a2 2 0 002 2zm7-5a2 2 0 11-4 0 2 2 0 014 0z" }
                                }
                                div { class: "text-slate-400 text-sm font-medium", "Créditos Activos" }
                            }
                            div { class: "text-3xl font-bold text-white", "{stats.creditos_activos}" }
                        }
                        div { class: "bg-slate-900 rounded-xl border border-slate-700 p-6",
                            div { class: "flex items-center gap-3 mb-4",
                                svg { class: "w-6 h-6 text-green-400", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                                    path { d: "M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" }
                                }
                                div { class: "text-slate-400 text-sm font-medium", "Capital Prestado" }
                            }
                            div { class: "text-3xl font-bold text-white", "${stats.capital_prestado} MXN" }
                        }
                        div { class: "bg-slate-900 rounded-xl border border-slate-700 p-6",
                            div { class: "flex items-center gap-3 mb-4",
                                svg { class: "w-6 h-6 text-yellow-400", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                                    path { d: "M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" }
                                }
                                div { class: "text-slate-400 text-sm font-medium", "Próximos Cobros" }
                            }
                            div { class: "text-3xl font-bold text-white", "{stats.proximos_cobros}" }
                        }
                    }
                    }
                },
                MenuState::AltaCliente => {
                    let status = search_status();
                    rsx! {
                        div {
                            h2 { class: "text-2xl font-bold mb-6 text-white", "Validación de Nuevo Cliente" }
                            div { class: "flex gap-3 mb-6",
                                input {
                                    class: "flex-1 bg-slate-900 border border-slate-600 text-slate-200 placeholder-slate-500 rounded-lg px-4 py-3 outline-none focus:border-blue-500 transition-colors",
                                    placeholder: "CURP o ID del Cliente",
                                    value: "{curp_input}",
                                    oninput: move |e| curp_input.set(e.value()),
                                }
                                button {
                                    class: "bg-blue-600 hover:bg-blue-700 text-white font-semibold px-6 py-3 rounded-lg transition-colors whitespace-nowrap",
                                    onclick: move |_| {
                                        let curp = curp_input();
                                        search_result.set(None);
                                        search_status.set(String::new());
                                        show_plan_modal.set(false);
                                        spawn(async move {
                                            match http_client()
                                                .get(&format!("{API_BASE}/api/clientes/{}", curp))
                                                .send()
                                                .await
                                            {
                                                Ok(res) => {
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
                                    class: "bg-green-900/20 border border-green-700/50 rounded-xl p-6 max-w-lg",
                                    div { class: "flex items-center gap-2 mb-4",
                                        div { class: "text-green-400 font-semibold", "¡Cliente encontrado en Red PYMZA!" }
                                    }
                                    div { class: "grid grid-cols-2 gap-4",
                                        div {
                                            div { class: "text-slate-400 text-xs uppercase mb-1", "Nombre Completo" }
                                            div { class: "text-white font-medium", "{cliente[\"nombre_completo\"].as_str().unwrap_or(\"—\")}" }
                                        }
                                        div {
                                            div { class: "text-slate-400 text-xs uppercase mb-1", "Score Crediticio" }
                                            div { class: "text-3xl font-bold text-green-400", "{cliente[\"score\"].as_i64().unwrap_or(0)}" }
                                        }
                                        div {
                                            div { class: "text-slate-400 text-xs uppercase mb-1", "Nivel de Riesgo" }
                                            div { class: "text-white font-medium", "{cliente[\"nivel_riesgo\"].as_str().unwrap_or(\"—\")}" }
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
                                        class: "bg-yellow-900/20 border border-yellow-700/50 rounded-xl p-6 max-w-lg",
                                        h3 { class: "text-yellow-400 font-semibold mb-4", "Cliente no encontrado — Alta de Cliente" }
                                        div { class: "flex flex-col gap-4",
                                            input {
                                                class: "bg-slate-800 border border-slate-600 text-white rounded-lg px-3 py-2 outline-none focus:border-blue-500",
                                                placeholder: "Nombre Completo",
                                                value: alta_nombre(),
                                                oninput: move |e| alta_nombre.set(e.value()),
                                            }
                                            input {
                                                class: "bg-slate-800 border border-slate-600 text-white rounded-lg px-3 py-2 outline-none focus:border-blue-500",
                                                placeholder: "Dirección",
                                                value: alta_direccion(),
                                                oninput: move |e| alta_direccion.set(e.value()),
                                            }
                                            input {
                                                class: "bg-slate-800 border border-slate-600 text-white rounded-lg px-3 py-2 outline-none focus:border-blue-500",
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
                                                    spawn(async move {
                                                        let body = serde_json::json!({
                                                            "curp": curp,
                                                            "nombre_completo": nombre,
                                                            "direccion": direccion,
                                                            "telefono": telefono,
                                                        });
                                                        match http_client().post(format!("{API_BASE}/api/clientes"))
                                                            .json(&body)
                                                            .send()
                                                            .await
                                                        {
                                                            Ok(res) => {
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
                                        class: "bg-yellow-900/20 border border-yellow-700/50 rounded-xl p-6 max-w-lg",
                                        div { class: "text-yellow-400 font-semibold mb-4", "{status}" }
                                    }
                                }
                            }

                            if show_plan_modal() {
                                div { class: "fixed inset-0 bg-black/80 flex items-center justify-center z-50",
                                    div { class: "bg-slate-900 border border-slate-700 p-6 rounded-xl w-full max-w-2xl",
                                        match modal_step() {
                                            0 => rsx! {
                                                h2 { class: "text-xl font-bold text-white mb-4", "Configurar Plan de Pagos" }
                                                div { class: "flex flex-col gap-4",
                                                    label { class: "text-slate-400 text-sm", "Producto o Servicio" }
                                                    input {
                                                        class: "bg-slate-800 border border-slate-600 text-white rounded-lg px-3 py-2 outline-none focus:border-blue-500",
                                                        value: plan_producto(),
                                                        oninput: move |e| plan_producto.set(e.value())
                                                    }

                                                    label { class: "text-slate-400 text-sm", "Monto Total ($)" }
                                                    input {
                                                        class: "bg-slate-800 border border-slate-600 text-white rounded-lg px-3 py-2 outline-none focus:border-blue-500",
                                                        type: "number",
                                                        value: plan_monto(),
                                                        oninput: move |e| plan_monto.set(e.value())
                                                    }

                                                    label { class: "text-slate-400 text-sm", "Plazo (Meses)" }
                                                    select {
                                                        class: "bg-slate-800 border border-slate-600 text-white rounded-lg px-3 py-2 outline-none focus:border-blue-500",
                                                        value: plan_plazo(),
                                                        onchange: move |e| plan_plazo.set(e.value()),
                                                        option { value: "3", "3 meses — Tasa 3%" }
                                                        option { value: "6", "6 meses — Tasa 6%" }
                                                        option { value: "9", "9 meses — Tasa 10%" }
                                                        option { value: "12", "12 meses — Tasa 15%" }
                                                    }
                                                }
                                                div { class: "flex justify-end gap-3 mt-6",
                                                    button {
                                                        class: "bg-slate-700 hover:bg-slate-600 text-white font-semibold px-4 py-2 rounded-lg",
                                                        onclick: move |_| { show_plan_modal.set(false); modal_step.set(0); },
                                                        "Cancelar"
                                                    }
                                                    button {
                                                        class: "bg-blue-600 hover:bg-blue-700 text-white font-semibold px-4 py-2 rounded-lg",
                                                        onclick: move |_| {
                                                            let curp = curp_input();
                                                            let monto_str = plan_monto();
                                                            let plazo = plan_plazo();
                                                            modal_step.set(1);
                                                            spawn(async move {
                                                                let monto_num: f64 = monto_str.parse().unwrap_or(0.0);
                                                                let plazo_num: i32 = plazo.parse().unwrap_or(3);
                                                                let body = serde_json::json!({
                                                                    "curp": curp,
                                                                    "monto": monto_num,
                                                                    "plazo_meses": plazo_num,
                                                                });
                                                                match http_client().post(format!("{API_BASE}/api/creditos/evaluar"))
                                                                    .json(&body)
                                                                    .send()
                                                                    .await
                                                                {
                                                                    Ok(res) => {
                                                                        match res.json::<serde_json::Value>().await {
                                                                            Ok(data) => {
                                                                                eval_estado.set(data["estado"].as_str().unwrap_or("").to_string());
                                                                                eval_pago_mensual.set(data["pago_mensual"].as_f64().unwrap_or(0.0));
                                                                                eval_tasa_interes.set(data["tasa_interes"].as_f64().unwrap_or(0.0));
                                                                                consideraciones.set(data["consideraciones"].as_str().unwrap_or("").to_string());
                                                                                if let Some(plan) = data["plan_pagos"].as_array() {
                                                                                    let planes: Vec<PagoInfo> = plan.iter().map(|p| PagoInfo {
                                                                                        mes: p["mes"].as_i64().unwrap_or(0) as i32,
                                                                                        pago: p["pago"].as_f64().unwrap_or(0.0),
                                                                                        interes: p["interes"].as_f64().unwrap_or(0.0),
                                                                                        capital: p["capital"].as_f64().unwrap_or(0.0),
                                                                                        saldo_restante: p["saldo_restante"].as_f64().unwrap_or(0.0),
                                                                                    }).collect();
                                                                                    eval_plan_pagos.set(planes);
                                                                                }
                                                                            }
                                                                            Err(_) => {
                                                                                eval_estado.set("Error".to_string());
                                                                                consideraciones.set("Error al procesar la evaluación".to_string());
                                                                            }
                                                                        }
                                                                    }
                                                                    Err(_) => {
                                                                        eval_estado.set("Error".to_string());
                                                                        consideraciones.set("Error de conexión con el servidor".to_string());
                                                                    }
                                                                }
                                                                modal_step.set(2);
                                                            });
                                                        },
                                                        "Evaluar Crédito"
                                                    }
                                                }
                                            },
                                            1 => rsx! {
                                                div { class: "flex flex-col items-center justify-center py-12",
                                                    div { class: "text-blue-400 text-lg font-semibold animate-pulse", "Evaluando riesgo en Red PYMZA..." }
                                                }
                                            },
                                            2 => {
                                                let planes = eval_plan_pagos();
                                                rsx! {
                                                h2 { class: "text-xl font-bold text-white mb-4", "Resultado de Evaluación" }
                                                div { class: "flex flex-col gap-4",
                                                    div {
                                                        class: format!("text-lg font-bold {}", if eval_estado() == "Aprobado" { "text-green-400" } else { "text-red-400" }),
                                                        "{eval_estado}"
                                                    }

                                                    div { class: "bg-slate-800 border border-slate-600 rounded-lg p-4",
                                                        div { class: "grid grid-cols-2 gap-4",
                                                            div {
                                                                div { class: "text-slate-400 text-xs uppercase mb-1", "Mensualidad Estimada" }
                                                                div { class: "text-2xl font-bold text-white", "${eval_pago_mensual} MXN" }
                                                            }
                                                            div {
                                                                div { class: "text-slate-400 text-xs uppercase mb-1", "Tasa de Interés" }
                                                                div { class: "text-xl font-bold text-yellow-400", "{(eval_tasa_interes() * 100.0) as i32}%" }
                                                            }
                                                        }
                                                    }

                                                    div { class: "bg-slate-800 border border-slate-600 rounded-lg p-4",
                                                        div { class: "text-slate-400 text-sm mb-2 font-semibold", "Plan de Pagos" }
                                                        div { class: "overflow-x-auto max-h-48 overflow-y-auto",
                                                            table { class: "w-full text-sm text-left",
                                                                thead {
                                                                    tr { class: "text-slate-400 border-b border-slate-700",
                                                                        th { class: "py-2 px-2", "Mes" }
                                                                        th { class: "py-2 px-2", "Pago" }
                                                                        th { class: "py-2 px-2", "Interés" }
                                                                        th { class: "py-2 px-2", "Capital" }
                                                                        th { class: "py-2 px-2", "Saldo Restante" }
                                                                    }
                                                                }
                                                                tbody {
                                                                    for p in &planes {
                                                                        tr { class: "border-b border-slate-700/50 text-slate-300",
                                                                            td { class: "py-2 px-2", "{p.mes}" }
                                                                            td { class: "py-2 px-2", "${p.pago}" }
                                                                            td { class: "py-2 px-2", "${p.interes}" }
                                                                            td { class: "py-2 px-2", "${p.capital}" }
                                                                            td { class: "py-2 px-2", "${p.saldo_restante}" }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }

                                                    div { class: "bg-slate-800 border border-slate-600 rounded-lg p-4 max-h-32 overflow-y-auto",
                                                        div { class: "text-slate-400 text-sm mb-2", "Consideraciones" }
                                                        div { class: "text-slate-200 text-sm whitespace-pre-wrap", "{consideraciones}" }
                                                    }

                                                    label { class: "flex items-start gap-2 text-slate-300 text-sm cursor-pointer",
                                                        input {
                                                            type: "checkbox",
                                                            class: "mt-1",
                                                            checked: terms_accepted(),
                                                            oninput: move |e| terms_accepted.set(e.checked()),
                                                        }
                                                        "He leído las consideraciones y asumo el riesgo de autorizar este crédito."
                                                    }
                                                }
                                                div { class: "flex justify-end gap-3 mt-6",
                                                    button {
                                                        class: "bg-slate-700 hover:bg-slate-600 text-white font-semibold px-4 py-2 rounded-lg",
                                                        onclick: move |_| { show_plan_modal.set(false); modal_step.set(0); },
                                                        "Cerrar"
                                                    }
                                                    button {
                                                        class: format!("text-white font-semibold px-4 py-2 rounded-lg {}", if terms_accepted() { "bg-green-600 hover:bg-green-700" } else { "bg-gray-600 cursor-not-allowed" }),
                                                        disabled: !terms_accepted(),
                                                        onclick: move |_| {
                                                            let empresa = current_company();
                                                            let cliente_curp = curp_input();
                                                            let producto = plan_producto();
                                                            let monto_total = plan_monto();
                                                            let plazo = plan_plazo();
                                                            let pago_mensual = eval_pago_mensual();
                                                            let tasa = eval_tasa_interes();
                                                            let _plan_pagos_snapshot = eval_plan_pagos();
                                                            spawn(async move {
                                                                let monto_num: f64 = monto_total.parse().unwrap_or(0.0);
                                                                let plazo_num: i32 = plazo.parse().unwrap_or(3);
                                                                let body = serde_json::json!({
                                                                    "empresa": empresa,
                                                                    "cliente_curp": cliente_curp,
                                                                    "producto": producto,
                                                                    "monto_total": monto_num,
                                                                    "plazo_meses": plazo_num,
                                                                    "pago_mensual": pago_mensual,
                                                                    "tasa_interes": tasa,
                                                                });
                                                                if let Ok(res) = http_client().post(format!("{API_BASE}/api/creditos/autorizar"))
                                                                    .json(&body)
                                                                    .send()
                                                                    .await
                                                                {
                                                                    if let Ok(data) = res.json::<serde_json::Value>().await {
                                                                        if data["status"] == "success" {
                                                                            // Update local dashboard stats
                                                                            show_plan_modal.set(false);
                                                                            modal_step.set(0);
                                                                            cartera_loaded.set(false);
                                                                            plan_producto.set(String::new());
                                                                            plan_monto.set(String::new());
                                                                            plan_plazo.set("3".to_string());
                                                                            terms_accepted.set(false);
                                                                            // Refresh dashboard
                                                                            if let Ok(stats_res) = http_client()
                                                                                .get(&format!("{API_BASE}/api/dashboard/{}", empresa))
                                                                                .send()
                                                                                .await
                                                                            {
                                                                                if let Ok(stats_data) = stats_res.json::<serde_json::Value>().await {
                                                                                    if let Some(s) = stats_data.get("stats") {
                                                                                        let e = s["empresa"].as_str().unwrap_or("").to_string();
                                                                                        let ca = s["creditos_activos"].as_i64().unwrap_or(0) as i32;
                                                                                        let cp = s["capital_prestado"].as_f64().unwrap_or(0.0);
                                                                                        let pc = s["proximos_cobros"].as_i64().unwrap_or(0) as i32;
                                                                                        dashboard_stats.set(DashboardStats {
                                                                                            empresa: e, creditos_activos: ca,
                                                                                            capital_prestado: cp, proximos_cobros: pc,
                                                                                        });
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            });
                                                        },
                                                        "Autorizar Crédito"
                                                    }
                                                }
                                                }
                                            },
                                            _ => rsx! {},
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                MenuState::Cartera => {
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
                },
            }
        }
    }
}
