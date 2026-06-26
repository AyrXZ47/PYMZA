use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Client {
    pub id: String,
    pub name: String,
    pub email: String,
}
