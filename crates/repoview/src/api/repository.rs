//! The API routes the Repository page reads. Empty until its story adds them.

use axum::Router;

use crate::server::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
}
