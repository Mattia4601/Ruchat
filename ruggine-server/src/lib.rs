use anyhow::Context;
use axum::http::StatusCode;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use dashmap::DashMap;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    /// Map of user_id -> sender used to forward messages to connected websocket sessions.
    pub ws_users: DashMap<String, UnboundedSender<String>>,
}

// Dato un percorso di file, restituisce un URL SQLite valido. Crea le directory genitrici se non esistono.
pub fn sqlite_url_for_path(p: &Path) -> anyhow::Result<String> {
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()?.join(p)
    };
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create parent dirs for {:?}", parent))?;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&abs)
        .with_context(|| format!("create/open sqlite file {:?}", abs))?;
    let s = abs.to_string_lossy().replace('\\', "/");
    Ok(format!("sqlite:///{}", s))
}

/// Crea un DB URL SQLite leggendo la variabile d'ambiente DATABASE_URL.
/// Se non è impostata, usa "ruggine.db" nella directory corrente.
pub fn build_sqlite_url() -> anyhow::Result<String> {
    let raw = std::env::var("DATABASE_URL").unwrap_or_else(|_| "ruggine.db".to_string());
    if raw == "sqlite::memory:" {
        return Ok(raw);
    }
    // Rimuovi il prefisso "sqlite://" se presente, per ottenere il percorso del file. 
    let path_part = if raw.starts_with("sqlite://") {
        raw.trim_start_matches("sqlite:///")
            .trim_start_matches("sqlite://")
            .to_string()
    } else {
        raw
    };
    sqlite_url_for_path(&PathBuf::from(path_part))
}

// Connect to the database and return a connection pool.
pub async fn connect_pool(db_url: &str) -> anyhow::Result<SqlitePool> {
    let pool = SqlitePool::connect(db_url)
        .await
        .with_context(|| format!("connect to sqlite via {}", db_url))?;
    Ok(pool)
}

// Esegue le migrazioni del database. Crea le tabelle se non esistono.
pub async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    // Enable foreign keys (SQLite)
    sqlx::query("PRAGMA foreign_keys = ON;")
        .execute(pool)
        .await
        .context("enable foreign_keys")?;

    let stmts = [
        r#"
        CREATE TABLE IF NOT EXISTS users (
            user_id      TEXT PRIMARY KEY,
            username     TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            token        TEXT,
            created_at   TEXT NOT NULL
        );"#,
        r#"
        CREATE TABLE IF NOT EXISTS groups (
            group_id   TEXT PRIMARY KEY,
            name       TEXT NOT NULL,
            created_at TEXT NOT NULL
        );"#,
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            message_id TEXT PRIMARY KEY,
            group_id   TEXT NOT NULL,
            sender_id  TEXT NOT NULL,
            content    TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY(group_id) REFERENCES groups(group_id),
            FOREIGN KEY(sender_id) REFERENCES users(user_id)
        );"#,
        r#"
        CREATE TABLE IF NOT EXISTS memberships (
            membership_id TEXT PRIMARY KEY,
            group_id      TEXT NOT NULL,
            user_id       TEXT NOT NULL,
            joined_at     TEXT NOT NULL,
            FOREIGN KEY(group_id) REFERENCES groups(group_id),
            FOREIGN KEY(user_id)  REFERENCES users(user_id)
        );"#,
        r#"
        CREATE TABLE IF NOT EXISTS invites (
            invite_id TEXT PRIMARY KEY,
            group_id  TEXT NOT NULL,
            invited   TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY(group_id) REFERENCES groups(group_id)
        );"#,
    ];
    // applica ogni statement di migrazione
    for s in &stmts {
        sqlx::query(s)
            .execute(pool)
            .await
            .with_context(|| format!("apply migration: {}", &s[..s.len().min(40)].replace('\n', " ")))?;
    }
    Ok(())
}

pub mod controllers;
pub mod routes;

/// Controlla lo stato di salute del database tentando di acquisire una connessione dal pool.
pub async fn health_with_pool(pool: &SqlitePool) -> StatusCode {
    match pool.acquire().await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}