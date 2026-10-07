//! Panel de control: KPIs honestos con selector de periodo (contrato API ola 8)
//! + las 6 gráficas del resumen de cartera en primitivas SVG de charts.rs.
//!
//! El periodo (semana/mes/bimestre/trimestre/semestre) se traduce a
//! `?desde=YYYY-MM-DD&hasta=YYYY-MM-DD` y refetchea dashboard + resumen; así la
//! serie cobrado-vs-por-cobrar y los KPIs dejan de mezclar periodos.

use dioxus::prelude::*;

use crate::api::{
    fecha_hoy, obtener_dashboard_periodo, obtener_resumen, rango_periodo, DashboardStats,
    PresetPeriodo, Resumen,
};
use crate::components::charts::{
    fmt_moneda, semaforo_morosidad, BarraApilada, BarraH, DatoCategoria, DatoMes, Donut, Linea,
};

const VALOR_NORMAL: &str = "text-slate-900 dark:text-white";

#[component]
pub fn Dashboard(token: Signal<String>, is_authenticated: Signal<bool>) -> Element {
    // Preset del selector (default "mes") y fecha local del navegador. `hoy`
    // se lee tras hidratar (Date solo existe en el navegador).
    let mut periodo = use_signal(|| PresetPeriodo::Mes);
    let mut hoy = use_signal(String::new);

    use_effect(move || {
        spawn(async move {
            if let Some(f) = fecha_hoy().await {
                hoy.set(f);
            }
        });
    });

    // KPIs y gráficas leen `periodo`/`hoy` dentro del resource → refetchean al
    // cambiar cualquiera. Nada de señales vivas sobre el await (clippy.toml).
    let stats = use_resource(move || async move {
        let token_val = token();
        let preset = periodo();
        let hoy_val = hoy();
        if hoy_val.is_empty() {
            return Ok(DashboardStats::default());
        }
        let (desde, hasta) = rango_periodo(preset, &hoy_val);
        obtener_dashboard_periodo(&token_val, &desde, &hasta, is_authenticated, token).await
    });

    let resumen = use_resource(move || async move {
        let token_val = token();
        let preset = periodo();
        let hoy_val = hoy();
        if hoy_val.is_empty() {
            return Ok(Resumen::default());
        }
        let (desde, hasta) = rango_periodo(preset, &hoy_val);
        obtener_resumen(&token_val, &desde, &hasta, is_authenticated, token).await
    });

    let st = stats().and_then(|r| r.ok()).unwrap_or_default();
    let rango = rango_periodo(periodo(), &hoy());
    rsx! {
        div { class: "flex flex-col gap-6",
            div { class: "flex flex-wrap items-center gap-2",
                span { class: "mr-1 text-sm text-slate-500 dark:text-slate-400", "Periodo:" }
                for p in PresetPeriodo::TODOS {
                    button {
                        class: format!("rounded-lg px-3 py-1.5 text-sm font-medium transition-colors {}",
                            if periodo() == p {
                                "bg-blue-600 text-white"
                            } else {
                                "border border-slate-200 bg-white text-slate-600 hover:bg-slate-100 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-300 dark:hover:bg-slate-800"
                            }
                        ),
                        onclick: move |_| periodo.set(p),
                        "{p.etiqueta()}"
                    }
                }
                if !rango.0.is_empty() {
                    span { class: "ml-2 text-xs text-slate-400 dark:text-slate-500", "{rango.0} → {rango.1}" }
                }
            }
            div { class: "grid grid-cols-2 gap-4 xl:grid-cols-5",
                KpiCard {
                    etiqueta: "Capital colocado".to_string(),
                    valor: fmt_moneda(st.capital_colocado),
                    icono: "M17 9V7a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2m2 4h10a2 2 0 002-2v-6a2 2 0 00-2-2H9a2 2 0 00-2 2v6a2 2 0 002 2zm7-5a2 2 0 11-4 0 2 2 0 014 0z".to_string(),
                    acento: "text-blue-600 dark:text-blue-400".to_string(),
                    valor_class: VALOR_NORMAL.to_string(),
                }
                KpiCard {
                    etiqueta: "Cobrado (periodo)".to_string(),
                    valor: fmt_moneda(st.cobrado_periodo),
                    icono: "M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z".to_string(),
                    acento: "text-green-600 dark:text-green-400".to_string(),
                    valor_class: VALOR_NORMAL.to_string(),
                }
                KpiCard {
                    etiqueta: "Por cobrar neto".to_string(),
                    valor: fmt_moneda(st.por_cobrar_neto),
                    icono: "M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2".to_string(),
                    acento: "text-indigo-600 dark:text-indigo-400".to_string(),
                    valor_class: VALOR_NORMAL.to_string(),
                }
                KpiCard {
                    etiqueta: "Cartera vencida".to_string(),
                    valor: fmt_moneda(st.cartera_vencida),
                    icono: "M12 9v2m0 4h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z".to_string(),
                    acento: "text-red-600 dark:text-red-400".to_string(),
                    valor_class: VALOR_NORMAL.to_string(),
                }
                KpiCard {
                    etiqueta: "Tasa de morosidad".to_string(),
                    valor: format!("{:.1}%", st.tasa_morosidad * 100.0),
                    icono: "M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z".to_string(),
                    acento: "text-amber-600 dark:text-amber-400".to_string(),
                    valor_class: semaforo_morosidad(st.tasa_morosidad).to_string(),
                }
            }
            match resumen() {
                None => rsx! {
                    div { class: "text-sm text-slate-500 animate-pulse dark:text-slate-400",
                        "Cargando gráficas de la cartera…"
                    }
                },
                Some(Err(e)) => rsx! {
                    div { class: "rounded-xl border border-red-300 bg-white p-6 text-sm text-red-600 dark:border-red-800/60 dark:bg-slate-900 dark:text-red-400",
                        "No se pudieron cargar las gráficas: {e}"
                    }
                },
                Some(Ok(r)) => rsx! { TarjetasResumen { resumen: r } },
            }
        }
    }
}

/// Card de KPI (mismo shell para los 5); `valor_class` deja colorear el número
/// (p. ej. el semáforo de morosidad).
#[component]
fn KpiCard(
    etiqueta: String,
    valor: String,
    icono: String,
    acento: String,
    valor_class: String,
) -> Element {
    rsx! {
        div { class: "rounded-xl border border-slate-200 bg-white p-5 dark:border-slate-700 dark:bg-slate-900",
            div { class: "mb-3 flex items-center gap-2",
                svg { class: format!("h-5 w-5 shrink-0 {acento}"), view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                    path { d: "{icono}" }
                }
                div { class: "text-sm font-medium text-slate-500 dark:text-slate-400", "{etiqueta}" }
            }
            div { class: format!("text-2xl font-bold {valor_class}"), "{valor}" }
        }
    }
}

/// Grid 2×N con las 6 gráficas del resumen (las de la captura de V que tienen
/// datos reales; las demás están diferidas por el plan, decision log 2026-08-31).
#[component]
fn TarjetasResumen(resumen: Resumen) -> Element {
    let barras: Vec<DatoMes> = resumen
        .cobrado_vs_por_cobrar
        .iter()
        .map(|m| DatoMes { mes: m.mes.clone(), cobrado: m.cobrado, por_cobrar: m.por_cobrar })
        .collect();
    let flujo: Vec<DatoCategoria> = resumen
        .flujo_proyectado
        .iter()
        .map(|f| DatoCategoria { etiqueta: format!("{} días", f.horizonte), valor: f.monto })
        .collect();
    let aging: Vec<DatoCategoria> = resumen
        .aging
        .iter()
        .map(|a| DatoCategoria { etiqueta: a.bucket.clone(), valor: a.monto })
        .collect();
    let deudores: Vec<DatoCategoria> = resumen
        .top_deudores
        .iter()
        .map(|d| DatoCategoria {
            etiqueta: if d.nombre.trim().is_empty() { d.cliente_curp.clone() } else { d.nombre.clone() },
            valor: d.saldo,
        })
        .collect();
    let dist: Vec<DatoCategoria> = resumen
        .distribucion_montos
        .iter()
        .map(|d| DatoCategoria { etiqueta: d.bucket.clone(), valor: d.n as f64 })
        .collect();
    let tasa_pct = resumen.tasa_morosidad * 100.0;
    rsx! {
        div { class: "grid grid-cols-2 gap-6",
            TarjetaGrafica { title: "Cobrado vs por cobrar (periodo, MXN)".to_string(),
                if barras.iter().any(|b| b.cobrado + b.por_cobrar > 0.0) {
                    BarraApilada { datos: barras }
                } else {
                    SinDatos {}
                }
            }
            TarjetaGrafica { title: "Tasa de morosidad".to_string(),
                div { class: "flex flex-col items-center justify-center gap-1 py-6",
                    div { class: format!("text-5xl font-bold {}", semaforo_morosidad(resumen.tasa_morosidad)),
                        "{tasa_pct:.1}%"
                    }
                    div { class: "text-xs text-slate-500 dark:text-slate-400",
                        "cartera vencida sobre capital colocado (dinero)"
                    }
                    div { class: "text-xs text-slate-400 dark:text-slate-500",
                        "<5% verde · 5–20% ámbar · >20% rojo"
                    }
                }
            }
            TarjetaGrafica { title: "Flujo de caja proyectado (MXN)".to_string(),
                if flujo.iter().any(|d| d.valor > 0.0) {
                    Linea { datos: flujo, color: "#3b82f6".to_string() }
                } else {
                    SinDatos {}
                }
            }
            TarjetaGrafica { title: "Créditos por monto (conteo)".to_string(),
                if dist.iter().any(|d| d.valor > 0.0) {
                    Donut { datos: dist }
                } else {
                    SinDatos {}
                }
            }
            TarjetaGrafica { title: "Aging de cartera (saldo vencido, MXN)".to_string(),
                if aging.iter().any(|d| d.valor > 0.0) {
                    BarraH { datos: aging, color: "#f59e0b".to_string() }
                } else {
                    SinDatos {}
                }
            }
            TarjetaGrafica { title: "Top 10 clientes con mayor deuda (MXN)".to_string(),
                if deudores.iter().any(|d| d.valor > 0.0) {
                    BarraH { datos: deudores, color: "#ef4444".to_string() }
                } else {
                    SinDatos {}
                }
            }
        }
    }
}

/// Card contenedora de gráfica (mismo shell que los KPIs).
#[component]
fn TarjetaGrafica(title: String, children: Element) -> Element {
    rsx! {
        div { class: "rounded-xl border border-slate-200 bg-white p-6 dark:border-slate-700 dark:bg-slate-900",
            div { class: "mb-4 text-sm font-medium text-slate-500 dark:text-slate-400", "{title}" }
            {children}
        }
    }
}

/// Estado vacío dentro del card (sin datos ≠ error).
#[component]
fn SinDatos() -> Element {
    rsx! {
        div { class: "py-10 text-center text-sm text-slate-400 dark:text-slate-500", "Sin datos aún" }
    }
}
