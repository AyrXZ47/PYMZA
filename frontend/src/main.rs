//! Punto de entrada: monta la app, restaura la sesión guardada y enruta
//! entre las pantallas según `MenuState` (sin router).

mod api;
mod components;

use dioxus::prelude::*;

use crate::api::{authed_request, evaluar_restauracion, token_borrar, token_leer};
use crate::components::alta_cliente::AltaCliente;
use crate::components::cartera::Cartera;
use crate::components::dashboard::Dashboard;
use crate::components::login::Login;
use crate::components::sidebar::Sidebar;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

/// Pantallas de la app tras autenticar (sin router; render condicional).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum MenuState {
    Dashboard,
    AltaCliente,
    Cartera,
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut is_authenticated = use_signal(|| false);
    let mut current_company = use_signal(|| String::new());
    let active_menu = use_signal(|| MenuState::Dashboard);
    let mut token = use_signal(|| String::new());

    // Restaurar sesión desde localStorage. use_effect: localStorage solo existe
    // tras la hidratación. Red caída o respuesta rara → sin sesión esta carga;
    // el token guardado sobrevive para reintentar.
    use_effect(move || {
        spawn(async move {
            let Some(saved) = token_leer().await else { return };
            match authed_request(reqwest::Method::GET, "/api/dashboard".to_string(), &saved)
                .send()
                .await
            {
                Ok(res) => {
                    let status = res.status();
                    if status == reqwest::StatusCode::UNAUTHORIZED {
                        token_borrar();
                        return;
                    }
                    if let Ok(data) = res.json::<serde_json::Value>().await {
                        if let Some(empresa) = evaluar_restauracion(status, &data) {
                            token.set(saved.clone());
                            current_company.set(empresa);
                            is_authenticated.set(true);
                        }
                    }
                }
                Err(_) => {}
            }
        });
    });

    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        if is_authenticated() {
            div {
                class: "flex h-screen text-white",
                Sidebar { current_company, active_menu, is_authenticated, token }
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
                        MenuState::Dashboard => rsx! { Dashboard { token, is_authenticated } },
                        MenuState::AltaCliente => rsx! { AltaCliente { token, is_authenticated } },
                        MenuState::Cartera => rsx! { Cartera { token, is_authenticated } },
                    }
                }
            }
        } else {
            Login { is_authenticated, current_company, token }
        }
    }
}