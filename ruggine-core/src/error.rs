use serde::{Deserialize, Serialize};

/// Errore condiviso per HTTP e WS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    /// Codice messaggio
    pub code: String,
    
    pub message: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
