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
    pub pago_mensual: f64,
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

/// Pago registrado de una cuota (ola 4). `plan_id` es el ObjectId del plan
/// (hex en la API). `fecha` es "YYYY-MM-DD" UTC.
#[derive(Serialize, Deserialize, Clone)]
pub struct Pago {
    pub plan_id: ObjectId,
    pub empresa: String,
    pub cliente_curp: String,
    pub cuota: i32,
    pub monto: f64,
    pub fecha: String,
}

/// Body de `POST /api/creditos/pagos`.
#[derive(Deserialize)]
pub struct RegistrarPagoReq {
    pub plan_id: String,
    pub cuota: i32,
    pub monto: f64,
}
