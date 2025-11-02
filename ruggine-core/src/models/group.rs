use serde::{Deserialize, Serialize};

/// Gruppo (chat room) esposto sul wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub group_id: String,
    pub name: String,
    pub created_at: String, // RFC3339 UTC
}
