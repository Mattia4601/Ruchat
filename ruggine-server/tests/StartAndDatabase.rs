use anyhow::Result;
use tempfile::TempDir;
use std::fs;
use std::path::PathBuf;
use ruggine_server::{sqlite_url_for_path, connect_pool, run_migrations, health_with_pool};

// Funzione di utilità per costruire l'URL SQLite da un percorso di file
fn sqlite_url_for(p: &PathBuf) -> String {
    sqlite_url_for_path(p.as_path()).expect("build sqlite url")
}

// Test che verifica che le migrazioni creino le tabelle necessarie
#[tokio::test]
async fn run_migrations_creates_tables() -> Result<()> {
    let td = TempDir::new()?;
    let db_path = td.path().join("ruggine.db");

    // assicurati che la directory genitrice esista e crea il file 
    if let Some(parent) = db_path.parent() { fs::create_dir_all(parent)?; }
    fs::File::create(&db_path)?;

    let url = sqlite_url_for(&db_path);
    let pool = connect_pool(&url).await?;
    run_migrations(&pool).await?;

    let names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' AND name IN ('users','groups','messages','memberships','invites')"
    ).fetch_all(&pool).await?;

    for expected in ["users", "groups", "messages", "memberships", "invites"] {
        assert!(names.contains(&expected.to_string()), "missing table {}", expected);
    }
    Ok(())
}

// Test che verifica che l'handler di health funzioni dopo le migrazioni
#[tokio::test]
async fn health_handler_works_after_migrations() -> Result<()> {
    let td = TempDir::new()?;
    let db_path = td.path().join("ruggine.db");
    if let Some(parent) = db_path.parent() { fs::create_dir_all(parent)?; }
    fs::File::create(&db_path)?;

    let url = sqlite_url_for(&db_path);
    let pool = connect_pool(&url).await?;
    run_migrations(&pool).await?;

    let status = health_with_pool(&pool).await;
    assert!(status.is_success(), "health should return 200 OK");
    Ok(())
}

// Test che verifica che la creazione del file DB e delle directory genitrici sia idempotente
#[tokio::test]
async fn creating_db_file_and_parent_dirs_is_idempotent() -> Result<()> {
    let td = TempDir::new()?;
    let nested = td.path().join("a").join("b").join("ruggine.db");
    let parent = nested.parent().unwrap().to_path_buf();
    assert!(!parent.exists());

    // usa la funzione di libreria che creerà le directory genitrici e il file
    let url = sqlite_url_for_path(nested.as_path())?;
    let pool = connect_pool(&url).await?;
    run_migrations(&pool).await?;

    assert!(parent.exists(), "parent dir should have been created");
    assert!(nested.exists(), "db file should have been created");

    // controllo di salute per assicurarsi che una tabella esista
    let rows: Vec<String> = sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' AND name='users'")
        .fetch_all(&pool).await?;
    assert!(!rows.is_empty());
    Ok(())
}