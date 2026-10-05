//! Cartera: planes del tenant con buscador/filtros en memoria, dos tablas
//! (activos —incluye Moroso— y liquidados/inactivos) y registro de pagos de
//! cuota y de abonos parciales (contrato API ola 7).

use std::cmp::Ordering;

use dioxus::prelude::*;

use crate::api::{
    authed_request, cobrado_plan, descargar_archivo, descargar_contrato, nombre_plan,
    registrar_abono, registrar_pago, saldo_plan, sesion_ok, siguiente_cuota_impaga,
};

/// Descarga la lista de planes del tenant (al montar y tras cada pago/abono).
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

// --- Filtrado y orden en memoria (puro, testeable en host). Los planes de una
// PYME son pocos miles; una pasada basta. ---

/// La cartera viva incluye Moroso: ambos van en la tabla de activos.
fn es_activo(estado: &str) -> bool {
    estado == "Activo" || estado == "Moroso"
}

/// Buscador único por nombre, CURP o `_id` del plan (case-insensitive).
fn coincide_busqueda(plan: &serde_json::Value, consulta: &str) -> bool {
    let q = consulta.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    [
        plan["nombre"].as_str().unwrap_or(""),
        plan["cliente_curp"].as_str().unwrap_or(""),
        plan["_id"].as_str().unwrap_or(""),
    ]
    .iter()
    .any(|campo| campo.to_lowercase().contains(&q))
}

fn coincide_producto(plan: &serde_json::Value, producto: &str) -> bool {
    producto == "Todos" || plan["producto"].as_str().unwrap_or("") == producto
}

fn coincide_estado(plan: &serde_json::Value, estado: &str) -> bool {
    estado == "Todos" || plan["estado"].as_str().unwrap_or("") == estado
}

/// Aplica buscador + filtros de estado y producto a los planes ya cargados.
fn filtrar_planes(
    planes: &[serde_json::Value],
    consulta: &str,
    estado: &str,
    producto: &str,
) -> Vec<serde_json::Value> {
    planes
        .iter()
        .filter(|p| {
            coincide_busqueda(p, consulta)
                && coincide_estado(p, estado)
                && coincide_producto(p, producto)
        })
        .cloned()
        .collect()
}

/// Compara dos planes por la columna ordenable (`fecha`, `monto` o `saldo`).
fn comparar_planes(a: &serde_json::Value, b: &serde_json::Value, campo: &str) -> Ordering {
    match campo {
        "monto" => a["monto_total"]
            .as_f64()
            .unwrap_or(0.0)
            .partial_cmp(&b["monto_total"].as_f64().unwrap_or(0.0))
            .unwrap_or(Ordering::Equal),
        "saldo" => saldo_plan(a).partial_cmp(&saldo_plan(b)).unwrap_or(Ordering::Equal),
        // Fecha ISO "YYYY-MM-DD": el orden lexicográfico es el cronológico.
        _ => a["fecha"].as_str().unwrap_or("").cmp(b["fecha"].as_str().unwrap_or("")),
    }
}

/// Ordena una lista según `(campo, ascendente)`; `None` deja el orden del backend.
fn ordenar_lista(
    mut planes: Vec<serde_json::Value>,
    orden: Option<(String, bool)>,
) -> Vec<serde_json::Value> {
    if let Some((campo, asc)) = orden {
        planes.sort_by(|a, b| {
            let ord = comparar_planes(a, b, &campo);
            if asc {
                ord
            } else {
                ord.reverse()
            }
        });
    }
    planes
}

/// Encabezado clickeable: ordena la tabla por `campo` alternando asc/desc.
#[component]
fn ThOrden(
    mut orden: Signal<Option<(String, bool)>>,
    campo: &'static str,
    label: &'static str,
) -> Element {
    let flecha = match orden() {
        Some((c, asc)) if c == campo => {
            if asc {
                " ▲"
            } else {
                " ▼"
            }
        }
        _ => "",
    };
    rsx! {
        th { class: "py-3 px-4",
            button {
                class: "flex items-center gap-1 font-semibold hover:text-slate-900 dark:hover:text-white",
                onclick: move |_| {
                    orden.set(match orden() {
                        Some((c, asc)) if c == campo => Some((campo.to_string(), !asc)),
                        _ => Some((campo.to_string(), true)),
                    });
                },
                "{label}{flecha}"
            }
        }
    }
}

/// Cabecera de las dos tablas: mismo esquema de columnas, distinto orden propio.
#[component]
fn CabeceraCartera(orden: Signal<Option<(String, bool)>>) -> Element {
    rsx! {
        thead {
            tr { class: "text-slate-500 border-b border-slate-200 dark:text-slate-400 dark:border-slate-700",
                th { class: "py-3 px-4 font-semibold", "Producto" }
                th { class: "py-3 px-4 font-semibold", "Cliente" }
                th { class: "py-3 px-4 font-semibold", "CURP" }
                ThOrden { orden, campo: "monto", label: "Monto Total" }
                th { class: "py-3 px-4 font-semibold", "Plazo" }
                th { class: "py-3 px-4 font-semibold", "Pago Mensual" }
                th { class: "py-3 px-4 font-semibold", "Interés" }
                th { class: "py-3 px-4 font-semibold", "Estado" }
                th { class: "py-3 px-4 font-semibold", "Cubiertas" }
                ThOrden { orden, campo: "saldo", label: "Saldo" }
                th { class: "py-3 px-4 font-semibold", "Acciones" }
                ThOrden { orden, campo: "fecha", label: "Fecha" }
            }
        }
    }
}

#[component]
pub fn Cartera(token: Signal<String>, is_authenticated: Signal<bool>) -> Element {
    let cartera_planes = use_signal(|| Vec::<serde_json::Value>::new());
    let mut cartera_loaded = use_signal(|| false);

    // Buscador + filtros (en memoria, sobre los planes ya cargados). El estado
    // vive en la sesión de la pantalla: vaciar el buscador y volver conserva el
    // filtro activo.
    let mut busqueda = use_signal(|| String::new());
    let mut filtro_estado = use_signal(|| "Todos".to_string());
    let mut filtro_producto = use_signal(|| "Todos".to_string());
    // Orden por columna, independiente para cada tabla.
    let orden_activos = use_signal(|| Option::<(String, bool)>::None);
    let orden_liquidados = use_signal(|| Option::<(String, bool)>::None);

    // Mini-form inline de pago de cuota: un solo formulario abierto a la vez.
    let pago_plan = use_signal(|| Option::<String>::None);
    let pago_cuota = use_signal(|| String::new());
    let pago_monto = use_signal(|| String::new());
    let pago_error = use_signal(|| String::new());
    let pago_enviando = use_signal(|| false);

    // Mini-form inline de abono parcial, independiente del pago de cuota.
    let abono_plan = use_signal(|| Option::<String>::None);
    let abono_monto = use_signal(|| String::new());
    let abono_nota = use_signal(|| String::new());
    let abono_error = use_signal(|| String::new());
    let abono_enviando = use_signal(|| false);

    // Descarga de contrato: el plan que está descargando (uno a la vez) y el
    // último error (visible bajo las tablas). Se escriben dentro de FilaPlan.
    let descargando = use_signal(|| Option::<String>::None);
    let descarga_error = use_signal(|| String::new());

    // Se monta/desmonta al navegar por el menú: refetchea la cartera en cada visita.
    if !cartera_loaded() {
        let token_val = token();
        cartera_loaded.set(true);
        spawn(cargar_planes(token_val, cartera_planes, is_authenticated, token));
    }

    let planes = cartera_planes();
    // Productos distintos ya presentes, para el filtro de producto.
    let mut productos: Vec<String> = planes
        .iter()
        .filter_map(|p| p["producto"].as_str().map(str::to_string))
        .collect();
    productos.sort();
    productos.dedup();

    let filtrados = filtrar_planes(&planes, &busqueda(), &filtro_estado(), &filtro_producto());
    let mut activos = Vec::new();
    let mut liquidados = Vec::new();
    for plan in filtrados {
        if es_activo(plan["estado"].as_str().unwrap_or("")) {
            activos.push(plan);
        } else {
            liquidados.push(plan);
        }
    }
    let activos = ordenar_lista(activos, orden_activos());
    let liquidados = ordenar_lista(liquidados, orden_liquidados());

    // (pago_abierto, abono_abierto, plan) precomputado: rsx no admite `let` en
    // el cuerpo del for.
    let filas_activos: Vec<(bool, bool, serde_json::Value)> = activos
        .iter()
        .map(|p| {
            let idp = p["_id"].as_str().unwrap_or("");
            (
                pago_plan().as_deref() == Some(idp),
                abono_plan().as_deref() == Some(idp),
                p.clone(),
            )
        })
        .collect();
    let filas_liquidados: Vec<(bool, bool, serde_json::Value)> = liquidados
        .iter()
        .map(|p| {
            let idp = p["_id"].as_str().unwrap_or("");
            (
                pago_plan().as_deref() == Some(idp),
                abono_plan().as_deref() == Some(idp),
                p.clone(),
            )
        })
        .collect();

    let input_class = "bg-white border border-slate-300 text-slate-900 rounded-lg px-3 py-2 text-sm outline-none focus:border-blue-500 dark:bg-slate-800 dark:border-slate-600 dark:text-white";
    rsx! {
        div {
            h2 { class: "text-2xl font-bold mb-6 text-slate-900 dark:text-white", "Cartera de Créditos" }
            if planes.is_empty() {
                div { class: "text-slate-500 dark:text-slate-400", "No hay créditos activos para esta empresa." }
            } else {
                div { class: "flex flex-wrap items-end gap-3 mb-6",
                    div { class: "flex flex-col gap-1",
                        label { class: "text-xs text-slate-500 dark:text-slate-400", "Buscar" }
                        input {
                            class: "{input_class} w-72",
                            placeholder: "Nombre, CURP o ID del plan",
                            value: busqueda(),
                            oninput: move |e| busqueda.set(e.value()),
                        }
                    }
                    div { class: "flex flex-col gap-1",
                        label { class: "text-xs text-slate-500 dark:text-slate-400", "Estado" }
                        select {
                            class: "{input_class}",
                            value: filtro_estado(),
                            onchange: move |e| filtro_estado.set(e.value()),
                            option { value: "Todos", "Todos" }
                            option { value: "Activo", "Activo" }
                            option { value: "Moroso", "Moroso" }
                            option { value: "Liquidado", "Liquidado" }
                        }
                    }
                    div { class: "flex flex-col gap-1",
                        label { class: "text-xs text-slate-500 dark:text-slate-400", "Producto" }
                        select {
                            class: "{input_class}",
                            value: filtro_producto(),
                            onchange: move |e| filtro_producto.set(e.value()),
                            option { value: "Todos", "Todos" }
                            for producto in &productos {
                                option { value: "{producto}", "{producto}" }
                            }
                        }
                    }
                }

                // Tabla 1: cartera viva (Activo + Moroso).
                h3 { class: "text-lg font-semibold mb-3 text-slate-900 dark:text-white", "Activos" }
                if filas_activos.is_empty() {
                    div { class: "mb-8 text-slate-500 dark:text-slate-400", "No hay créditos activos con este filtro." }
                } else {
                    div { class: "overflow-x-auto mb-8",
                        table { class: "w-full text-sm text-left bg-white rounded-xl border border-slate-200 dark:bg-slate-900 dark:border-slate-700",
                            CabeceraCartera { orden: orden_activos }
                            tbody {
                                for (pago_abierto, abono_abierto, plan) in filas_activos {
                                    FilaPlan {
                                        plan,
                                        form_pago: pago_abierto,
                                        form_abono: abono_abierto,
                                        token,
                                        is_authenticated,
                                        cartera_planes,
                                        pago_plan,
                                        pago_cuota,
                                        pago_monto,
                                        pago_error,
                                        pago_enviando,
                                        abono_plan,
                                        abono_monto,
                                        abono_nota,
                                        abono_error,
                                        abono_enviando,
                                        descargando,
                                        descarga_error,
                                    }
                                }
                            }
                        }
                    }
                }

                // Tabla 2: liquidados / inactivos.
                h3 { class: "text-lg font-semibold mb-3 text-slate-900 dark:text-white", "Liquidados / inactivos" }
                if filas_liquidados.is_empty() {
                    div { class: "text-slate-500 dark:text-slate-400", "No hay créditos liquidados con este filtro." }
                } else {
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm text-left bg-white rounded-xl border border-slate-200 dark:bg-slate-900 dark:border-slate-700",
                            CabeceraCartera { orden: orden_liquidados }
                            tbody {
                                for (pago_abierto, abono_abierto, plan) in filas_liquidados {
                                    FilaPlan {
                                        plan,
                                        form_pago: pago_abierto,
                                        form_abono: abono_abierto,
                                        token,
                                        is_authenticated,
                                        cartera_planes,
                                        pago_plan,
                                        pago_cuota,
                                        pago_monto,
                                        pago_error,
                                        pago_enviando,
                                        abono_plan,
                                        abono_monto,
                                        abono_nota,
                                        abono_error,
                                        abono_enviando,
                                        descargando,
                                        descarga_error,
                                    }
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

/// Fila de un plan + sus mini-forms inline (pago de cuota y abono), renderizados
/// como segunda fila de la tabla cuando correspondan. Componente propio para que
/// los closures de evento capturen valores owned.
#[component]
#[allow(clippy::too_many_arguments)]
fn FilaPlan(
    plan: serde_json::Value,
    form_pago: bool,
    form_abono: bool,
    token: Signal<String>,
    is_authenticated: Signal<bool>,
    cartera_planes: Signal<Vec<serde_json::Value>>,
    mut pago_plan: Signal<Option<String>>,
    mut pago_cuota: Signal<String>,
    mut pago_monto: Signal<String>,
    mut pago_error: Signal<String>,
    mut pago_enviando: Signal<bool>,
    mut abono_plan: Signal<Option<String>>,
    mut abono_monto: Signal<String>,
    mut abono_nota: Signal<String>,
    mut abono_error: Signal<String>,
    mut abono_enviando: Signal<bool>,
    mut descargando: Signal<Option<String>>,
    mut descarga_error: Signal<String>,
) -> Element {
    let id = plan["_id"].as_str().unwrap_or("").to_string();
    // Clones por closure: cada handler mueve su copia.
    let id_pago = id.clone();
    let id_abono = id.clone();
    let id_descarga = id;
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
    let saldo = saldo_plan(&plan);
    let cobrado = cobrado_plan(&plan);
    let nombre = nombre_plan(&plan);
    let siguiente = siguiente_cuota_impaga(plazo, pagadas);
    // Opciones del select: todas las cuotas impagas, empezando por la siguiente.
    let cuotas: Vec<i64> = siguiente.map(|s| (s..=plazo).collect()).unwrap_or_default();
    let input_class = "bg-white border border-slate-300 text-slate-900 rounded-lg px-3 py-2 text-sm outline-none focus:border-blue-500 dark:bg-slate-800 dark:border-slate-600 dark:text-white";
    // Un plan liquidado no admite pago de cuota ni abono (el backend los rechaza).
    let puede_pagar = siguiente.is_some() && estado != "Liquidado";
    let puede_abonar = estado != "Liquidado";
    let saldo_inicial = format!("{saldo}");
    rsx! {
        tr { class: "border-b border-slate-200 text-slate-700 dark:border-slate-700/50 dark:text-slate-300",
            td { class: "py-3 px-4", "{plan[\"producto\"].as_str().unwrap_or(\"—\")}" }
            td { class: "py-3 px-4 font-medium", "{nombre}" }
            td { class: "py-3 px-4 font-mono", "{curp}" }
            td { class: "py-3 px-4", "${plan[\"monto_total\"].as_f64().unwrap_or(0.0)} MXN" }
            td { class: "py-3 px-4", "{plazo} meses" }
            td { class: "py-3 px-4", "${pago_mensual} MXN" }
            td { class: "py-3 px-4", "{(plan[\"tasa_interes\"].as_f64().unwrap_or(0.0) * 100.0) as i32}%" }
            td { class: "py-3 px-4",
                span { class: format!("px-2 py-1 rounded-full text-xs {badge}"), "{estado}" }
            }
            td { class: "py-3 px-4",
                div { class: "text-xs", "Cuota {pagadas}/{plazo} cubiertas" }
                if vencidas > 0 {
                    div { class: "text-xs text-red-600 dark:text-red-400", "{vencidas} vencidas" }
                }
            }
            td { class: "py-3 px-4 font-medium", title: "Cobrado: ${cobrado} MXN", "${saldo} MXN" }
            td { class: "py-3 px-4",
                div { class: "flex flex-col items-start gap-1.5",
                    if puede_pagar {
                        button {
                            class: "bg-blue-600 hover:bg-blue-700 text-white text-xs font-semibold px-3 py-1.5 rounded-lg",
                            onclick: move |_| {
                                pago_plan.set(Some(id_pago.clone()));
                                abono_plan.set(None);
                                pago_monto.set(if pago_mensual > 0.0 { format!("{pago_mensual}") } else { String::new() });
                                pago_cuota.set(siguiente.map(|c| c.to_string()).unwrap_or_default());
                                pago_error.set(String::new());
                            },
                            "Registrar pago"
                        }
                    }
                    if puede_abonar {
                        button {
                            class: "bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-semibold px-3 py-1.5 rounded-lg",
                            onclick: move |_| {
                                abono_plan.set(Some(id_abono.clone()));
                                pago_plan.set(None);
                                abono_monto.set(saldo_inicial.clone());
                                abono_nota.set(String::new());
                                abono_error.set(String::new());
                            },
                            "Registrar abono"
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
        if form_pago {
            tr {
                td { colspan: "12", class: "px-4 pb-4",
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
                                    r#type: "number",
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
                                                        // Refresca la lista: badges, cuotas y saldo al día.
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
        if form_abono {
            tr {
                td { colspan: "12", class: "px-4 pb-4",
                    div { class: "bg-emerald-50 border border-emerald-200 rounded-lg p-4 dark:bg-emerald-900/20 dark:border-emerald-800/50",
                        div { class: "text-xs text-emerald-700 dark:text-emerald-300 font-semibold mb-3", "Abono parcial o total — no marca la cuota como pagada" }
                        div { class: "flex flex-wrap items-end gap-3",
                            div { class: "flex flex-col gap-1",
                                label { class: "text-xs text-slate-500 dark:text-slate-400", "Monto del abono ($)" }
                                input {
                                    class: "{input_class}",
                                    r#type: "number",
                                    value: abono_monto(),
                                    oninput: move |e| abono_monto.set(e.value()),
                                }
                            }
                            div { class: "flex flex-col gap-1 flex-1 min-w-48",
                                label { class: "text-xs text-slate-500 dark:text-slate-400", "Nota (opcional)" }
                                input {
                                    class: "{input_class}",
                                    value: abono_nota(),
                                    placeholder: "p. ej. abono semanal",
                                    oninput: move |e| abono_nota.set(e.value()),
                                }
                            }
                            button {
                                class: "bg-emerald-600 hover:bg-emerald-700 text-white text-sm font-semibold px-4 py-2 rounded-lg",
                                disabled: abono_enviando(),
                                onclick: move |_| {
                                    let token_val = token();
                                    let Some(plan_id) = abono_plan() else { return };
                                    let monto = match abono_monto().trim().parse::<f64>() {
                                        Ok(m) if m > 0.0 => m,
                                        _ => {
                                            abono_error.set("Monto inválido".to_string());
                                            return;
                                        }
                                    };
                                    let nota = abono_nota();
                                    abono_enviando.set(true);
                                    abono_error.set(String::new());
                                    spawn(async move {
                                        match registrar_abono(&plan_id, monto, &nota, &token_val).send().await {
                                            Ok(res) => {
                                                if sesion_ok(&res, is_authenticated, token) {
                                                    if res.status().is_success() {
                                                        abono_plan.set(None);
                                                        abono_error.set(String::new());
                                                        // Refresca saldo y estado al día.
                                                        spawn(cargar_planes(
                                                            token_val.clone(),
                                                            cartera_planes,
                                                            is_authenticated,
                                                            token,
                                                        ));
                                                    } else if let Ok(data) = res.json::<serde_json::Value>().await {
                                                        abono_error.set(data["message"]
                                                            .as_str()
                                                            .unwrap_or("No se pudo registrar el abono")
                                                            .to_string());
                                                    } else {
                                                        abono_error.set("No se pudo registrar el abono".to_string());
                                                    }
                                                }
                                            }
                                            Err(_) => abono_error.set("Sin conexión con el servidor".to_string()),
                                        }
                                        abono_enviando.set(false);
                                    });
                                },
                                if abono_enviando() { "Registrando…" } else { "Registrar abono" }
                            }
                            button {
                                class: "bg-slate-200 hover:bg-slate-300 text-slate-700 text-sm font-semibold px-4 py-2 rounded-lg dark:bg-slate-700 dark:hover:bg-slate-600 dark:text-white",
                                onclick: move |_| {
                                    abono_plan.set(None);
                                    abono_error.set(String::new());
                                },
                                "Cancelar"
                            }
                        }
                        if !abono_error().is_empty() {
                            div { class: "mt-3 text-sm text-red-600 dark:text-red-400", "{abono_error}" }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(nombre: &str, curp: &str, id: &str, estado: &str, producto: &str, monto: f64, saldo: f64, fecha: &str) -> serde_json::Value {
        serde_json::json!({
            "_id": id,
            "nombre": nombre,
            "cliente_curp": curp,
            "estado": estado,
            "producto": producto,
            "monto_total": monto,
            "saldo": saldo,
            "fecha": fecha,
        })
    }

    #[test]
    fn es_activo_incluye_moroso_y_excluye_liquidado() {
        assert!(es_activo("Activo"));
        assert!(es_activo("Moroso"));
        assert!(!es_activo("Liquidado"));
        assert!(!es_activo(""));
    }

    #[test]
    fn coincide_busqueda_por_nombre_curp_o_id_sin_importar_mayusculas() {
        let p = plan("María García", "GARM980412HDFNRL05", "665f1a2b3c4d5e6f7a8b9c0d", "Activo", "Crédito", 1000.0, 500.0, "2026-01-01");
        assert!(coincide_busqueda(&p, ""));
        assert!(coincide_busqueda(&p, "  "));
        assert!(coincide_busqueda(&p, "maría"));
        assert!(coincide_busqueda(&p, "garm980412"));
        assert!(coincide_busqueda(&p, "665F1A2B"));
        assert!(!coincide_busqueda(&p, "zzz"));
    }

    #[test]
    fn filtrar_planes_combina_busqueda_estado_y_producto() {
        let planes = vec![
            plan("Ana", "AAAA", "id1", "Activo", "Moto", 1000.0, 400.0, "2026-01-01"),
            plan("Beto", "BBBB", "id2", "Moroso", "Moto", 2000.0, 1500.0, "2026-02-01"),
            plan("Caro", "CCCC", "id3", "Liquidado", "Refri", 3000.0, 0.0, "2026-03-01"),
        ];
        assert_eq!(filtrar_planes(&planes, "", "Todos", "Todos").len(), 3);
        assert_eq!(filtrar_planes(&planes, "", "Liquidado", "Todos").len(), 1);
        assert_eq!(filtrar_planes(&planes, "", "Todos", "Moto").len(), 2);
        // producto + estado + búsqueda se aplican juntos
        let r = filtrar_planes(&planes, "beto", "Moroso", "Moto");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0]["_id"], "id2");
        assert!(filtrar_planes(&planes, "zzz", "Todos", "Todos").is_empty());
        // la lista se puede vaciar y volver a poblar sin perder consistencia
        assert!(filtrar_planes(&planes, "nadie", "Todos", "Todos").is_empty());
        assert_eq!(filtrar_planes(&planes, "", "Todos", "Todos").len(), 3);
    }

    #[test]
    fn ordenar_lista_por_monto_saldo_y_fecha() {
        let planes = vec![
            plan("Ana", "AAAA", "id1", "Activo", "Moto", 1000.0, 400.0, "2026-03-01"),
            plan("Beto", "BBBB", "id2", "Activo", "Moto", 3000.0, 100.0, "2026-01-01"),
            plan("Caro", "CCCC", "id3", "Activo", "Moto", 2000.0, 900.0, "2026-02-01"),
        ];
        let por_monto = ordenar_lista(planes.clone(), Some(("monto".into(), true)));
        assert_eq!(por_monto.iter().map(|p| p["_id"].as_str().unwrap()).collect::<Vec<_>>(), ["id1", "id3", "id2"]);
        let por_saldo_desc = ordenar_lista(planes.clone(), Some(("saldo".into(), false)));
        assert_eq!(por_saldo_desc[0]["_id"], "id3");
        let por_fecha = ordenar_lista(planes.clone(), Some(("fecha".into(), true)));
        assert_eq!(por_fecha.iter().map(|p| p["_id"].as_str().unwrap()).collect::<Vec<_>>(), ["id2", "id3", "id1"]);
        // sin orden se respeta el orden del backend
        assert_eq!(ordenar_lista(planes, None)[0]["_id"], "id1");
    }
}
