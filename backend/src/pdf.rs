//! Contrato PDF (ola 6): función pura con `printpdf` y fuentes base14
//! (WinAnsiEncoding ≈ latin1: los acentos pasan; caracteres no codificables
//! los descarta el crate). Sin logos, imágenes ni motor de layout.
//! ponytail: el PDF se regenera siempre bajo demanda desde datos vivos, no se
//! almacena; techo: firma electrónica o logo si el negocio lo pide.

use printpdf::{
    BuiltinFont, IndirectFontRef, Mm, PdfDocument, PdfDocumentReference, PdfLayerReference,
};

use crate::models::credito::{Pago, PlanPago};
use crate::routes::credito::generar_plan_pagos;

/// A4 en mm.
const A4: Mm = Mm(210.0);
const A4_ALTO: Mm = Mm(297.0);

fn fuente(doc: &PdfDocumentReference, b: BuiltinFont) -> IndirectFontRef {
    doc.add_builtin_font(b).expect("fuente base14 incluida en el crate")
}

/// Escribe una línea en (x, y) mm desde abajo-izquierda.
fn escribe(capa: &PdfLayerReference, f: &IndirectFontRef, tam: f32, x: f32, y: f32, txt: &str) {
    capa.begin_text_section();
    capa.set_font(f, tam);
    capa.set_text_cursor(Mm(x), Mm(y));
    capa.write_text(txt, f);
    capa.end_text_section();
}

/// Genera el PDF del contrato de crédito (función PURA): título, fecha de
/// emisión, datos de empresa y cliente, datos del crédito, resumen de cobranza
/// (cobrado/saldo + sello `LIQUIDADO — FINIQUITO` si saldo ≤ 0.01), sección de
/// pagos/abonos registrados y tabla completa de amortización (regenerada con
/// la MISMA fórmula de `evaluar`). La tabla de pagos (izquierda) y la de
/// abonos (derecha) truncan a la altura de la firma; ponytail: si un plan
/// tiene muchas cuotas/abonos no caben en una página — techo: paginación si el
/// negocio pide plazos/abonos largos.
pub fn pdf_contrato(
    empresa_nombre: &str,
    empresa_correo: &str,
    cliente_nombre: &str,
    cliente_curp: &str,
    plan: &PlanPago,
    pagos: &[Pago],
    cobrado: f64,
    saldo: f64,
    fecha_emision: &str,
) -> Vec<u8> {
    let (doc, pagina, capa_idx) = PdfDocument::new(
        "CONTRATO DE CREDITO PYMZA", A4, A4_ALTO, "Contrato",
    );
    let capa = doc.get_page(pagina).get_layer(capa_idx);
    let normal = fuente(&doc, BuiltinFont::Helvetica);
    let negrita = fuente(&doc, BuiltinFont::HelveticaBold);
    let mono = fuente(&doc, BuiltinFont::Courier);

    // Misma fórmula de `evaluar_credito`: el plan guardado no persiste la tabla.
    let filas = generar_plan_pagos(plan.monto_total, plan.plazo_meses, plan.tasa_interes);

    let mut y: f32 = 282.0;
    escribe(&capa, &negrita, 18.0, 20.0, y, "CONTRATO DE CRÉDITO"); y -= 10.0;
    escribe(&capa, &normal, 10.0, 20.0, y, &format!("Fecha de emisión: {fecha_emision}")); y -= 12.0;

    escribe(&capa, &negrita, 12.0, 20.0, y, "Datos de la empresa"); y -= 8.0;
    escribe(&capa, &normal, 10.0, 20.0, y, &format!("Nombre: {empresa_nombre}")); y -= 7.0;
    escribe(&capa, &normal, 10.0, 20.0, y, &format!("Correo: {empresa_correo}")); y -= 10.0;

    escribe(&capa, &negrita, 12.0, 20.0, y, "Datos del cliente"); y -= 8.0;
    escribe(&capa, &normal, 10.0, 20.0, y, &format!("Nombre: {cliente_nombre}")); y -= 7.0;
    escribe(&capa, &normal, 10.0, 20.0, y, &format!("CURP: {cliente_curp}")); y -= 10.0;

    escribe(&capa, &negrita, 12.0, 20.0, y, "Datos del crédito"); y -= 8.0;
    escribe(&capa, &normal, 10.0, 20.0, y, &format!("Producto: {}", plan.producto)); y -= 7.0;
    escribe(&capa, &normal, 10.0, 20.0, y, &format!("Monto total: ${:.2}", plan.monto_total)); y -= 7.0;
    escribe(&capa, &normal, 10.0, 20.0, y, &format!("Plazo: {} meses", plan.plazo_meses)); y -= 7.0;
    escribe(&capa, &normal, 10.0, 20.0, y, &format!("Tasa de interés: {:.0}%", plan.tasa_interes * 100.0)); y -= 7.0;
    escribe(&capa, &normal, 10.0, 20.0, y, &format!("Pago mensual: ${:.2}", plan.pago_mensual)); y -= 10.0;

    // Ola 7: resumen de cobranza + sello de finiquito.
    escribe(&capa, &negrita, 12.0, 20.0, y, "Resumen de cobranza"); y -= 8.0;
    escribe(&capa, &normal, 10.0, 20.0, y, &format!("Cobrado: ${cobrado:.2}")); y -= 7.0;
    escribe(&capa, &normal, 10.0, 20.0, y, &format!("Saldo pendiente: ${saldo:.2}")); y -= 8.0;
    if saldo <= 0.01 {
        escribe(&capa, &negrita, 14.0, 20.0, y, "LIQUIDADO — FINIQUITO"); y -= 12.0;
    }

    // Dos columnas: tabla de amortización a la izquierda, pagos/abonos a la
    // derecha; ambas truncan a la altura de la firma (y_tope).
    const X_IZQ: f32 = 20.0;
    const X_DER: f32 = 128.0;
    const Y_TOPE: f32 = 45.0;

    let mut y_t = y;
    escribe(&capa, &negrita, 12.0, X_IZQ, y_t, "Tabla de pagos"); y_t -= 8.0;
    escribe(
        &capa, &mono, 10.0, X_IZQ, y_t,
        &format!("{:<4}{:>10}{:>11}{:>11}{:>11}", "Mes", "Pago", "Interés", "Capital", "Saldo"),
    );
    y_t -= 8.0;
    let mut truncada = false;
    for p in &filas {
        if y_t < Y_TOPE {
            truncada = true;
            break;
        }
        escribe(
            &capa, &mono, 10.0, X_IZQ, y_t,
            &format!(
                "{:<4}{:>10.2}{:>11.2}{:>11.2}{:>11.2}",
                p.mes, p.pago, p.interes, p.capital, p.saldo_restante
            ),
        );
        y_t -= 8.0;
    }
    if truncada {
        escribe(
            &capa, &normal, 9.0, X_IZQ, y_t,
            &format!("(tabla truncada: el plan tiene {} pagos y no caben en una página)", filas.len()),
        );
    }

    // Sección de pagos/abonos (ola 7): fecha, tipo y monto de cada registro.
    let mut y_a = y;
    escribe(&capa, &negrita, 12.0, X_DER, y_a, "Pagos y abonos"); y_a -= 8.0;
    escribe(
        &capa, &mono, 10.0, X_DER, y_a,
        &format!("{:<12}{:<8}{:>10}", "Fecha", "Tipo", "Monto"),
    );
    y_a -= 8.0;
    if pagos.is_empty() {
        escribe(&capa, &normal, 9.0, X_DER, y_a, "(sin pagos registrados)"); y_a -= 8.0;
    }
    let mut truncados = false;
    for p in pagos {
        if y_a < Y_TOPE {
            truncados = true;
            break;
        }
        escribe(
            &capa, &mono, 9.0, X_DER, y_a,
            &format!("{:<12}{:<8}{:>10.2}", p.fecha, p.tipo, p.monto),
        );
        y_a -= 7.0;
    }
    if truncados {
        escribe(&capa, &normal, 9.0, X_DER, y_a, "(más pagos no mostrados)");
    }

    // Línea de firma con guiones bajos: cero API extra de formas/gráficos del crate.
    y = y_t.min(y_a) - 14.0;
    escribe(&capa, &normal, 11.0, 20.0, y, "________________________________"); y -= 8.0;
    escribe(&capa, &normal, 11.0, 20.0, y, "Firma del cliente"); y -= 16.0;
    escribe(&capa, &normal, 9.0, 20.0, y, "Contrato generado por PYMZA");

    doc.save_to_bytes().expect("serialización de un documento PDF bien formado")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_ejemplo() -> PlanPago {
        PlanPago {
            id: None,
            empresa: "demo@pymza.mx".into(),
            cliente_curp: "GARM980412HDFNRL05".into(),
            producto: "Crédito comercial".into(),
            monto_total: 10600.0,
            plazo_meses: 6,
            pago_mensual: 1766.67,
            tasa_interes: 0.06,
            estado: "Activo".into(),
            fecha: "2026-01-01".into(),
        }
    }

    /// printpdf codifica el texto builtin en WinAnsi (≈latin1) y lopdf lo
    /// escribe en el stream como hex-string `<...>`; en `cargo test` (build de
    /// debug) printpdf NO comprime los streams, así que el hex queda legible
    /// en los bytes. Aquí rehacemos esa codificación: cada char se toma como
    /// codepoint latin1 de 1 byte (válido para el texto ascii+acentos con el
    /// que testea; WinAnsi coincide con latin1 en todas las vocales
    /// acentuadas europeas, y los guiones largos van a su byte WinAnsi).
    /// Comparación case-insensitiva (el caso de los dígitos hex lo decide el
    /// serializador, y aquí fue mayúsculas).
    fn byte_winansi(c: char) -> u8 {
        match c {
            '\u{2014}' => 0x97, // em dash
            '\u{2013}' => 0x96, // en dash
            _ => {
                let code = c as u32;
                assert!(code <= 0xFF, "el helper de test solo cubre texto latin1 de 1 byte por carácter: '{}'", c);
                code as u8
            }
        }
    }

    fn contiene_texto(bytes: &[u8], texto: &str) -> bool {
        let hex: String = texto.chars().map(|c| format!("{:02X}", byte_winansi(c))).collect();
        let hay: String = String::from_utf8_lossy(bytes).to_lowercase();
        hay.contains(&hex.to_lowercase())
    }

    fn pago(fecha: &str, tipo: &str, cuota: i32, monto: f64) -> Pago {
        Pago {
            plan_id: mongodb::bson::oid::ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap(),
            empresa: "demo@pymza.mx".into(),
            cliente_curp: "GARM980412HDFNRL05".into(),
            cuota,
            monto,
            fecha: fecha.into(),
            tipo: tipo.into(),
            nota: None,
        }
    }

    fn pdf_ejemplo() -> Vec<u8> {
        let pagos = vec![
            pago("2026-02-01", "cuota", 1, 1766.67),
            pago("2026-02-15", "abono", 0, 500.0),
        ];
        let cobrado = 2266.67;
        let saldo = 10600.02 - cobrado;
        pdf_contrato(
            "Ferretería El Tornillo",
            "demo@pymza.mx",
            "María García Rodríguez",
            "GARM980412HDFNRL05",
            &plan_ejemplo(),
            &pagos,
            cobrado,
            saldo,
            "2026-09-06",
        )
    }

    #[test]
    fn pdf_tiene_header_y_pesa_mas_de_1kb() {
        let bytes = pdf_ejemplo();
        assert!(bytes.starts_with(b"%PDF-"), "header PDF esperado, hay {:?}", &bytes[..8]);
        assert!(bytes.len() > 1024, "un contrato con la tabla pesa más de 1KB, hay {}", bytes.len());
    }

    #[test]
    fn pdf_incluye_el_titulo_con_acento_en_winansi() {
        let bytes = pdf_ejemplo();
        // "CRÉDITO": la É viaja como 0xC9 (WinAnsi) dentro del hex-string del stream
        assert!(contiene_texto(&bytes, "CONTRATO DE CRÉDITO"), "título acentuado no encontrado en el stream");
    }

    #[test]
    fn pdf_incluye_seccion_de_pagos_y_abonos() {
        let bytes = pdf_ejemplo();
        assert!(contiene_texto(&bytes, "Pagos y abonos"), "título de sección ausente");
        assert!(contiene_texto(&bytes, "2026-02-15"), "fecha de abono ausente");
        assert!(contiene_texto(&bytes, "abono"), "tipo de abono ausente");
        assert!(contiene_texto(&bytes, "500.00"), "monto del abono ausente");
        assert!(contiene_texto(&bytes, "Cobrado: $2266.67"), "cobrado de emisión ausente");
        assert!(!contiene_texto(&bytes, "LIQUIDADO"), "plan con saldo no debe sellarse liquidado");
    }

    #[test]
    fn pdf_marca_liquidado_cuando_saldo_cero() {
        let plan = plan_ejemplo();
        let pagos = vec![pago("2026-02-01", "abono", 0, 10600.02)];
        let bytes = pdf_contrato(
            "Ferretería El Tornillo",
            "demo@pymza.mx",
            "María García Rodríguez",
            "GARM980412HDFNRL05",
            &plan,
            &pagos,
            10600.02,
            0.0,
            "2026-09-06",
        );
        assert!(contiene_texto(&bytes, "LIQUIDADO — FINIQUITO"), "sello de finiquito ausente");
        assert!(contiene_texto(&bytes, "Saldo pendiente: $0.00"), "saldo cero de emisión ausente");
    }

    #[test]
    fn pdf_incluye_datos_del_tenant_y_tabla() {
        let bytes = pdf_ejemplo();
        assert!(contiene_texto(&bytes, "María García Rodríguez"), "nombre del cliente ausente");
        assert!(contiene_texto(&bytes, "GARM980412HDFNRL05"), "CURP ausente");
        assert!(contiene_texto(&bytes, "demo@pymza.mx"), "correo de la empresa ausente");
        // encabezado de la tabla con acento: "Interés" (í = 0xED)
        assert!(contiene_texto(&bytes, "Interés"), "encabezado de tabla ausente");
    }
}
