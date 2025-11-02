/* This file defines how data "travel" through the web socket
    WsMessage is an enum for the envelope, this contains all the variants of ws data types which are:
    SendMessage -> message from clent 
    Message -> message from server
    Ack -> ack sent from the server in response to a request from client (for example in response to a SendMessage)
    Error -> for errors not related to a command
*/
use serde::{Deserialize, Serialize};

use crate::{error::Error, models::Message};

/// Messaggio WS con envelope { type, payload }.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsMessage {
    /// Client → Server: richiesta di inviare un messaggio.
    #[serde(rename = "sendMessage")]
    SendMessage(SendMessage),
    /// Server → Client: evento di nuovo messaggio.
    #[serde(rename = "message")]
    Message(Message),
    /// Server → Client: riscontro ad un intento (idempotente).
    #[serde(rename = "ack")]
    Ack(Ack),
    /// Server → Client: errore fuori banda.
    #[serde(rename = "error")]
    Error(Error),
}

/// Payload per l'intento di invio messaggio (C→S).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessage {
    pub client_msg_id: String,
    pub group_id: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_at: Option<String>, // RFC3339 (opzionale)
}

/// Stato dell'acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AckStatus {
    #[serde(rename = "ok")] 
    Ok,
    #[serde(rename = "error")]
    Error,
}

/// Risposta del server ad un intento (S→C), idempotente.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ack {
    /// clientMsgId del comando a cui rispondiamo.
    pub in_reply_to: String,
    pub status: AckStatus,
    /// Presente se status = ok
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    /// Presente se status = ok
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Echo utile per client (facoltativo)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Presente se status = error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
}
