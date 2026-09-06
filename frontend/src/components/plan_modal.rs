//! Modal de plan de pagos: configurar → evaluar → autorizar crédito.

use dioxus::prelude::*;

use crate::api::{authed_request, descargar_archivo, descargar_contrato, sesion_ok, PagoInfo};

#[component]
pub fn PlanModal(
    mut show_plan_modal: Signal<bool>,
    curp: String,
    token: Signal<String>,
    is_authenticated: Signal<bool>,
) -> Element {
    let mut plan_producto = use_signal(|| String::new());
    let mut plan_monto = use_signal(|| String::new());
    let mut plan_plazo = use_signal(|| "3".to_string());
    let mut modal_step = use_signal(|| 0u8);
    let mut eval_estado = use_signal(|| String::new());
    let mut eval_pago_mensual = use_signal(|| 0.0);
    let mut eval_tasa_interes = use_signal(|| 0.0);
    let mut eval_plan_pagos = use_signal(|| Vec::<PagoInfo>::new());
    let mut consideraciones = use_signal(|| String::new());
    let mut terms_accepted = use_signal(|| false);

    rsx! {
        div { class: "fixed inset-0 bg-black/80 flex items-center justify-center z-50",
            div { class: "bg-white border border-slate-200 p-6 rounded-xl w-full max-w-2xl dark:bg-slate-900 dark:border-slate-700",
                match modal_step() {
                    0 => rsx! {
                        h2 { class: "text-xl font-bold text-slate-900 mb-4 dark:text-white", "Configurar Plan de Pagos" }
                        div { class: "flex flex-col gap-4",
                            label { class: "text-slate-500 text-sm dark:text-slate-400", "Producto o Servicio" }
                            input {
                                class: "bg-white border border-slate-300 text-slate-900 rounded-lg px-3 py-2 outline-none focus:border-blue-500 dark:bg-slate-800 dark:border-slate-600 dark:text-white",
                                value: plan_producto(),
                                oninput: move |e| plan_producto.set(e.value())
                            }

                            label { class: "text-slate-500 text-sm dark:text-slate-400", "Monto Total ($)" }
                            input {
                                class: "bg-white border border-slate-300 text-slate-900 rounded-lg px-3 py-2 outline-none focus:border-blue-500 dark:bg-slate-800 dark:border-slate-600 dark:text-white",
                                type: "number",
                                value: plan_monto(),
                                oninput: move |e| plan_monto.set(e.value())
                            }

                            label { class: "text-slate-500 text-sm dark:text-slate-400", "Plazo (Meses)" }
                            select {
                                class: "bg-white border border-slate-300 text-slate-900 rounded-lg px-3 py-2 outline-none focus:border-blue-500 dark:bg-slate-800 dark:border-slate-600 dark:text-white",
                                value: plan_plazo(),
                                onchange: move |e| plan_plazo.set(e.value()),
                                option { value: "3", "3 meses — Tasa 3%" }
                                option { value: "6", "6 meses — Tasa 6%" }
                                option { value: "9", "9 meses — Tasa 10%" }
                                option { value: "12", "12 meses — Tasa 15%" }
                            }
                        }
                        div { class: "flex justify-end gap-3 mt-6",
                            button {
                                class: "bg-slate-200 hover:bg-slate-300 text-slate-700 font-semibold px-4 py-2 rounded-lg dark:bg-slate-700 dark:hover:bg-slate-600 dark:text-white",
                                onclick: move |_| { show_plan_modal.set(false); modal_step.set(0); },
                                "Cancelar"
                            }
                            button {
                                class: "bg-blue-600 hover:bg-blue-700 text-white font-semibold px-4 py-2 rounded-lg",
                                onclick: move |_| {
                                    let curp = curp.clone();
                                    let monto_str = plan_monto();
                                    let plazo = plan_plazo();
                                    let token_val = token();
                                    modal_step.set(1);
                                    spawn(async move {
                                        let monto_num: f64 = monto_str.parse().unwrap_or(0.0);
                                        let plazo_num: i32 = plazo.parse().unwrap_or(3);
                                        let body = serde_json::json!({
                                            "curp": curp,
                                            "monto": monto_num,
                                            "plazo_meses": plazo_num,
                                        });
                                        match authed_request(reqwest::Method::POST, "/api/creditos/evaluar".to_string(), &token_val)
                                            .json(&body)
                                            .send()
                                            .await
                                        {
                                            Ok(res) => {
                                                if sesion_ok(&res, is_authenticated, token) {
                                                    match res.json::<serde_json::Value>().await {
                                                        Ok(data) => {
                                                            eval_estado.set(data["estado"].as_str().unwrap_or("").to_string());
                                                            eval_pago_mensual.set(data["pago_mensual"].as_f64().unwrap_or(0.0));
                                                            eval_tasa_interes.set(data["tasa_interes"].as_f64().unwrap_or(0.0));
                                                            consideraciones.set(data["consideraciones"].as_str().unwrap_or("").to_string());
                                                            if let Some(plan) = data["plan_pagos"].as_array() {
                                                                let planes: Vec<PagoInfo> = plan.iter().map(|p| PagoInfo {
                                                                    mes: p["mes"].as_i64().unwrap_or(0) as i32,
                                                                    pago: p["pago"].as_f64().unwrap_or(0.0),
                                                                    interes: p["interes"].as_f64().unwrap_or(0.0),
                                                                    capital: p["capital"].as_f64().unwrap_or(0.0),
                                                                    saldo_restante: p["saldo_restante"].as_f64().unwrap_or(0.0),
                                                                }).collect();
                                                                eval_plan_pagos.set(planes);
                                                            }
                                                        }
                                                        Err(_) => {
                                                            eval_estado.set("Error".to_string());
                                                            consideraciones.set("Error al procesar la evaluación".to_string());
                                                        }
                                                    }
                                                }
                                            }
                                            Err(_) => {
                                                eval_estado.set("Error".to_string());
                                                consideraciones.set("Error de conexión con el servidor".to_string());
                                            }
                                        }
                                        modal_step.set(2);
                                    });
                                },
                                "Evaluar Crédito"
                            }
                        }
                    },
                    1 => rsx! {
                        div { class: "flex flex-col items-center justify-center py-12",
                            div { class: "text-blue-600 dark:text-blue-400 text-lg font-semibold animate-pulse", "Evaluando riesgo en Red PYMZA..." }
                        }
                    },
                    2 => {
                        let planes = eval_plan_pagos();
                        rsx! {
                        h2 { class: "text-xl font-bold text-slate-900 mb-4 dark:text-white", "Resultado de Evaluación" }
                        div { class: "flex flex-col gap-4",
                            div {
                                class: format!("text-lg font-bold {}", if eval_estado() == "Aprobado" { "text-green-600 dark:text-green-400" } else { "text-red-600 dark:text-red-400" }),
                                "{eval_estado}"
                            }

                            div { class: "bg-slate-50 border border-slate-200 rounded-lg p-4 dark:bg-slate-800 dark:border-slate-600",
                                div { class: "grid grid-cols-2 gap-4",
                                    div {
                                        div { class: "text-slate-500 text-xs uppercase mb-1 dark:text-slate-400", "Mensualidad Estimada" }
                                        div { class: "text-2xl font-bold text-slate-900 dark:text-white", "${eval_pago_mensual} MXN" }
                                    }
                                    div {
                                        div { class: "text-slate-500 text-xs uppercase mb-1 dark:text-slate-400", "Tasa de Interés" }
                                        div { class: "text-xl font-bold text-yellow-400", "{(eval_tasa_interes() * 100.0) as i32}%" }
                                    }
                                }
                            }

                            div { class: "bg-slate-50 border border-slate-200 rounded-lg p-4 dark:bg-slate-800 dark:border-slate-600",
                                div { class: "text-slate-500 text-sm mb-2 font-semibold dark:text-slate-400", "Plan de Pagos" }
                                div { class: "overflow-x-auto max-h-48 overflow-y-auto",
                                    table { class: "w-full text-sm text-left",
                                        thead {
                                        tr { class: "text-slate-500 border-b border-slate-200 dark:text-slate-400 dark:border-slate-700",
                                            th { class: "py-2 px-2", "Mes" }
                                            th { class: "py-2 px-2", "Pago" }
                                            th { class: "py-2 px-2", "Interés" }
                                            th { class: "py-2 px-2", "Capital" }
                                            th { class: "py-2 px-2", "Saldo Restante" }
                                        }
                                        }
                                        tbody {
                                            for p in &planes {
                                                tr { class: "border-b border-slate-200 text-slate-700 dark:border-slate-700/50 dark:text-slate-300",
                                                    td { class: "py-2 px-2", "{p.mes}" }
                                                    td { class: "py-2 px-2", "${p.pago}" }
                                                    td { class: "py-2 px-2", "${p.interes}" }
                                                    td { class: "py-2 px-2", "${p.capital}" }
                                                    td { class: "py-2 px-2", "${p.saldo_restante}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "bg-slate-50 border border-slate-200 rounded-lg p-4 max-h-32 overflow-y-auto dark:bg-slate-800 dark:border-slate-600",
                                div { class: "text-slate-400 text-sm mb-2", "Consideraciones" }
                                div { class: "text-slate-700 text-sm whitespace-pre-wrap dark:text-slate-200", "{consideraciones}" }
                            }

                            label { class: "flex items-start gap-2 text-slate-700 text-sm cursor-pointer dark:text-slate-300",
                                input {
                                    type: "checkbox",
                                    class: "mt-1",
                                    checked: terms_accepted(),
                                    oninput: move |e| terms_accepted.set(e.checked()),
                                }
                                "He leído las consideraciones y asumo el riesgo de autorizar este crédito."
                            }
                        }
                        div { class: "flex justify-end gap-3 mt-6",
                            button {
                                class: "bg-slate-200 hover:bg-slate-300 text-slate-700 font-semibold px-4 py-2 rounded-lg dark:bg-slate-700 dark:hover:bg-slate-600 dark:text-white",
                                onclick: move |_| { show_plan_modal.set(false); modal_step.set(0); },
                                "Cerrar"
                            }
                            button {
                                class: format!("text-white font-semibold px-4 py-2 rounded-lg {}", if terms_accepted() { "bg-green-600 hover:bg-green-700" } else { "bg-slate-300 cursor-not-allowed dark:bg-gray-600" }),
                                disabled: !terms_accepted(),
                                onclick: move |_| {
                                    let cliente_curp = curp.clone();
                                    let producto = plan_producto();
                                    let monto_total = plan_monto();
                                    let plazo = plan_plazo();
                                    let pago_mensual = eval_pago_mensual();
                                    let tasa = eval_tasa_interes();
                                    let token_val = token();
                                    let _plan_pagos_snapshot = eval_plan_pagos();
                                    spawn(async move {
                                        let monto_num: f64 = monto_total.parse().unwrap_or(0.0);
                                        let plazo_num: i32 = plazo.parse().unwrap_or(3);
                                        // Contrato ola 1: la empresa sale del JWT, no del body.
                                        // (clone: `cliente_curp` se reusa abajo para descargar el contrato)
                                        let body = serde_json::json!({
                                            "cliente_curp": cliente_curp.clone(),
                                            "producto": producto,
                                            "monto_total": monto_num,
                                            "plazo_meses": plazo_num,
                                            "pago_mensual": pago_mensual,
                                            "tasa_interes": tasa,
                                        });
                                        if let Ok(res) = authed_request(reqwest::Method::POST, "/api/creditos/autorizar".to_string(), &token_val)
                                            .json(&body)
                                            .send()
                                            .await
                                        {
                                            if sesion_ok(&res, is_authenticated, token) {
                                                if let Ok(data) = res.json::<serde_json::Value>().await {
                                                    if data["status"] == "success" {
                                                        // El dashboard y la cartera refetchean solos al volver a
                                                        // sus pantallas (se montan/desmontan con el menú).
                                                        show_plan_modal.set(false);
                                                        modal_step.set(0);
                                                        plan_producto.set(String::new());
                                                        plan_monto.set(String::new());
                                                        plan_plazo.set("3".to_string());
                                                        terms_accepted.set(false);
                                                        // Contrato PDF (ola 6): descarga automática del
                                                        // contrato del plan recién autorizado (el id viene
                                                        // en la respuesta).
                                                        if let Some(plan_id) = data["plan_id"].as_str() {
                                                            match descargar_contrato(
                                                                plan_id,
                                                                &cliente_curp,
                                                                &token_val,
                                                                is_authenticated,
                                                                token,
                                                            )
                                                            .await
                                                            {
                                                                Ok((bytes, nombre)) => descargar_archivo(&bytes, &nombre),
                                                                Err(_) => {} // ponytail: aquí el modal ya cerró y no hay dónde mostrar el error; la cartera ofrece el botón con errores visibles
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    });
                                },
                                "Autorizar Crédito"
                            }
                        }
                        }
                    },
                    _ => rsx! {},
                }
            }
        }
    }
}