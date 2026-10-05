//! A throwaway route, compiled into test builds only, so the tests can show that a route an API
//! module adds sits behind the server's guard.

use axum::Router;
use axum::routing::get;

use crate::server::AppState;

pub const PATH: &str = "/api/probe";
pub const BODY: &str = "probe\n";

pub fn routes() -> Router<AppState> {
    Router::new().route(PATH, get(|| async { BODY }))
}
