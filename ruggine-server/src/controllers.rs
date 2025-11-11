use axum::{extract::Extension, http::StatusCode, Json};
use ruggine_core::{protocol::http::{RegisterRequest, RegisterResponse, LoginRequest, LoginResponse}, models::User, utils::now_timestamp};
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
