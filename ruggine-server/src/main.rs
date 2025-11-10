use axum::{routing::get, Router, Extension};
use std::net::SocketAddr;
use std::sync::Arc;
use anyhow::Context;

// ri-utilizziamo le funzioni e strutture definite in lib.rs
use ruggine_server::{build_sqlite_url, connect_pool, run_migrations, health_with_pool, AppState};


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Costruisci l'URL del database SQLite
    let db_url = build_sqlite_url().context("build sqlite DATABASE_URL")?;
    println!("Using DATABASE_URL = {}", db_url);
    // Connetti al database
    let pool = connect_pool(&db_url).await.context("connect to sqlite")?;
    // Esegui le migrazioni del database
    run_migrations(&pool).await.context("run migrations")?;
    // Crea lo stato dell'applicazione condiviso
    let state = Arc::new(AppState { pool });
    // Configura le rotte dell'applicazione
    let app = Router::new()
        .route("/health", get(|Extension(state): Extension<Arc<AppState>>| async move {
            health_with_pool(&state.pool).await
        }))
        .layer(Extension(state));
    // Ottieni l'indirizzo di binding dal env o usa il default
    let bind = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    // converte la stringa bind in un socketAddr -> il tipo della libreria standard che rappresenta host + porta
    let addr: SocketAddr = bind.parse().context("parse BIND_ADDR")?;
    println!("Listening on http://{}", addr);
    // Crea il listener TCP, un socket tcp e lo lega all'indirizzo addr
    /*
     *Note su comportamento: il TcpListener::bind crea il socket
     *in modalità non-bloccante (compatibile con Tokio) e ritorna immediatamente quando il bind è completato;
    */
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("bind tcp listener")?;
    // Avvia il server Axum
    /*
     *Cosa fa: avvia il server HTTP che accetta connessioni sul listener e instrada
     * le richieste usando il Router/service creato (app.into_make_service()).
     */
    axum::serve(listener, app.into_make_service())
        .await
        .context("server shutdown")?;

    Ok(())
}