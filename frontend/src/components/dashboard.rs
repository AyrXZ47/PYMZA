//! Panel de control: tarjetas de estadísticas de la empresa + 6 gráficas del
//! resumen de cartera (contrato API ola 4) en primitivas SVG de charts.rs.

use dioxus::prelude::*;

use crate::api::{authed_request, obtener_resumen, sesion_ok, DashboardStats, Resumen};
use crate::components::charts::{
    semaforo_morosidad, BarraApilada, BarraH, DatoCategoria, DatoMes, Donut, Linea,
};

#[component]
pub fn Dashboard(token: Signal<String>, is_authenticated: Signal<bool>) -> Element {
    let mut dashboard_stats = use_signal(|| DashboardStats::default());
    let mut stats_loaded = use_signal(|| false);

    // Se monta/desmonta al navegar por el menú: refetchea stats en cada visita.
    if !stats_loaded() {
        let token_val = token();
        stats_loaded.set(true);
        spawn(async move {
            match authed_request(reqwest::Method::GET, "/api/dashboard".to_string(), &token_val)
                .send()
                .await
            {
                Ok(res) => {
                    if sesion_ok(&res, is_authenticated, token) {
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
                }
                Err(_) => {}
            }
        });
    }

    // Resumen de cartera (fuente de las gráficas): se lee el token ANTES del
    // await (clippy.toml: nada de señales vivo sobre un await).
    let resumen = use_resource(move || {
        let token_val = token();
        async move { obtener_resumen(&token_val, is_authenticated, token).await }
    });

    let stats = dashboard_stats();
    rsx! {
        div { class: "flex flex-col gap-6",
            div { class: "grid grid-cols-3 gap-6",
                div { class: "bg-white rounded-xl border border-slate-200 p-6 dark:bg-slate-900 dark:border-slate-700",
                    div { class: "flex items-center gap-3 mb-4",
                        svg { class: "w-6 h-6 text-blue-600 dark:text-blue-400", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                            path { d: "M17 9V7a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2m2 4h10a2 2 0 002-2v-6a2 2 0 00-2-2H9a2 2 0 00-2 2v6a2 2 0 002 2zm7-5a2 2 0 11-4 0 2 2 0 014 0z" }
                        }
                        div { class: "text-slate-500 text-sm font-medium dark:text-slate-400", "Créditos Activos" }
                    }
                    div { class: "text-3xl font-bold text-slate-900 dark:text-white", "{stats.creditos_activos}" }
                }
                div { class: "bg-white rounded-xl border border-slate-200 p-6 dark:bg-slate-900 dark:border-slate-700",
                    div { class: "flex items-center gap-3 mb-4",
                        svg { class: "w-6 h-6 text-green-600 dark:text-green-400", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                            path { d: "M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" }
                        }
                        div { class: "text-slate-500 text-sm font-medium dark:text-slate-400", "Capital Prestado" }
                    }
                    div { class: "text-3xl font-bold text-slate-900 dark:text-white", "${stats.capital_prestado} MXN" }
                }
                div { class: "bg-white rounded-xl border border-slate-200 p-6 dark:bg-slate-900 dark:border-slate-700",
                    div { class: "flex items-center gap-3 mb-4",
                        svg { class: "w-6 h-6 text-yellow-600 dark:text-yellow-400", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                            path { d: "M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" }
                        }
                        div { class: "text-slate-500 text-sm font-medium dark:text-slate-400", "Próximos Cobros" }
                    }
                    div { class: "text-3xl font-bold text-slate-900 dark:text-white", "{stats.proximos_cobros}" }
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
            TarjetaGrafica { title: "Cobrado vs por cobrar (6 meses)".to_string(),
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
                        "planes morosos sobre planes no liquidados"
                    }
                    div { class: "text-xs text-slate-400 dark:text-slate-500",
                        "<5% verde · 5–20% ámbar · >20% rojo"
                    }
                }
            }
            TarjetaGrafica { title: "Flujo de caja proyectado".to_string(),
                if flujo.iter().any(|d| d.valor > 0.0) {
                    Linea { datos: flujo, color: "#3b82f6".to_string() }
                } else {
                    SinDatos {}
                }
            }
            TarjetaGrafica { title: "Créditos por monto".to_string(),
                if dist.iter().any(|d| d.valor > 0.0) {
                    Donut { datos: dist }
                } else {
                    SinDatos {}
                }
            }
            TarjetaGrafica { title: "Aging de cartera (saldo vencido)".to_string(),
                if aging.iter().any(|d| d.valor > 0.0) {
                    BarraH { datos: aging, color: "#f59e0b".to_string() }
                } else {
                    SinDatos {}
                }
            }
            TarjetaGrafica { title: "Top 10 clientes con mayor deuda".to_string(),
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
