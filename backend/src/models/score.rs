use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Score {
    pub id: String,
    pub client_id: String,
    pub score: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
