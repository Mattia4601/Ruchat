use axum::{extract::Extension, http::StatusCode, Json, extract::{Query, WebSocketUpgrade}, response::IntoResponse};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{StreamExt, SinkExt};
use ruggine_core::{protocol::http::{RegisterRequest, RegisterResponse, LoginRequest, LoginResponse}, protocol::ws::{WsMessage, Authenticate}, models::User, utils::now_timestamp};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;

/// Handler per POST /api/register
pub async fn register(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), (StatusCode, String)> {
    // controllo se lo username esiste già:
    // query_scalar Makes a SQL query that is mapped to a single concrete type
    let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE username = ?")
        .bind(&req.username)    // bind imposta il parametro della query
        .fetch_one(&state.pool)     // fetch_one esegue la query usando il pool asincrono e si aspetta che la query ritorni esattamente una riga
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {}", e)))?; // se la query fallisce mappo l'errore in (StatusCode, string)
    if existing > 0 {
        /* Se il risultato è maggiore di 0 allora lo username esiste già */
        return Err((StatusCode::CONFLICT, "username already exists".to_string()));
    }

    // genera id utente e token
    let user_id = Uuid::new_v4().to_string();
    let token = Uuid::new_v4().to_string();
    // hash semplice della password
    let mut hasher = Sha256::new();
    hasher.update(req.password.as_bytes());
    let password_hash = format!("{:x}", hasher.finalize());
    let created_at = now_timestamp();

    // inserisci
    sqlx::query("INSERT INTO users (user_id, username, password_hash, token, created_at) VALUES (?, ?, ?, ?, ?)")
        .bind(&user_id)
        .bind(&req.username)
        .bind(&password_hash)
        .bind(&token)
        .bind(&created_at)
        /* execute esegue la query, non ritorna righe ma il risultato dell'esecuzione della query */
        .execute(&state.pool)
        .await
        /* se l'INSERT fallisce map_err converte l'errore in 500 ed esce dall'handler con l'operatore ? */
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db insert error: {}", e)))?;

    /* creazione della risposta */
    let user = User { user_id: user_id.clone(), username: req.username.clone(), created_at };
    let resp = RegisterResponse { user, token };
    Ok((StatusCode::CREATED, Json(resp)))
}

/// Handler per /ws
pub async fn ws_handler(
    Extension(state): Extension<Arc<AppState>>,
    ws: WebSocketUpgrade,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let token = params.get("token").cloned();
    ws.on_upgrade(move |socket| handle_socket(socket, state, token))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>, token_q: Option<String>) {
    // Try authenticate via query param first
    let mut user_opt: Option<User> = None;

    if let Some(token) = token_q {
        match sqlx::query("SELECT user_id, username, created_at FROM users WHERE token = ?")
            .bind(&token)
            .fetch_optional(&state.pool)
            .await
        {
            Ok(Some(row)) => {
                let user_id: String = row.try_get("user_id").unwrap_or_default();
                let username: String = row.try_get("username").unwrap_or_default();
                let created_at: String = row.try_get("created_at").unwrap_or_default();
                user_opt = Some(User { user_id, username, created_at });
            }
            Ok(None) => {
                // invalid token
            }
            Err(e) => {
                let _ = socket.send(Message::Text(serde_json::to_string(&WsMessage::Error(ruggine_core::error::Error { code: "internal_error".to_string(), message: format!("db error: {}", e), details: None })).unwrap())).await;
                return;
            }
        }
    }

    // If not authenticated via query, wait for first Authenticate message
    if user_opt.is_none() {
        // read one message
        if let Some(Ok(msg)) = socket.next().await {
            if let Message::Text(txt) = msg {
                match serde_json::from_str::<WsMessage>(&txt) {
                    Ok(WsMessage::Authenticate(auth)) => {
                        // lookup token
                        match sqlx::query("SELECT user_id, username, created_at FROM users WHERE token = ?")
                            .bind(&auth.token)
                            .fetch_optional(&state.pool)
                            .await
                        {
                            Ok(Some(row)) => {
                                let user_id: String = row.try_get("user_id").unwrap_or_default();
                                let username: String = row.try_get("username").unwrap_or_default();
                                let created_at: String = row.try_get("created_at").unwrap_or_default();
                                user_opt = Some(User { user_id, username, created_at });
                            }
                            Ok(None) => {
                                // invalid token
                            }
                            Err(e) => {
                                let _ = socket.send(Message::Text(serde_json::to_string(&WsMessage::Error(ruggine_core::error::Error { code: "internal_error".to_string(), message: format!("db error: {}", e), details: None })).unwrap())).await;
                                return;
                            }
                        }
                    }
                    _ => {
                        // unexpected message
                        let _ = socket.send(Message::Text(serde_json::to_string(&WsMessage::Error(ruggine_core::error::Error { code: "auth_required".to_string(), message: "expected Authenticate message".to_string(), details: None })).unwrap())).await;
                        return;
                    }
                }
            } else {
                // non-text first message
                let _ = socket.send(Message::Text(serde_json::to_string(&WsMessage::Error(ruggine_core::error::Error { code: "auth_required".to_string(), message: "expected text Authenticate message".to_string(), details: None })).unwrap())).await;
                return;
            }
        } else {
            // connection closed or error
            return;
        }
    }

    // if still none -> auth failed
    let user = match user_opt {
        Some(u) => u,
        None => {
            let _ = socket.send(Message::Text(serde_json::to_string(&WsMessage::Error(ruggine_core::error::Error { code: "unauthorized".to_string(), message: "invalid token".to_string(), details: None })).unwrap())).await;
            return;
        }
    };

    // Register sender per questa sessione WebSocket.
    // `tx` è un `UnboundedSender<String>` che altre parti del server possono clonare e usare
    // per inviare messaggi a questo client (server -> client). 
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    state.ws_users.insert(user.user_id.clone(), tx.clone());

    // Send AuthOk
    let _ = socket.send(Message::Text(serde_json::to_string(&WsMessage::AuthOk(user.clone())).unwrap())).await;

    // Split socket into sink/stream
    /* socket.split() divide l'oggetto WebSocket in due metà indipendenti:
        sender (un Sink) usato per inviare messaggi verso il client,
        receiver (uno Stream) usato per ricevere messaggi dal client. */
    let (mut sender, mut receiver) = socket.split();

    // Task: forward messages from rx -> websocket
    let forward_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Task: read incoming messages and (for now) ignore or log SendMessage
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(t) => {
                if let Ok(parsed) = serde_json::from_str::<WsMessage>(&t) {
                    match parsed {
                        WsMessage::SendMessage(sm) => {
                            tracing::info!("received sendMessage from {}: group {}", user.user_id, sm.group_id);
                            // For now we don't implement broadcasting; just ack could be added here.
                        }
                        _ => {}
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    // cleanup
    state.ws_users.remove(&user.user_id);
    // ensure forward task ends
    let _ = forward_task.await;
}

/// Handler per POST /api/login
pub async fn login(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    // cerca utente
    let row = sqlx::query("SELECT user_id, password_hash, created_at FROM users WHERE username = ?")
        .bind(&req.username) // passa parametro alla query
        .fetch_optional(&state.pool)    // esegue la query ritornando un option<Row>
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {}", e)))?; // se fallisce mappa l'errore in 500 internal server error
    let row = match row {
        Some(r) => r,
        /* nessun utente trovato con quello username */
        None => return Err((StatusCode::NOT_FOUND, "user not found".to_string())),
    };
    // cerco di ottenere i vari parametri dall'utente restituito perché row è di tipo Some(Row)
    let user_id: String = row.try_get("user_id").map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db get error: {}", e)))?;
    let stored_hash: String = row.try_get("password_hash").map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db get error: {}", e)))?;
    let created_at: String = row.try_get("created_at").map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db get error: {}", e)))?;

    // Calcolo hash sulla password fornita e confronto dell'hash preso dal db
    let mut hasher = Sha256::new();
    hasher.update(req.password.as_bytes());
    let password_hash = format!("{:x}", hasher.finalize());
    if password_hash != stored_hash {
        /* se non coincidono ritorno UNAUTHORIZED */
        return Err((StatusCode::UNAUTHORIZED, "invalid credentials".to_string()));
    }

    // genera token nuovo e aggiorna
    let token = Uuid::new_v4().to_string();
    sqlx::query("UPDATE users SET token = ? WHERE user_id = ?")
        .bind(&token)
        .bind(&user_id)
        /* execute esegue l'UPDATE che non ritorna righe; eventuali errori DB sono mappati in 500. */
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db update error: {}", e)))?;

    let user = User { user_id, username: req.username.clone(), created_at };
    let resp = LoginResponse { token, user };
    Ok(Json(resp))
}
