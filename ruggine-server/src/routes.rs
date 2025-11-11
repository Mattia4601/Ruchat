use axum::{routing::{get, post}, Router, Extension};
use std::sync::Arc;

use crate::{AppState, health_with_pool};
use crate::controllers;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(|Extension(state): Extension<Arc<AppState>>| async move {
            health_with_pool(&state.pool).await
        }))
        .route("/api/register", post(controllers::register))
        .route("/api/login", post(controllers::login))
        .layer(Extension(state))
}
