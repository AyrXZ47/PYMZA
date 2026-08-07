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
    pub empresa: String,
    pub cliente_curp: String,
    pub producto: String,
    pub monto_total: f64,
    pub plazo_meses: i32,
    pub pago_mensual: f64,
    pub tasa_interes: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlanPago {
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
