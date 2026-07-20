use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SolicitudCredito {
    pub curp: String,
    pub producto: String,
    pub monto: f64,
    pub plazo_meses: i32,
    pub empresa_logueada: String,
}

#[derive(Serialize, Deserialize)]
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
