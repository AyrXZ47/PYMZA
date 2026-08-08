use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct AlertaMorosidad {
    pub empresa: String,
    pub motivo: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Cliente {
    pub curp: String,
    pub nombre_completo: String,
    pub score: i32,
    pub nivel_riesgo: String,
    pub historial_pagos: String,
    pub direccion: String,
    pub telefono: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alerta: Option<AlertaMorosidad>,
}

#[derive(Deserialize)]
pub struct CrearClienteReq {
    pub curp: String,
    pub nombre_completo: String,
    pub direccion: String,
    pub telefono: String,
}

#[derive(Deserialize)]
pub struct ReportarReq {
    pub empresa: String,
    pub motivo: String,
}
