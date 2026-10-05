//! The API routes the Plan pages read. Empty until their story adds them.

use axum::Router;

use crate::server::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
}
