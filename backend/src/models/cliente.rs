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
    // Ola 3: identidad verificable. Clientes previos en DB no tienen estos
    // campos; los defaults los leen como None/false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correo: Option<String>,
    #[serde(default)]
    pub telefono_verificado: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alerta: Option<AlertaMorosidad>,
}

#[derive(Deserialize)]
pub struct CrearClienteReq {
    pub curp: String,
    pub nombre_completo: String,
    pub direccion: String,
    pub telefono: String,
    #[serde(default)]
    pub correo: Option<String>,
}

#[derive(Deserialize)]
pub struct ReportarReq {
    // La empresa sale del token JWT (EmpresaSession), no del body.
    pub motivo: String,
}
