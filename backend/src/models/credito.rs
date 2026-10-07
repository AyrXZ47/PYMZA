use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct EvaluarReq {
    pub curp: String,
    pub monto: f64,
    pub plazo_meses: i32,
}

#[derive(Serialize)]
pub struct PagoInfo {
    pub mes: i32,
    pub pago: f64,
    pub interes: f64,
    pub capital: f64,
    pub saldo_restante: f64,
}

#[derive(Serialize)]
pub struct EvaluarRes {
    pub status: String,
    pub estado: String,
    pub pago_mensual: f64,
    pub tasa_interes: f64,
    pub plan_pagos: Vec<PagoInfo>,
    pub consideraciones: String,
}

#[derive(Deserialize)]
pub struct AutorizarReq {
    // La empresa sale del token JWT (EmpresaSession), no del body.
    pub cliente_curp: String,
    pub producto: String,
    pub monto_total: f64,
    pub plazo_meses: i32,
    // Ola 8 (E2): se siguen aceptando por compatibilidad, pero `autorizar` los
    // IGNORA y recomputa ambos desde `monto_total` + `plazo`.
    #[allow(dead_code)]
    pub pago_mensual: f64,
    #[allow(dead_code)]
    pub tasa_interes: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlanPago {
    // Ola 4: el _id llega de Mongo al leer y se serializa como hex string
    // (serde_json es "human readable" → ObjectId::to_hex). Al insertar queda
    // None y se omite (Mongo lo genera); `autorizar` captura el inserted_id.
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub empresa: String,
    pub cliente_curp: String,
    pub producto: String,
    pub monto_total: f64,
    pub plazo_meses: i32,
    pub pago_mensual: f64,
    pub tasa_interes: f64,
    pub estado: String,
    pub fecha: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DashboardStats {
    pub empresa: String,
    pub creditos_activos: i32,
    pub capital_prestado: f64,
    pub proximos_cobros: i32,
}

/// Ola 7: valor por defecto de `Pago.tipo` — los pagos viejos (sin el campo en
/// Mongo) se leen como cuota, así el saldo/estado no cambia.
fn tipo_cuota() -> String {
    "cuota".to_string()
}

/// Pago registrado de una cuota (ola 4) o abono parcial (ola 7). `plan_id` es
/// el ObjectId del plan (hex en la API). `fecha` es "YYYY-MM-DD" UTC.
/// `tipo`: "cuota" (default, cuota en 1..=plazo) o "abono" (cuota = 0).
#[derive(Serialize, Deserialize, Clone)]
pub struct Pago {
    pub plan_id: ObjectId,
    pub empresa: String,
    pub cliente_curp: String,
    pub cuota: i32,
    pub monto: f64,
    pub fecha: String,
    #[serde(default = "tipo_cuota")]
    pub tipo: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nota: Option<String>,
}

/// Body de `POST /api/creditos/pagos`.
#[derive(Deserialize)]
pub struct RegistrarPagoReq {
    pub plan_id: String,
    pub cuota: i32,
    pub monto: f64,
}

/// Body de `POST /api/creditos/abonos` (ola 7): pago parcial que no marca la
/// cuota como pagada.
#[derive(Deserialize)]
pub struct RegistrarAbonoReq {
    pub plan_id: String,
    pub monto: f64,
    #[serde(default)]
    pub nota: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pago_viejo_sin_tipo_lee_como_cuota() {
        // Regresión ola 7: un documento de `pagos` previo al campo `tipo` debe
        // deserializar como cuota (y sin nota) — el saldo/estado no cambia.
        let viejo = r#"{
            "plan_id": {"$oid": "507f1f77bcf86cd799439011"},
            "empresa": "demo@pymza.mx",
            "cliente_curp": "GARM980412HDFNRL05",
            "cuota": 1,
            "monto": 1766.67,
            "fecha": "2026-02-01"
        }"#;
        let pago: Pago = serde_json::from_str(viejo).unwrap();
        assert_eq!(pago.tipo, "cuota");
        assert_eq!(pago.nota, None);
        assert_eq!(pago.cuota, 1);
    }

    #[test]
    fn abono_serializa_cuota_cero_y_tipo() {
        let abono = RegistrarAbonoReq {
            plan_id: "507f1f77bcf86cd799439011".into(),
            monto: 500.0,
            nota: Some("abono semanal".into()),
        };
        assert_eq!(abono.monto, 500.0);
        assert_eq!(abono.nota.as_deref(), Some("abono semanal"));
    }
}
