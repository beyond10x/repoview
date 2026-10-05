#[path = "build_check.rs"]
mod build_check;

use std::path::Path;

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let dist = Path::new(&manifest).join("../../web/dist");
    println!("cargo::rerun-if-changed={}", dist.display());
    // The embedded web app is read by a proc macro, which neither cargo's fingerprint nor a
    // compiler cache sees when web/dist appears or changes. The digest puts its content into
    // the compiler invocation, so a changed web build always recompiles the crate.
    println!(
        "cargo::rustc-env=REPOVIEW_WEB_DIST_DIGEST={}",
        build_check::digest(&dist)
    );
    let profile = std::env::var("PROFILE").unwrap_or_default();
    if let Err(message) = build_check::check(&profile, &dist) {
        println!("cargo::error={message}");
    }
}
