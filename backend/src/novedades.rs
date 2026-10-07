//! Novedades ("what's new", ola 8): versión del backend + changelog estático.
//! El frontend compara `VERSION` contra su `APP_VERSION` compilada para avisar
//! "hay actualización, recarga" y para el badge de la campanita. Solo texto.

use axum::Json;

/// Versión actual de la app. Se bumpea a mano por release (techo: inyectarla
/// por env en build desde los Dockerfiles; upgrade path documentado en .env).
pub const VERSION: &str = "0.8.0";

/// Changelog visible: (fecha, titulo, detalle). Hitos de las olas.
const NOVEDADES: &[(&str, &str, &str)] = &[
    (
        "2026-10-06",
        "Abonos a prueba de concurrencia",
        "Los abonos y pagos concurrentes ya no pueden rebasar la deuda: el cobrado reserva de forma atómica y el saldo queda siempre consistente.",
    ),
    (
        "2026-10-06",
        "Tablero por periodo",
        "Nuevos indicadores de capital colocado, cobrado del periodo, por cobrar neto, cartera vencida y morosidad por dinero; con filtros de semana, mes, bimestre, trimestre y semestre.",
    ),
    (
        "2026-10-06",
        "Autorizar con montos confiables",
        "El plan de pagos se recalcula desde el monto total y el plazo en el servidor; la tasa y el pago mensual ya no dependen de lo que llegue en la solicitud.",
    ),
    (
        "2026-10-05",
        "Cobranza real",
        "Abonos parciales sin marcar la cuota, saldo y estado por dinero, contrato con el detalle de pagos y cartera con buscador y filtros.",
    ),
    (
        "2026-09-30",
        "Verificación de identidad",
        "KYC de INE y score alternativo por recibos de servicios para respaldar la decisión de crédito.",
    ),
];

/// GET /api/novedades (pública, sin JWT): versión + changelog. No expone
/// secretos ni datos de ninguna empresa.
pub async fn obtener_novedades() -> Json<serde_json::Value> {
    let novedades: Vec<serde_json::Value> = NOVEDADES
        .iter()
        .map(|(fecha, titulo, detalle)| {
            serde_json::json!({ "fecha": fecha, "titulo": titulo, "detalle": detalle })
        })
        .collect();
    Json(serde_json::json!({
        "status": "success",
        "version": VERSION,
        "novedades": novedades,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn novedades_shape_del_contrato() {
        let Json(v) = obtener_novedades().await;
        assert_eq!(v["status"], "success");
        assert_eq!(v["version"], VERSION);
        let lista = v["novedades"].as_array().expect("novedades es array");
        assert!(!lista.is_empty());
        for n in lista {
            assert!(n["fecha"].is_string());
            assert!(n["titulo"].is_string());
            assert!(n["detalle"].is_string());
        }
    }
}
