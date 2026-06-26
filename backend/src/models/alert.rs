use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub client_id: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
