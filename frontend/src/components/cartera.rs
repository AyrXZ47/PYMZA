//! Cartera: tabla de planes de pago de la empresa con registro de pagos
//! (contrato API ola 4: estados Activo/Moroso/Liquidado + cuotas_pagadas).

use dioxus::prelude::*;

use crate::api::{
    authed_request, descargar_archivo, descargar_contrato, registrar_pago, sesion_ok,
    siguiente_cuota_impaga,
};

/// Descarga la lista de planes del tenant (al montar y tras cada pago).
async fn cargar_planes(
    token_val: String,
    mut cartera_planes: Signal<Vec<serde_json::Value>>,
    is_authenticated: Signal<bool>,
    token: Signal<String>,
) {
    match authed_request(reqwest::Method::GET, "/api/creditos".to_string(), &token_val).send().await
    {
        Ok(res) => {
            if sesion_ok(&res, is_authenticated, token) {
                if let Ok(data) = res.json::<serde_json::Value>().await {
                    if data["status"] == "success" {
                        if let Some(arr) = data["creditos"].as_array() {
                            cartera_planes.set(arr.clone());
                        }
                    }
                }
            }
        }
        Err(_) => {}
    }
}

#[component]
pub fn Cartera(token: Signal<String>, is_authenticated: Signal<bool>) -> Element {
    let cartera_planes = use_signal(|| Vec::<serde_json::Value>::new());
    let mut cartera_loaded = use_signal(|| false);

    // Mini-form inline de pago: un solo formulario abierto a la vez.
    let pago_plan = use_signal(|| Option::<String>::None);
    let pago_cuota = use_signal(|| String::new());
    let pago_monto = use_signal(|| String::new());
    let pago_error = use_signal(|| String::new());
    let pago_enviando = use_signal(|| false);

    // Descarga de contrato: el plan que está descargando (uno a la vez) y el
    // último error (visible bajo la tabla). Se escriben dentro de FilaPlan.
    let descargando = use_signal(|| Option::<String>::None);
    let descarga_error = use_signal(|| String::new());

    // Se monta/desmonta al navegar por el menú: refetchea la cartera en cada visita.
    if !cartera_loaded() {
        let token_val = token();
        cartera_loaded.set(true);
        spawn(cargar_planes(token_val, cartera_planes, is_authenticated, token));
    }

    let planes = cartera_planes();
    // (form_abierto, plan) precomputado: rsx no admite let en el cuerpo del for.
    let filas: Vec<(bool, serde_json::Value)> = planes
        .iter()
        .map(|p| {
            let abierto = pago_plan().as_deref() == Some(p["_id"].as_str().unwrap_or(""));
            (abierto, p.clone())
        })
        .collect();
    rsx! {
        div {
            h2 { class: "text-2xl font-bold mb-6 text-slate-900 dark:text-white", "Cartera de Créditos" }
            if planes.is_empty() {
                div { class: "text-slate-500 dark:text-slate-400", "No hay créditos activos para esta empresa." }
            } else {
                div { class: "overflow-x-auto",
                    table { class: "w-full text-sm text-left bg-white rounded-xl border border-slate-200 dark:bg-slate-900 dark:border-slate-700",
                        thead {
                            tr { class: "text-slate-500 border-b border-slate-200 dark:text-slate-400 dark:border-slate-700",
                                th { class: "py-3 px-4", "Producto" }
                                th { class: "py-3 px-4", "Cliente (CURP)" }
                                th { class: "py-3 px-4", "Monto Total" }
                                th { class: "py-3 px-4", "Plazo" }
                                th { class: "py-3 px-4", "Pago Mensual" }
                                th { class: "py-3 px-4", "Interés" }
                                th { class: "py-3 px-4", "Estado" }
                                th { class: "py-3 px-4", "Cuotas" }
                                th { class: "py-3 px-4", "Acciones" }
                                th { class: "py-3 px-4", "Fecha" }
                            }
                        }
                        tbody {
                            for (abierto, plan) in filas {
                                FilaPlan {
                                    plan,
                                    form_abierto: abierto,
                                    token,
                                    is_authenticated,
                                    cartera_planes,
                                    pago_plan,
                                    pago_cuota,
                                    pago_monto,
                                    pago_error,
                                    pago_enviando,
                                    descargando,
                                    descarga_error,
                                }
                            }
                        }
                    }
                }
                if !descarga_error().is_empty() {
                    div { class: "mt-3 text-sm text-red-600 dark:text-red-400", "{descarga_error}" }
                }
            }
        }
    }
}

/// Fila de un plan + su mini-form inline de pago (renderizado como segunda
/// fila de la tabla cuando `form_abierto`). Componente propio para que los
/// closures de evento capturen valores owned.
#[component]
#[allow(clippy::too_many_arguments)]
fn FilaPlan(
    plan: serde_json::Value,
    form_abierto: bool,
    token: Signal<String>,
    is_authenticated: Signal<bool>,
    mut cartera_planes: Signal<Vec<serde_json::Value>>,
    mut pago_plan: Signal<Option<String>>,
    mut pago_cuota: Signal<String>,
    mut pago_monto: Signal<String>,
    mut pago_error: Signal<String>,
    mut pago_enviando: Signal<bool>,
    mut descargando: Signal<Option<String>>,
    mut descarga_error: Signal<String>,
) -> Element {
    let id = plan["_id"].as_str().unwrap_or("").to_string();
    // Clon para el botón de descarga: el closure de "Registrar pago" mueve `id`.
    let id_descarga = id.clone();
    let curp = plan["cliente_curp"].as_str().unwrap_or("").to_string();
    let estado = plan["estado"].as_str().unwrap_or("—").to_string();
    let badge = match estado.as_str() {
        "Activo" => "bg-green-100 text-green-700 dark:bg-green-900/50 dark:text-green-400",
        "Moroso" => "bg-amber-100 text-amber-700 dark:bg-amber-900/50 dark:text-amber-400",
        _ => "bg-slate-200 text-slate-700 dark:bg-slate-700 dark:text-slate-300",
    };
    let plazo = plan["plazo_meses"].as_i64().unwrap_or(0);
    let pagadas = plan["cuotas_pagadas"].as_i64().unwrap_or(0);
    let vencidas = plan["cuotas_vencidas"].as_i64().unwrap_or(0);
    let pago_mensual = plan["pago_mensual"].as_f64().unwrap_or(0.0);
    let siguiente = siguiente_cuota_impaga(plazo, pagadas);
    // Opciones del select: todas las cuotas impagas, empezando por la siguiente.
    let cuotas: Vec<i64> = siguiente.map(|s| (s..=plazo).collect()).unwrap_or_default();
    let input_class = "bg-white border border-slate-300 text-slate-900 rounded-lg px-3 py-2 text-sm outline-none focus:border-blue-500 dark:bg-slate-800 dark:border-slate-600 dark:text-white";
    rsx! {
        tr { class: "border-b border-slate-200 text-slate-700 dark:border-slate-700/50 dark:text-slate-300",
            td { class: "py-3 px-4", "{plan[\"producto\"].as_str().unwrap_or(\"—\")}" }
            td { class: "py-3 px-4 font-mono", "{plan[\"cliente_curp\"].as_str().unwrap_or(\"—\")}" }
            td { class: "py-3 px-4", "${plan[\"monto_total\"].as_f64().unwrap_or(0.0)} MXN" }
            td { class: "py-3 px-4", "{plan[\"plazo_meses\"].as_i64().unwrap_or(0)} meses" }
            td { class: "py-3 px-4", "${plan[\"pago_mensual\"].as_f64().unwrap_or(0.0)} MXN" }
            td { class: "py-3 px-4", "{(plan[\"tasa_interes\"].as_f64().unwrap_or(0.0) * 100.0) as i32}%" }
            td { class: "py-3 px-4",
                span { class: format!("px-2 py-1 rounded-full text-xs {badge}"), "{estado}" }
            }
            td { class: "py-3 px-4",
                div { class: "text-xs", "Cuota {pagadas}/{plazo} pagadas" }
                if vencidas > 0 {
                    div { class: "text-xs text-red-600 dark:text-red-400", "{vencidas} vencidas" }
                }
            }
            td { class: "py-3 px-4",
                div { class: "flex flex-col items-start gap-1.5",
                    if estado == "Liquidado" || siguiente.is_none() {
                        span { class: "text-xs text-slate-300 dark:text-slate-600", "—" }
                    } else {
                        button {
                            class: "bg-blue-600 hover:bg-blue-700 text-white text-xs font-semibold px-3 py-1.5 rounded-lg",
                            onclick: move |_| {
                                pago_plan.set(Some(id.clone()));
                                pago_monto.set(if pago_mensual > 0.0 { format!("{pago_mensual}") } else { String::new() });
                                pago_cuota.set(siguiente.map(|c| c.to_string()).unwrap_or_default());
                                pago_error.set(String::new());
                            },
                            "Registrar pago"
                        }
                    }
                    // Contrato PDF (ola 6): disponible para todo plan del tenant,
                    // incluso liquidado (el contrato sigue siendo histórico válido).
                    button {
                        class: "bg-slate-200 hover:bg-slate-300 text-slate-700 text-xs font-semibold px-3 py-1.5 rounded-lg dark:bg-slate-700 dark:hover:bg-slate-600 dark:text-white",
                        disabled: descargando().as_deref() == Some(id_descarga.as_str()),
                        onclick: move |_| {
                            let token_val = token();
                            let plan_id = id_descarga.clone();
                            let curp_val = curp.clone();
                            descargando.set(Some(plan_id.clone()));
                            descarga_error.set(String::new());
                            spawn(async move {
                                match descargar_contrato(
                                    &plan_id,
                                    &curp_val,
                                    &token_val,
                                    is_authenticated,
                                    token,
                                )
                                .await
                                {
                                    Ok((bytes, nombre)) => descargar_archivo(&bytes, &nombre),
                                    Err(e) => descarga_error.set(e),
                                }
                                descargando.set(None);
                            });
                        },
                        if descargando().as_deref() == Some(id_descarga.as_str()) {
                            "Descargando…"
                        } else {
                            "Descargar contrato"
                        }
                    }
                }
            }
            td { class: "py-3 px-4", "{plan[\"fecha\"].as_str().unwrap_or(\"—\")}" }
        }
        if form_abierto {
            tr {
                td { colspan: "10", class: "px-4 pb-4",
                    div { class: "bg-slate-50 border border-slate-200 rounded-lg p-4 dark:bg-slate-800/60 dark:border-slate-700",
                        div { class: "flex flex-wrap items-end gap-3",
                            div { class: "flex flex-col gap-1",
                                label { class: "text-xs text-slate-500 dark:text-slate-400", "Cuota a pagar" }
                                select {
                                    class: "{input_class}",
                                    value: pago_cuota(),
                                    onchange: move |e| pago_cuota.set(e.value()),
                                    for c in &cuotas {
                                        option { value: "{c}", "Cuota {c}" }
                                    }
                                }
                            }
                            div { class: "flex flex-col gap-1",
                                label { class: "text-xs text-slate-500 dark:text-slate-400", "Monto ($)" }
                                input {
                                    class: "{input_class}",
                                    type: "number",
                                    value: pago_monto(),
                                    oninput: move |e| pago_monto.set(e.value()),
                                }
                            }
                            button {
                                class: "bg-blue-600 hover:bg-blue-700 text-white text-sm font-semibold px-4 py-2 rounded-lg",
                                disabled: pago_enviando(),
                                onclick: move |_| {
                                    let token_val = token();
                                    let Some(plan_id) = pago_plan() else { return };
                                    let cuota: i64 = pago_cuota().parse().unwrap_or(0);
                                    let monto = match pago_monto().trim().parse::<f64>() {
                                        Ok(m) if m > 0.0 => m,
                                        _ => {
                                            pago_error.set("Monto inválido".to_string());
                                            return;
                                        }
                                    };
                                    pago_enviando.set(true);
                                    pago_error.set(String::new());
                                    spawn(async move {
                                        match registrar_pago(&plan_id, cuota, monto, &token_val).send().await {
                                            Ok(res) => {
                                                if sesion_ok(&res, is_authenticated, token) {
                                                    if res.status().is_success() {
                                                        pago_plan.set(None);
                                                        pago_error.set(String::new());
                                                        // Refresca la lista: badges y cuotas al día.
                                                        spawn(cargar_planes(
                                                            token_val.clone(),
                                                            cartera_planes,
                                                            is_authenticated,
                                                            token,
                                                        ));
                                                    } else if let Ok(data) = res.json::<serde_json::Value>().await {
                                                        pago_error.set(data["message"]
                                                            .as_str()
                                                            .unwrap_or("No se pudo registrar el pago")
                                                            .to_string());
                                                    } else {
                                                        pago_error.set("No se pudo registrar el pago".to_string());
                                                    }
                                                }
                                            }
                                            Err(_) => pago_error.set("Sin conexión con el servidor".to_string()),
                                        }
                                        pago_enviando.set(false);
                                    });
                                },
                                if pago_enviando() { "Registrando…" } else { "Registrar" }
                            }
                            button {
                                class: "bg-slate-200 hover:bg-slate-300 text-slate-700 text-sm font-semibold px-4 py-2 rounded-lg dark:bg-slate-700 dark:hover:bg-slate-600 dark:text-white",
                                onclick: move |_| {
                                    pago_plan.set(None);
                                    pago_error.set(String::new());
                                },
                                "Cancelar"
                            }
                        }
                        if !pago_error().is_empty() {
                            div { class: "mt-3 text-sm text-red-600 dark:text-red-400", "{pago_error}" }
                        }
                    }
                }
            }
        }
    }
}
