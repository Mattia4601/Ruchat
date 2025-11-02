use serde::{Deserialize, Serialize};

/// Utente esposto al client/server sul wire (non è un modello di DB).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub user_id: String,
    pub username: String,
    pub created_at: String, // RFC3339 UTC
}
