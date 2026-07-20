use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct EvaluarReq {
    pub curp: String,
    pub monto: f64,
    pub plazo_meses: i32,
}

#[derive(Deserialize)]
pub struct AutorizarReq {
    pub empresa: String,
    pub cliente_curp: String,
    pub producto: String,
    pub monto_total: f64,
    pub plazo_meses: i32,
    pub pago_mensual: f64,
}

#[derive(Serialize)]
pub struct PlanPago {
    pub empresa: String,
    pub cliente_curp: String,
    pub producto: String,
    pub monto_total: f64,
    pub plazo_meses: i32,
    pub pago_mensual: f64,
    pub estado: String,
    pub fecha: String,
}
