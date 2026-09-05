//! Primitivas de gráficas en SVG puro dentro de rsx — CERO librería de charts,
//! CERO dependencias (decisión de plan 2026-08-31).
//!
//! ponytail: sin interactividad (tooltips/zoom); si se necesita, migrar a
//! librería. Colores de series en hex (válido en claro y oscuro); el texto va
//! con `fill: "currentColor"` + clases Tailwind para respetar el tema.

use dioxus::prelude::*;

/// Par de series apiladas de un mes (cobrado vs por cobrar).
#[derive(Clone, PartialEq)]
pub struct DatoMes {
    pub mes: String,
    pub cobrado: f64,
    pub por_cobrar: f64,
}

/// Categoría etiquetada: punto de línea, sector de donut o barra horizontal.
#[derive(Clone, PartialEq)]
pub struct DatoCategoria {
    pub etiqueta: String,
    pub valor: f64,
}

/// Monto compacto para etiquetas: "$85", "$1.2k", "$1.2M".
pub fn fmt_monto(v: f64) -> String {
    let a = v.abs();
    if a >= 1_000_000.0 {
        format!("${:.1}M", v / 1_000_000.0)
    } else if a >= 1_000.0 {
        format!("${:.1}k", v / 1_000.0)
    } else {
        format!("${:.0}", v)
    }
}

/// Semáforo de morosidad: <5% verde, 5-20% ámbar, >20% rojo (clases Tailwind).
pub fn semaforo_morosidad(tasa: f64) -> &'static str {
    if tasa < 0.05 {
        "text-emerald-600 dark:text-emerald-400"
    } else if tasa <= 0.20 {
        "text-amber-600 dark:text-amber-400"
    } else {
        "text-red-600 dark:text-red-400"
    }
}

/// Recorta etiquetas largas (nombres) para que quepan junto a la barra.
pub fn truncar(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}

fn vacio() -> Element {
    rsx! { div { class: "text-slate-400 text-sm py-8 text-center dark:text-slate-500", "Sin datos" } }
}

/// Barras apiladas de 2 series por mes: cobrado (emerald, abajo) vs por cobrar
/// (blue, arriba), con total encima y leyenda.
#[component]
pub fn BarraApilada(datos: Vec<DatoMes>) -> Element {
    if datos.is_empty() {
        return vacio();
    }
    let n = datos.len() as f64;
    let slot = 384.0 / n;
    let bar_w = (slot * 0.62).min(52.0);
    let max = datos
        .iter()
        .map(|d| d.cobrado + d.por_cobrar)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    // Geometría precomputada por mes (rsx no admite let en el cuerpo del for).
    // (mes, x, x_centro, y_cobrado, y_tope, w, h_cobrado, h_por_cobrar, total)
    let filas: Vec<(String, f64, f64, f64, f64, f64, f64, f64, f64)> = datos
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let x = 8.0 + i as f64 * slot + (slot - bar_w) / 2.0;
            let ha = d.cobrado / max * 170.0;
            let hb = d.por_cobrar / max * 170.0;
            (
                d.mes.clone(),
                x,
                x + bar_w / 2.0,
                190.0 - ha,
                190.0 - ha - hb,
                bar_w,
                ha,
                hb,
                d.cobrado + d.por_cobrar,
            )
        })
        .collect();
    rsx! {
        div {
            svg { class: "w-full text-slate-400 dark:text-slate-500", view_box: "0 0 400 230",
                line { x1: 8, y1: 190.5, x2: 392, y2: 190.5, stroke: "currentColor", stroke_width: 1 }
                for (mes, x, xc, ya, ytop, w, ha, hb, total) in &filas {
                    if *total > 0.0 {
                        text {
                            x: format!("{xc:.1}"), y: format!("{ytop:.1}"),
                            text_anchor: "middle", font_size: "10", fill: "currentColor",
                            class: "text-slate-500 dark:text-slate-400",
                            {fmt_monto(*total)}
                        }
                    }
                    if *ha > 0.0 {
                        rect {
                            x: format!("{x:.1}"), y: format!("{ya:.1}"),
                            width: format!("{w:.1}"), height: format!("{ha:.1}"),
                            fill: "#10b981", rx: 2,
                        }
                    }
                    if *hb > 0.0 {
                        rect {
                            x: format!("{x:.1}"), y: format!("{ytop:.1}"),
                            width: format!("{w:.1}"), height: format!("{hb:.1}"),
                            fill: "#3b82f6", rx: 2,
                        }
                    }
                    text {
                        x: format!("{xc:.1}"), y: "210",
                        text_anchor: "middle", font_size: "10", fill: "currentColor",
                        {mes.clone()}
                    }
                }
            }
            div { class: "mt-2 flex items-center gap-4 text-xs text-slate-600 dark:text-slate-300",
                div { class: "flex items-center gap-1.5",
                    div { class: "h-2.5 w-2.5 rounded-sm bg-emerald-500" }
                    "Cobrado"
                }
                div { class: "flex items-center gap-1.5",
                    div { class: "h-2.5 w-2.5 rounded-sm bg-blue-500" }
                    "Por cobrar"
                }
            }
        }
    }
}

/// Línea con puntos y etiquetas (flujo de caja proyectado).
#[component]
pub fn Linea(datos: Vec<DatoCategoria>, color: String) -> Element {
    if datos.is_empty() {
        return vacio();
    }
    let pasos = (datos.len().max(2) - 1) as f64;
    let max = datos.iter().map(|d| d.valor).fold(0.0_f64, f64::max).max(1.0);
    let pts: Vec<String> = datos
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let x = 40.0 + i as f64 * (320.0 / pasos);
            let y = 160.0 - d.valor / max * 136.0;
            format!("{x:.1},{y:.1}")
        })
        .collect();
    let puntos: Vec<(f64, f64, f64)> = datos
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let x = 40.0 + i as f64 * (320.0 / pasos);
            let y = 160.0 - d.valor / max * 136.0;
            (x, y, y - 12.0)
        })
        .collect();
    rsx! {
        svg { class: "w-full text-slate-400 dark:text-slate-500", view_box: "0 0 400 200",
            line { x1: 40, y1: 160.5, x2: 360, y2: 160.5, stroke: "currentColor", stroke_width: 1, stroke_dasharray: "4 4" }
            polyline {
                points: pts.join(" "), fill: "none", stroke: "{color}",
                stroke_width: 3, stroke_linecap: "round", stroke_linejoin: "round",
            }
            for (i, (x, y, yval)) in puntos.iter().enumerate() {
                circle { cx: format!("{x:.1}"), cy: format!("{y:.1}"), r: 5, fill: "{color}" }
                text {
                    x: format!("{x:.1}"), y: format!("{yval:.1}"),
                    text_anchor: "middle", font_size: "11", fill: "currentColor",
                    class: "text-slate-700 dark:text-slate-200",
                    {fmt_monto(datos[i].valor)}
                }
                text {
                    x: format!("{x:.1}"), y: "184",
                    text_anchor: "middle", font_size: "11", fill: "currentColor",
                    {datos[i].etiqueta.clone()}
                }
            }
        }
    }
}

/// Donut por stroke-dasharray con total al centro y leyenda (distribución de
/// montos). Paleta fija: blue, emerald, amber, red.
#[component]
pub fn Donut(datos: Vec<DatoCategoria>) -> Element {
    let total: f64 = datos.iter().map(|d| d.valor).sum();
    if datos.is_empty() || total <= 0.0 {
        return vacio();
    }
    const PALETA: [&str; 4] = ["#3b82f6", "#10b981", "#f59e0b", "#ef4444"];
    let c = 2.0 * std::f64::consts::PI * 70.0;
    let mut acc = 0.0_f64;
    // (fill, dasharray, dashoffset) por sector.
    let segmentos: Vec<(String, String, String)> = datos
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let len = d.valor / total * c;
            let off = acc;
            acc += len;
            let dibuja = (len - 2.0).max(0.0);
            (
                PALETA[i % 4].to_string(),
                format!("{dibuja:.2} {:.2}", c - dibuja),
                format!("{:.2}", -off),
            )
        })
        .collect();
    rsx! {
        div {
            svg { class: "mx-auto w-48 text-slate-500 dark:text-slate-400", view_box: "0 0 200 200",
                g { transform: "rotate(-90 100 100)",
                    for (fill, dash, off) in segmentos.iter() {
                        circle {
                            cx: 100, cy: 100, r: 70, fill: "none",
                            stroke: "{fill}", stroke_width: 30,
                            stroke_dasharray: "{dash}", stroke_dashoffset: "{off}",
                        }
                    }
                }
                text {
                    x: 100, y: 96, text_anchor: "middle", font_size: "28", font_weight: "700",
                    fill: "currentColor", class: "text-slate-900 dark:text-white",
                    {format!("{}", total as i64)}
                }
                text {
                    x: 100, y: 116, text_anchor: "middle", font_size: "11", fill: "currentColor",
                    class: "text-slate-500 dark:text-slate-400",
                    "créditos"
                }
            }
            div { class: "mt-3 flex flex-wrap justify-center gap-x-4 gap-y-1 text-xs text-slate-600 dark:text-slate-300",
                for (i, d) in datos.iter().enumerate() {
                    div { class: "flex items-center gap-1.5",
                        div {
                            style: format!("background-color: {}", PALETA[i % 4]),
                            class: "h-2.5 w-2.5 rounded-full",
                        }
                        {format!("{}: {}", d.etiqueta, d.valor)}
                    }
                }
            }
        }
    }
}

/// Barras horizontales con etiqueta y valor (aging, top deudores).
#[component]
pub fn BarraH(datos: Vec<DatoCategoria>, color: String) -> Element {
    if datos.is_empty() {
        return vacio();
    }
    let alto = format!("{:.0}", datos.len() as f64 * 34.0 + 10.0);
    let max = datos.iter().map(|d| d.valor).fold(0.0_f64, f64::max).max(1.0);
    // (y_texto, y_barra, x_valor, w) por fila.
    let filas: Vec<(f64, f64, f64, f64)> = datos
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let y = i as f64 * 34.0;
            let w = (d.valor / max * 180.0).max(if d.valor > 0.0 { 2.0 } else { 0.0 });
            (y + 19.0, y + 6.0, 150.0 + w + 6.0, w)
        })
        .collect();
    rsx! {
        svg { class: "w-full text-slate-500 dark:text-slate-400", view_box: "0 0 400 {alto}",
            for (i, (ytxt, ybar, xval, w)) in filas.iter().enumerate() {
                text {
                    x: "0", y: format!("{ytxt:.1}"),
                    font_size: "11", fill: "currentColor",
                    {truncar(&datos[i].etiqueta, 18)}
                }
                if *w > 0.0 {
                    rect {
                        x: "150", y: format!("{ybar:.1}"),
                        width: format!("{w:.1}"), height: "18",
                        fill: "{color}", rx: 3,
                    }
                }
                text {
                    x: format!("{xval:.1}"), y: format!("{ytxt:.1}"),
                    font_size: "11", fill: "currentColor",
                    class: "text-slate-700 dark:text-slate-200",
                    {fmt_monto(datos[i].valor)}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_monto_compacta_miles_y_millones() {
        assert_eq!(fmt_monto(0.0), "$0");
        assert_eq!(fmt_monto(85.4), "$85");
        assert_eq!(fmt_monto(1234.5), "$1.2k");
        assert_eq!(fmt_monto(1_200_000.0), "$1.2M");
    }

    #[test]
    fn semaforo_morosidad_verde_ambar_rojo() {
        assert!(semaforo_morosidad(0.049).contains("emerald"));
        assert!(semaforo_morosidad(0.05).contains("amber"));
        assert!(semaforo_morosidad(0.20).contains("amber"));
        assert!(semaforo_morosidad(0.21).contains("red"));
    }

    #[test]
    fn truncar_recorta_largas_y_deja_cortas_intactas() {
        let largo = "María Guadalupe González Pérez";
        assert_eq!(truncar(largo, 18), format!("{}…", largo.chars().take(18).collect::<String>()));
        assert_eq!(truncar("Ana", 18), "Ana");
    }
}
