//! Panel de control: tarjetas de estadísticas de la empresa.

use dioxus::prelude::*;

use crate::api::{authed_request, sesion_ok, DashboardStats};

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

    let stats = dashboard_stats();
    rsx! {
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
    }
}