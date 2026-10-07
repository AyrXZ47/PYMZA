//! Campanita de novedades ("what's new", contrato API ola 8): compara la
//! versión de `GET /api/novedades` con la compilada, avisa con banner si hay
//! actualización y con badge si hay changelog sin ver; al abrir, modal.
//!
//! Si el endpoint falla, la app no se rompe: sin datos, sin campanita.

use dioxus::prelude::*;

use crate::api::{
    novedades_vistas_guardar, novedades_vistas_leer, obtener_novedades, recargar_pagina,
    version_es_mayor, Novedades, APP_VERSION,
};

#[component]
pub fn CampanitaNovedades() -> Element {
    // `None` mientras carga o si la petición falla (la app sigue igual).
    let datos = use_resource(move || async move { obtener_novedades().await.ok() });
    // Última versión vista (localStorage) y si ya se leyó (para no parpadear).
    let mut version_vista = use_signal(|| Option::<String>::None);
    let mut vistas_leidas = use_signal(|| false);
    let mut modal_abierto = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            version_vista.set(novedades_vistas_leer().await);
            vistas_leidas.set(true);
        });
    });

    let info: Option<Novedades> = datos().flatten();
    let version_servidor = info.as_ref().map(|n| n.version.clone()).unwrap_or_default();
    let hay_actualizacion = info
        .as_ref()
        .map(|n| version_es_mayor(&n.version, APP_VERSION))
        .unwrap_or(false);
    let hay_badge = vistas_leidas()
        && !version_servidor.is_empty()
        && version_vista().as_deref() != Some(version_servidor.as_str());

    rsx! {
        if hay_actualizacion {
            button {
                class: "mb-4 w-full rounded-lg border border-amber-300 bg-amber-50 p-2 text-left text-xs font-medium text-amber-800 hover:bg-amber-100 dark:border-amber-700/60 dark:bg-amber-900/20 dark:text-amber-200",
                onclick: move |_| recargar_pagina(),
                "⬆ Hay una actualización disponible — recarga la página"
            }
        }
        button {
            class: "relative mb-6 flex w-full items-center rounded-lg p-3 text-slate-600 transition-colors hover:bg-slate-100 hover:text-slate-900 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-white",
            onclick: move |_| {
                if !version_servidor.is_empty() {
                    novedades_vistas_guardar(&version_servidor);
                    version_vista.set(Some(version_servidor.clone()));
                }
                modal_abierto.set(true);
            },
            svg { class: "mr-3 h-5 w-5", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                path { d: "M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" }
            }
            "Novedades"
            if hay_badge {
                span {
                    class: "absolute right-3 top-1/2 -translate-y-1/2 rounded-full bg-blue-600 px-2 py-0.5 text-[10px] font-bold text-white",
                    "nuevo"
                }
            }
        }
        if modal_abierto() {
            div {
                class: "fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4",
                onclick: move |_| modal_abierto.set(false),
                div {
                    class: "max-h-[80vh] w-full max-w-lg overflow-y-auto rounded-xl border border-slate-200 bg-white p-6 dark:border-slate-700 dark:bg-slate-900",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "mb-4 flex items-center justify-between",
                        div { class: "text-lg font-bold text-slate-900 dark:text-white", "Novedades" }
                        button {
                            class: "rounded-lg px-2 text-xl text-slate-400 hover:text-slate-700 dark:hover:text-slate-200",
                            onclick: move |_| modal_abierto.set(false),
                            "×"
                        }
                    }
                    if let Some(n) = info.clone() {
                        if n.novedades.is_empty() {
                            div { class: "text-sm text-slate-500 dark:text-slate-400", "Sin novedades registradas." }
                        }
                        for item in n.novedades {
                            div { class: "mb-4 border-b border-slate-100 pb-3 dark:border-slate-800",
                                div { class: "text-xs text-slate-400 dark:text-slate-500", "{item.fecha}" }
                                div { class: "font-medium text-slate-900 dark:text-white", "{item.titulo}" }
                                div { class: "text-sm text-slate-600 dark:text-slate-300", "{item.detalle}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
