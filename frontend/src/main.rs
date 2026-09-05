//! Punto de entrada: monta la app, restaura la sesión/tema guardados y enruta
//! entre portal público (`VistaPublica`) y la app autenticada (`MenuState`),
//! sin router (ponytail: 6 pantallas no justifican router; techo: router
//! cuando existan URLs públicas reales tras desplegar el portal, ola 4).

mod api;
mod components;

use dioxus::prelude::*;

use crate::api::{authed_request, evaluar_restauracion, theme_aplicar, theme_leer, token_borrar, token_leer};
use crate::components::alta_cliente::AltaCliente;
use crate::components::cartera::Cartera;
use crate::components::dashboard::Dashboard;
use crate::components::landing::Landing;
use crate::components::login::Login;
use crate::components::registro::Registro;
use crate::components::sidebar::Sidebar;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
const FAVICON: Asset = asset!("/assets/favicon.svg");

/// Pantallas del portal público (sin auth; render condicional).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum VistaPublica {
    Landing,
    Login,
    Registro,
}

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
    let vista_publica = use_signal(|| VistaPublica::Landing);
    let mut theme = use_signal(|| "dark".to_string());
    use_context_provider(|| theme);

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

    // Tema: restaurar preferencia guardada (default dark = look actual).
    use_effect(move || {
        spawn(async move {
            if let Some(guardado) = theme_leer().await {
                theme.set(guardado);
            }
        });
    });

    // Aplicar la clase `dark` en <html> cada vez que cambie el tema.
    use_effect(move || {
        theme_aplicar(&theme());
    });

    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        document::Link { rel: "icon", href: FAVICON }
        if is_authenticated() {
            div {
                class: "flex h-screen bg-slate-100 text-slate-900 dark:bg-slate-800 dark:text-white",
                Sidebar { current_company, active_menu, is_authenticated, token, vista_publica }
                div {
                    class: "flex-1 p-8 overflow-y-auto",
                    div {
                        class: "bg-white border border-slate-200 rounded-xl p-6 mb-6 flex items-center justify-between dark:bg-gradient-to-r dark:from-blue-900/40 dark:to-slate-800 dark:border-blue-800/50",
                        div {
                            class: "flex items-center gap-3",
                            svg { class: "w-8 h-8 text-blue-600 dark:text-blue-400", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                                path { d: "M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" }
                            }
                            div {
                                div { class: "text-slate-500 text-sm font-medium uppercase tracking-wider dark:text-slate-400", "Panel de Control" }
                                div { class: "text-slate-900 text-2xl font-bold dark:text-white", "{current_company}" }
                            }
                        }
                        div {
                            class: "flex items-center gap-2 bg-slate-100 px-4 py-2 rounded-lg border border-slate-200 dark:bg-slate-900/60 dark:border-slate-700",
                            div { class: "w-2 h-2 rounded-full bg-green-500" }
                            div { class: "text-slate-600 text-sm dark:text-slate-300", "Sesión activa" }
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
            match vista_publica() {
                VistaPublica::Landing => rsx! { Landing { vista_publica } },
                VistaPublica::Login => rsx! { Login { vista_publica, is_authenticated, current_company, token } },
                VistaPublica::Registro => rsx! { Registro { vista_publica, is_authenticated, current_company, token } },
            }
        }
    }
}