//! Botón de tema 🌙/☀️ compartido (landing y sidebar). Lee/escribe el signal
//! de tema del contexto, persiste en localStorage y deja que el `use_effect`
//! de App aplique la clase `dark` en <html>.

use dioxus::prelude::*;

use crate::api::{theme_guardar, theme_invertir};

#[component]
pub fn ThemeToggle() -> Element {
    let mut theme = use_context::<Signal<String>>();
    rsx! {
        button {
            class: "px-3 py-2 rounded-lg border border-slate-300 text-slate-600 hover:bg-slate-100 transition-colors dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800",
            title: "Cambiar tema claro/oscuro",
            onclick: move |_| {
                let nuevo = theme_invertir(&theme());
                theme_guardar(nuevo);
                theme.set(nuevo.to_string());
            },
            {
                if theme() == "dark" { "🌙" } else { "☀️" }
            }
        }
    }
}