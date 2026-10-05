//! repoview: project discovery, the snapshot, and the local server that serves it with the SPA.

pub mod assets;
pub mod browser;
pub mod project;
pub mod server;

#[cfg(test)]
#[path = "../build_check.rs"]
mod build_check;
