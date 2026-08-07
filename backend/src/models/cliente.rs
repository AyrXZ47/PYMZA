use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct Cliente {
    pub curp: String,
    pub nombre_completo: String,
    pub score: i32,
    pub nivel_riesgo: String,
    pub historial_pagos: String,
    pub direccion: String,
    pub telefono: String,
}

#[derive(Deserialize)]
pub struct CrearClienteReq {
    pub curp: String,
    pub nombre_completo: String,
    pub direccion: String,
    pub telefono: String,
}
