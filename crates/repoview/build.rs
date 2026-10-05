#[path = "build_check.rs"]
mod build_check;

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let dist = std::path::Path::new(&manifest).join("../../web/dist");
    println!(
        "cargo::rerun-if-changed={}",
        dist.join("index.html").display()
    );
    let profile = std::env::var("PROFILE").unwrap_or_default();
    if let Err(message) = build_check::check(&profile, &dist) {
        println!("cargo::error={message}");
    }
}
