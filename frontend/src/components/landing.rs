//! Landing pública: la primera impresión vende. Hero con el pitch, beneficios
//! de la red y CTAs a Registro/Login. Sin auth → es lo primero que se ve.

use dioxus::prelude::*;

use crate::VistaPublica;
use crate::components::theme_toggle::ThemeToggle;

#[component]
pub fn Landing(mut vista_publica: Signal<VistaPublica>) -> Element {
    rsx! {
        div {
            class: "min-h-screen bg-white text-slate-900 dark:bg-slate-950 dark:text-slate-100",
            nav {
                class: "flex items-center justify-between px-6 py-5 border-b border-slate-200 dark:border-slate-800",
                div { class: "text-blue-600 dark:text-blue-500 font-bold text-2xl", "PYMZA" }
                div {
                    class: "flex items-center gap-3",
                    ThemeToggle {}
                    button {
                        class: "px-4 py-2 rounded-lg border border-slate-300 text-slate-700 hover:bg-slate-100 transition-colors dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800",
                        onclick: move |_| vista_publica.set(VistaPublica::Login),
                        "Iniciar sesión"
                    }
                    button {
                        class: "px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white font-semibold transition-colors",
                        onclick: move |_| vista_publica.set(VistaPublica::Registro),
                        "Crear cuenta"
                    }
                }
            }
            section {
                class: "max-w-4xl mx-auto px-6 py-20 text-center",
                h1 {
                    class: "text-4xl md:text-5xl font-bold mb-6 leading-tight",
                    "Crédito con cobranza respaldada para tu negocio"
                }
                p {
                    class: "text-lg text-slate-600 dark:text-slate-400 max-w-2xl mx-auto mb-10",
                    "PYMZA evalúa el riesgo de tus clientes con datos que los burós no ven, te avisa antes de que un crédito se quiebre y estructura planes de pago que se cobran solos."
                }
                div { class: "flex flex-col sm:flex-row gap-4 justify-center",
                    button {
                        class: "px-8 py-3 rounded-lg bg-blue-600 hover:bg-blue-700 text-white font-semibold text-lg transition-colors",
                        onclick: move |_| vista_publica.set(VistaPublica::Registro),
                        "Crear cuenta gratis"
                    }
                    button {
                        class: "px-8 py-3 rounded-lg border border-slate-300 text-slate-700 hover:bg-slate-100 transition-colors dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800",
                        onclick: move |_| vista_publica.set(VistaPublica::Login),
                        "Ya tengo cuenta"
                    }
                }
            }
            section {
                class: "max-w-5xl mx-auto px-6 pb-20 grid md:grid-cols-2 gap-6",
                Beneficio {
                    titulo: "Score con datos alternativos",
                    texto: "Construimos el riesgo del cliente con recibos, historial de pagos y comportamiento — no solo buró de crédito.",
                }
                Beneficio {
                    titulo: "Red de alerta temprana",
                    texto: "Si otra empresa de la red reporta morosidad o desaparición de un cliente, lo sabes antes de prestar.",
                }
                Beneficio {
                    titulo: "Planes de pago estructurados",
                    texto: "Evalúa, aprueba y autoriza créditos con tablas de amortización claras por plazo y tasa.",
                }
                Beneficio {
                    titulo: "Cartera y dashboard",
                    texto: "Visualiza capital prestado, créditos activos y próximos cobros de tu negocio en un panel.",
                }
            }
            section {
                class: "bg-slate-50 border-t border-slate-200 dark:bg-slate-900 dark:border-slate-800",
                div { class: "max-w-4xl mx-auto px-6 py-16 text-center",
                    h2 { class: "text-3xl font-bold mb-4", "Tu cartera, bajo control." }
                    p { class: "text-slate-600 dark:text-slate-400 mb-8", "Regístrate en un minuto y otorga tu primer crédito hoy." }
                    button {
                        class: "px-8 py-3 rounded-lg bg-blue-600 hover:bg-blue-700 text-white font-semibold text-lg transition-colors",
                        onclick: move |_| vista_publica.set(VistaPublica::Registro),
                        "Crear cuenta"
                    }
                }
            }
            footer {
                class: "px-6 py-6 text-center text-sm text-slate-500 dark:text-slate-500 border-t border-slate-200 dark:border-slate-800",
                "PYMZA — Perfilación de Crédito y Cobranza para PYMES"
            }
        }
    }
}

#[component]
fn Beneficio(titulo: String, texto: String) -> Element {
    rsx! {
        div {
            class: "bg-white border border-slate-200 rounded-xl p-6 dark:bg-slate-900 dark:border-slate-800",
            h3 { class: "text-lg font-bold mb-2 text-blue-600 dark:text-blue-400", "{titulo}" }
            p { class: "text-slate-600 dark:text-slate-400", "{texto}" }
        }
    }
}