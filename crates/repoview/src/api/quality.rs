//! The API routes the Quality page reads. Empty until its story adds them.

use axum::Router;

use crate::server::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
}
