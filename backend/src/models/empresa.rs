use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Empresa {
    pub correo: String,
    pub password: String,
    pub nombre_empresa: String,
}
