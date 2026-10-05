//! The JSON API, one module per page family. A page story adds its routes to its own module's
//! `routes()`, with full paths under `/api/`; `server::router` merges `api::routes()`, so every
//! route here gets the `Host` check, the token check and the `/api` 404 fallback.

use axum::Router;

use crate::server::AppState;

pub mod plan;
pub mod quality;
pub mod repository;
pub mod spec;

#[cfg(test)]
mod probe;
#[cfg(test)]
mod tests;

/// Every module's routes, merged.
pub fn routes() -> Router<AppState> {
    let routes = Router::new()
        .merge(plan::routes())
        .merge(spec::routes())
        .merge(quality::routes())
        .merge(repository::routes());
    #[cfg(test)]
    let routes = routes.merge(probe::routes());
    routes
}
