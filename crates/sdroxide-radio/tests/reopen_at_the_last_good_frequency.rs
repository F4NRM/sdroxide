//! The link button opens the radio where it last tuned (#72).
//!
//! The report that produced this: a start frequency the tuner cannot reach —
//! an I.F. offset saved as a frequency, so the dongle was asked for 6 GHz —
//! meant the dongle never opened and the operator had to correct the number by
//! hand **every time**, including after pressing the link button. Two things
//! had to be true for that, and both are pinned here:
//!
//! - the program's own "last frequency that tuned" record held the frequency it
//!   had merely been *asked* for, so the fallback was falling back to the same
//!   wrong number; and
//! - the button and the background reconnect opened at the dial, which is that
//!   same wrong number — the first fix lived in the start-up path alone.
//!
//! The second is the one this file exists for: `EngineSwap::ReopenSource` is
//! the route the link button takes, and it must ask for a frequency this radio
//! has tuned.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sdroxide_radio::{
    Complex32, EngineConfig, EngineSwap, IqSource, ReopenFn, Result, start_engine,
};
use sdroxide_types::DeviceCaps;

/// The start frequency that could not be tuned: an I.F. offset saved as one.
const IMPOSSIBLE: f64 = 6_000_000_000.0;
/// Where the radio was actually left — a QO-100 satellite downlink.
const QO100: f64 = 144_150_000.0;
const RATE: f64 = 48_000.0;

/// Every centre the factory was asked to open, in order.
type Asked = Arc<Mutex<Vec<f64>>>;

/// A source that comes up and tunes, at whatever centre it is opened on.
struct Real {
    center: f64,
}

impl IqSource for Real {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        self.center
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let n = buf.len().min(1024);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "the radio".into()
    }
}

/// The stand-in the program substitutes when the radio would not open: it
/// reports the centre that was **asked for**, and asks to be reopened.
struct StandIn {
    center: f64,
}

impl IqSource for StandIn {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        self.center
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let n = buf.len().min(1024);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "no radio".into()
    }
    fn open_status(&self) -> Option<String> {
        Some("the tuner refused to open".into())
    }
    fn needs_reopen(&self) -> bool {
        true
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "test".into(),
        label: "the radio".into(),
        rx_channels: 1,
        tx_channels: 0,
        audio_mode: false,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 1_800_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// `SDROXIDE_CONFIG_DIR` is process state, and Rust runs the tests in one
/// binary on several threads — so two tests each setting it would race, and
/// the loser would read the winner's session. Held for the length of a test.
static CONFIG_DIR: Mutex<()> = Mutex::new(());

/// An isolated configuration directory, so the session this writes is this
/// test's and not the operator's. `SDROXIDE_CONFIG_DIR` is what every `Store`
/// constructor reads, which is how the other persistence tests do it.
///
/// Callers hold [`CONFIG_DIR`].
fn temp_root(tag: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("sdroxide-reopen-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };
    root
}

/// A factory that behaves like an RTL-SDR: it refuses anything past its tuner
/// and opens anything below, recording what it was asked for each time.
fn tuner(asked: Asked) -> ReopenFn {
    Box::new(move |center: f64| {
        asked.lock().unwrap().push(center);
        if center > 1_800_000_000.0 {
            return Err(format!(
                "{center} is above this RTL-SDR's tuner range, which ends at 1766 MHz"
            ));
        }
        Ok((Box::new(Real { center }) as Box<dyn IqSource>, caps()))
    })
}

/// A session that remembers where the radio was left, which is the only place
/// a fallback can honestly come from.
fn session_recalling_qo100(store: &sdroxide_config::Store) {
    let mut s = store.load_session();
    s.last_good_vfo_hz = QO100;
    store.save_session(&s).unwrap();
}

#[test]
fn the_link_button_opens_at_the_last_frequency_that_tuned() {
    let _dir = CONFIG_DIR.lock().unwrap_or_else(|e| e.into_inner());
    let root = temp_root("link");
    let store = sdroxide_config::Store::radio(0);
    session_recalling_qo100(&store);

    let asked: Asked = Arc::new(Mutex::new(Vec::new()));
    let cfg = EngineConfig {
        tx_ham_only: false,
        remember_session: true,
        store: store.clone(),
        reopen: Some(tuner(Arc::clone(&asked))),
        ..Default::default()
    };
    // Started on the stand-in at the impossible frequency — a start frequency
    // the tuner cannot reach, which is what the report is.
    let mut h = start_engine(Box::new(StandIn { center: IMPOSSIBLE }), caps(), cfg);

    // The link button. Its route is a swap, not a command, and going through
    // the real one is the point: this is what the button does.
    h.swap_tx.send(EngineSwap::ReopenSource).unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if asked.lock().unwrap().iter().any(|&c| (c - QO100).abs() < 1.0) {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let asks = asked.lock().unwrap().clone();
    assert!(
        asks.iter().any(|&c| (c - QO100).abs() < 1.0),
        "the link button never asked for the frequency the radio had tuned; it asked {asks:?} \
         — {IMPOSSIBLE} Hz is past an RTL-SDR's tuner, so none of those could open"
    );
    assert!(
        !asks.iter().any(|&c| (c - IMPOSSIBLE).abs() < 1.0),
        "the button repeated the impossible frequency rather than the one that worked: {asks:?}"
    );

    // Stop it, which is what writes the session.
    let thread = h.thread.take();
    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_start_that_never_opened_does_not_record_the_frequency_it_was_asked_for() {
    let _dir = CONFIG_DIR.lock().unwrap_or_else(|e| e.into_inner());
    let root = temp_root("seed");
    let store = sdroxide_config::Store::radio(0);
    session_recalling_qo100(&store);

    // No factory at all: nothing can open, so this run never tunes.
    let cfg = EngineConfig {
        tx_ham_only: false,
        remember_session: true,
        store: store.clone(),
        ..Default::default()
    };
    let mut h = start_engine(Box::new(StandIn { center: IMPOSSIBLE }), caps(), cfg);

    let thread = h.thread.take();
    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }

    let recorded = store.load_session().last_good_vfo_hz;
    assert_ne!(
        recorded, IMPOSSIBLE,
        "a front end that never opened recorded {IMPOSSIBLE} Hz as a frequency that tuned — \
         that is what made the fallback fall back to the same wrong number for ever"
    );
    assert_eq!(
        recorded, QO100,
        "nothing tuned on this run, so the record should still be the last frequency that did"
    );

    let _ = std::fs::remove_dir_all(&root);
}