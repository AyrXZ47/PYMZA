//! Barra lateral de navegación y cierre de sesión.

use dioxus::prelude::*;

use crate::api::token_borrar;
use crate::MenuState;

#[component]
pub fn Sidebar(
    mut current_company: Signal<String>,
    mut active_menu: Signal<MenuState>,
    mut is_authenticated: Signal<bool>,
    mut token: Signal<String>,
) -> Element {
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
            button {
                class: "mt-auto w-full p-3 rounded-lg flex items-center justify-center gap-2 text-red-400 hover:bg-red-900/20 hover:text-red-300 transition-colors",
                onclick: move |_| {
                    token_borrar();
                    is_authenticated.set(false);
                    current_company.set(String::new());
                    active_menu.set(MenuState::Dashboard);
                    token.set(String::new());
                },
                svg { class: "w-5 h-5", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                    path { d: "M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" }
                }
                "Cerrar Sesión"
            }
        }
    }
}