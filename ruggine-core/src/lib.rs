//! ruggine-core: tipi condivisi tra client e server (modelli, DTO HTTP, messaggi WS, errori).
//! Niente I/O o dipendenze non compatibili con WASM.

pub mod models;
pub mod protocol;
pub mod error;
pub mod utils;

// Re-export utili per ridurre i percorsi nei crate client/server
pub use error::Error;
pub use models::{group::Group, message::Message, user::User};
pub use protocol::ws::{Ack, AckStatus, SendMessage, WsMessage};
pub use protocol::http::{
    CreateGroupRequest, CreateGroupResponse, ListGroupsResponse, ListMessagesResponse,
    LoginRequest, LoginResponse, RegisterRequest, RegisterResponse,
};
pub use utils::{new_client_msg_id, now_timestamp};
