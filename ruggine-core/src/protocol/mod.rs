pub mod ws;
pub mod http;

// Re-export comodi
pub use ws::{Ack, AckStatus, SendMessage, WsMessage};
pub use http::{
    RegisterRequest, RegisterResponse, LoginRequest, LoginResponse, ListGroupsResponse,
    CreateGroupRequest, CreateGroupResponse, ListMessagesResponse,
};
