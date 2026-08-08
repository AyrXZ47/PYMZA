use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Empresa {
    pub correo: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub nombre_empresa: String,
}
