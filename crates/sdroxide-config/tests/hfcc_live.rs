//! Live check on HFCC: the package downloads, parses, and is **refused**.
//!
//! `#[ignore]`d because it needs the network. It is here to pin a decision rather
//! than a feature: HFCC's operational schedule carries no broadcaster names, so
//! shipping it would mean a schedule full of codes. If a future build teaches it
//! the by-broadcaster names, **this test failing is the signal to change**, not a
//! regression to fix.
//!
//! ```sh
//! SDROXIDE_CONFIG_DIR=/tmp/hfcc-live \
//!   cargo test -p sdroxide-config --test hfcc_live -- --ignored --nocapture
//! ```
#[test]
#[ignore = "needs the network"]
fn hfcc_refuses_rather_than_shipping_a_schedule_of_codes() {
    let err = sdroxide_config::fetch_broadcast_schedule_from(sdroxide_types::ScheduleSource::Hfcc)
        .expect_err("HFCC has no names and must not be loaded");
    let msg = err.to_string();
    assert!(msg.contains("names"), "it must say why: {msg}");
    assert!(msg.contains("by-broadcaster"), "and where they are: {msg}");
}
