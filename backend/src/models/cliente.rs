use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Cliente {
    pub curp: String,
    pub nombre_completo: String,
    pub score: i32,
    pub nivel_riesgo: String,
    pub historial_pagos: String,
}
