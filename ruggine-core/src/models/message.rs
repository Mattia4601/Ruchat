use serde::{Deserialize, Serialize};

/// Messaggio persistito dal server e notificato via WS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub message_id: String,
    pub group_id: String,
    pub sender_id: String,
    pub content: String,
    pub created_at: String, // RFC3339 UTC
}
