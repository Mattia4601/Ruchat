use uuid::Uuid;

/// Genera un nuovo clientMsgId unico (UUIDv4) come stringa.
pub fn new_client_msg_id() -> String {
    Uuid::new_v4().to_string()
}
