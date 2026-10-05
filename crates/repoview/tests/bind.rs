//! Acceptance 6: the listener is on 127.0.0.1 whatever the environment says.
//!
//! This file holds a single test so that setting process environment variables races nothing.

use std::net::{IpAddr, Ipv4Addr};

#[tokio::test]
async fn bind_is_loopback_whatever_the_environment_says() {
    for name in [
        "HOST",
        "BIND",
        "REPOVIEW_HOST",
        "REPOVIEW_BIND",
        "REPOVIEW_ADDR",
    ] {
        // SAFETY: this test binary runs exactly one test, so no other thread reads the
        // environment concurrently.
        unsafe { std::env::set_var(name, "0.0.0.0") };
    }
    let listener = repoview::server::bind(0).await.unwrap();
    let address = listener.local_addr().unwrap();
    assert_eq!(address.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
    assert_ne!(address.port(), 0);
}
