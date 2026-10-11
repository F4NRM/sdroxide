//! Auto mode — the policy for the unattended 11 m sequencer.
//!
//! This is pure: given the decodes just heard and the operator's log, it
//! decides whether auto mode may run and, when it may, which station to answer
//! or that nobody is worth answering and a CQ should go out instead. The UI
//! owns the log and the input clock, so the decision lives here beside them
//! while the actual sequencing stays in the engine's QSO machine.
//!
//! Auto mode is deliberately narrow. It is for the 11 m citizens' band only —
//! the fork's WSJT-CB operation, where an unattended run is the point — and it
//! only ever answers a station calling CQ whose callsign is *new*: not one
//! already in the log. Calling a station mid-exchange, or working a dupe, is
//! left to the operator.

use std::collections::HashSet;

use crate::awards::LogIndex;
use crate::digi::{Decode, cq_is_for_us};
use crate::{Band, Mode};

/// How long the app may go without a single input event before auto mode
/// disarms, when the operator has not chosen a figure. The backstop for an
/// unattended transmitter: a window nobody has touched for this long is a
/// window nobody is watching.
pub const AUTO_IDLE_STOP_S: f64 = 20.0 * 60.0;

/// The longest the inactivity stop may be set to, in minutes.
///
/// Deliberately a ceiling rather than a free field: auto mode is for a bathroom
/// break, not for leaving a station to work a contest unattended. The operator
/// chooses within it, never past it.
pub const AUTO_IDLE_STOP_MAX_MIN: u32 = 45;

/// The stored value that means "never stop for inactivity" — the **DISABLE**
/// entry. Only honoured on a radio whose backend
/// [`has_long_idle_stop`]; anywhere else it clamps to the ceiling like any
/// other figure past it.
pub const AUTO_IDLE_STOP_NEVER: u32 = u32::MAX;

/// The choices offered on a radio with the long inactivity stop, in the order
/// the list shows them: the default first, then the short ones, the long ones
/// and **DISABLE** last.
pub const AUTO_IDLE_STOP_LONG_CHOICES: [u32; 8] =
    [20, 30, 45, 60, 120, 240, 360, AUTO_IDLE_STOP_NEVER];

/// Whether this radio may set the inactivity stop past
/// [`AUTO_IDLE_STOP_MAX_MIN`], up to six hours, or switch it off.
///
/// Only the PlutoSDR for now: it is the radio this was asked for and tested
/// against, and every other backend keeps the 45-minute ceiling until there is
/// a reason to widen it.
pub fn has_long_idle_stop(backend: crate::Backend) -> bool {
    backend == crate::Backend::Pluto
}

/// The inactivity stop in seconds, from the operator's choice in minutes, or
/// `None` when it is switched off.
///
/// Zero is the default (twenty minutes), which a config written before this
/// existed loads as. With `long` (see [`has_long_idle_stop`]) the figure is
/// taken as it is and [`AUTO_IDLE_STOP_NEVER`] means no stop; without it the
/// figure is clamped to [`AUTO_IDLE_STOP_MAX_MIN`], `NEVER` included, so a
/// setting carried over from a Pluto never switches the stop off elsewhere.
pub fn auto_idle_stop_s(minutes: u32, long: bool) -> Option<f64> {
    let m = if minutes == 0 { (AUTO_IDLE_STOP_S / 60.0) as u32 } else { minutes };
    if long {
        if m == AUTO_IDLE_STOP_NEVER {
            return None;
        }
        return Some(f64::from(m) * 60.0);
    }
    Some(f64::from(m.min(AUTO_IDLE_STOP_MAX_MIN)) * 60.0)
}

/// How long after issuing a command the tick waits before issuing another.
///
/// Only to cover the round trip to the engine: the command is queued and the
/// status that reflects it arrives a frame or two later. Without it, the idle
/// status still showing would each frame queue another CQ.
pub const AUTO_COOLDOWN_S: f64 = 3.0;

/// The station auto mode has chosen to answer.
#[derive(Debug, Clone, PartialEq)]
pub struct AutoTarget {
    pub call: String,
    /// Their grid, when the CQ carried one. 11 m CQs usually do not.
    pub grid: Option<String>,
    /// Their signal at us — the report we will send.
    pub snr_db: i16,
    /// The tone offset their CQ was heard at, to answer on.
    pub audio_hz: f32,
}

/// Whether auto mode may run: a slotted FT mode in a band this station may
/// transmit on, with a non-zero transmit watchdog to bound an unattended run.
///
/// `tx_permitted` is the engine's own answer for the current dial — the same
/// question its key-down gate asks — so auto mode never arms somewhere it could
/// not actually key: outside an amateur allocation with `tx_ham_only` set, on a
/// broadcast band, or on a receive-only radio. It is passed in rather than
/// worked out here because the licence gate and the band plan live in the
/// engine, not in this crate.
///
/// The watchdog cannot be zero: it is the only thing that stops a station
/// calling CQ into an empty band forever, and auto mode is exactly the case
/// where nobody is watching to notice.
pub fn auto_ready(mode: Mode, watchdog_min: u32, tx_permitted: bool) -> bool {
    matches!(mode, Mode::Ft8 | Mode::Ft4 | Mode::Ft2) && tx_permitted && watchdog_min > 0
}

/// Why auto mode may not run, as a sentence for the toggle's tooltip, or
/// `None` when it may. The same tests as [`auto_ready`], in the order the
/// operator can do something about them.
pub fn auto_block_reason(
    mode: Mode,
    watchdog_min: u32,
    tx_permitted: bool,
) -> Option<&'static str> {
    if !matches!(mode, Mode::Ft8 | Mode::Ft4 | Mode::Ft2) {
        return Some("Auto mode runs FT8, FT4 and FT2 only.");
    }
    if !tx_permitted {
        return Some(
            "This radio may not transmit on the current band — tune to an amateur band \
             (or enable 11 m transmit) to arm it.",
        );
    }
    if watchdog_min == 0 {
        return Some(
            "Set a transmit watchdog first (the digi setup): auto mode needs it to pace itself.",
        );
    }
    None
}

/// Pick the CQ auto mode should answer, or `None` when calling CQ is the move.
///
/// A candidate is a CQ that passes [`cq_is_for_us`], names a callsign, is not
/// already in the log, and has not already been tried this session. The
/// strongest is chosen — the one most likely to complete a contact — with the
/// newest slot breaking ties.
///
/// `skip` is asked about every row before anything else, so a caller can rule
/// a station out here instead of handing over a filtered copy. The decode list
/// is borrowed for the whole call and holds up to 200 rows, and this runs every
/// frame: building a copy of it to drop a handful of rows would cost far more
/// than the handful of comparisons it saves.
pub fn pick_cq(
    decodes: &[Decode],
    log: &LogIndex,
    my_call: &str,
    my_grid: &str,
    tried: &HashSet<String>,
    skip: impl Fn(&Decode) -> bool,
) -> Option<AutoTarget> {
    let mut best: Option<&Decode> = None;
    for d in decodes {
        if skip(d) {
            continue;
        }
        let Some(from) = d.from.as_deref().filter(|c| !c.is_empty()) else { continue };
        if !d.is_cq || !cq_is_for_us(d, my_call, my_grid) {
            continue;
        }
        let key = from.to_ascii_uppercase();
        if tried.contains(&key) {
            continue;
        }
        // The empty band: 11 m has no ADIF enumeration and no per-band log
        // judgement, so this is the all-time "never worked this call" test.
        if !log.novelty(&key, d.grid.as_deref(), "").new_call {
            continue;
        }
        match best {
            Some(b) if (d.snr_db, d.slot_utc) <= (b.snr_db, b.slot_utc) => {}
            _ => best = Some(d),
        }
    }
    best.map(|d| AutoTarget {
        call: d.from.clone().unwrap_or_default(),
        grid: d.grid.clone(),
        snr_db: d.snr_db,
        audio_hz: d.audio_hz,
    })
}

/// Whether the licence gate lets this band be keyed, mirroring the engine's
/// own key-down test: every amateur allocation always, and 11 m when the
/// operator has opened it. The broadcast services and general coverage never.
///
/// The engine is the authority and checks more than this (the device's range,
/// the offset against the band plan); this is only the band-level question auto
/// mode needs to decide whether arming is even meaningful.
pub fn band_may_transmit(band: Band, ham_only: bool, cb_tx_allowed: bool) -> bool {
    !ham_only || band.is_amateur() || (band == Band::M11 && cb_tx_allowed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QsoRecord;

    fn cq(from: &str, snr: i16, slot: i64) -> Decode {
        Decode {
            slot_utc: slot,
            snr_db: snr,
            dt: 0.0,
            audio_hz: 1000.0,
            message: format!("CQ {from}"),
            to: None,
            from: Some(from.to_string()),
            grid: None,
            is_cq: true,
            cq_to: None,
            free_text: false,
            rr73_to: None,
        }
    }

    fn log_with(calls: &[&str]) -> LogIndex {
        let log: Vec<QsoRecord> = calls
            .iter()
            .map(|c| QsoRecord { call: (*c).to_string(), mode: "FT8".into(), ..Default::default() })
            .collect();
        LogIndex::build(&log)
    }

    #[test]
    fn picks_the_strongest_new_cq() {
        let d = [cq("19LR121", -5, 1), cq("14AT276", 12, 2), cq("30AO010", 3, 3)];
        let got =
            pick_cq(&d, &log_with(&[]), "26AT715", "", &HashSet::new(), |_| false).expect("a CQ");
        assert_eq!(got.call, "14AT276");
        assert_eq!(got.snr_db, 12);
    }

    #[test]
    fn never_answers_a_call_already_in_the_log() {
        let d = [cq("19LR121", 20, 1)];
        assert!(
            pick_cq(&d, &log_with(&["19LR121"]), "26AT715", "", &HashSet::new(), |_| false)
                .is_none()
        );
    }

    #[test]
    fn skips_a_station_already_tried_this_session() {
        let d = [cq("19LR121", 20, 1)];
        let tried: HashSet<String> = ["19LR121".to_string()].into_iter().collect();
        assert!(pick_cq(&d, &log_with(&[]), "26AT715", "", &tried, |_| false).is_none());
    }

    #[test]
    fn a_skipped_station_is_never_chosen_even_when_it_is_the_strongest() {
        // The session ignore list arrives here as a predicate rather than as a
        // filtered copy of the decode list, so this is the case that says the
        // rule is actually consulted: the muted station is the best CQ on the
        // band, and it must still lose to a weaker one.
        let d = [cq("19LR121", -5, 1), cq("14AT276", 20, 2), cq("30AO010", 3, 3)];
        let muted = ["14AT276".to_string()];
        let got = pick_cq(&d, &log_with(&[]), "26AT715", "", &HashSet::new(), |d| {
            d.from.as_deref().is_some_and(|c| muted.iter().any(|m| m == c))
        })
        .expect("a CQ");
        assert_eq!(got.call, "30AO010", "the muted station is the strongest and must not win");
    }

    #[test]
    fn ignores_a_cq_aimed_elsewhere() {
        let mut d = cq("JA1ABC", 20, 1);
        d.cq_to = Some("JA".to_string());
        // Not a Japanese station: "CQ JA" is not ours to answer.
        assert!(pick_cq(&[d], &log_with(&[]), "26AT715", "", &HashSet::new(), |_| false).is_none());
    }

    #[test]
    fn ignores_non_cq_traffic() {
        let mut d = cq("19LR121", 20, 1);
        d.is_cq = false;
        assert!(pick_cq(&[d], &log_with(&[]), "26AT715", "", &HashSet::new(), |_| false).is_none());
    }

    #[test]
    fn readiness_needs_an_ft_mode_where_tx_is_permitted_and_a_watchdog() {
        assert!(auto_ready(Mode::Ft8, 6, true));
        assert!(auto_ready(Mode::Ft4, 6, true));
        assert!(auto_ready(Mode::Ft2, 1, true));
        // Wrong mode, no permission on this band, no watchdog.
        assert!(!auto_ready(Mode::Js8, 6, true));
        assert!(!auto_ready(Mode::Ft8, 6, false));
        assert!(!auto_ready(Mode::Ft8, 0, true));
    }

    #[test]
    fn the_licence_gate_opens_amateur_bands_and_11m_once_opted_in() {
        // An amateur band is always keyable under the gate.
        assert!(band_may_transmit(Band::M20, true, false));
        // 11 m only once opened.
        assert!(!band_may_transmit(Band::M11, true, false));
        assert!(band_may_transmit(Band::M11, true, true));
        // Broadcast and general coverage stay locked either way.
        assert!(!band_may_transmit(Band::Sw, true, true));
        assert!(!band_may_transmit(Band::Gen, true, true));
        // The gate off opens everything, as `tx_ham_only = false` means.
        assert!(band_may_transmit(Band::Gen, false, false));
    }

    #[test]
    fn the_inactivity_stop_is_clamped_to_the_ceiling() {
        assert_eq!(auto_idle_stop_s(0, false), Some(AUTO_IDLE_STOP_S), "zero means the default");
        assert_eq!(auto_idle_stop_s(20, false), Some(20.0 * 60.0));
        assert_eq!(auto_idle_stop_s(45, false), Some(45.0 * 60.0));
        assert_eq!(auto_idle_stop_s(120, false), Some(45.0 * 60.0), "never past the ceiling");
        assert_eq!(
            auto_idle_stop_s(AUTO_IDLE_STOP_NEVER, false),
            Some(45.0 * 60.0),
            "DISABLE carried to another radio still stops at the ceiling"
        );
    }

    #[test]
    fn a_pluto_may_run_up_to_six_hours_or_never_stop() {
        assert!(has_long_idle_stop(crate::Backend::Pluto));
        assert!(!has_long_idle_stop(crate::Backend::SdrPlay));
        assert!(!has_long_idle_stop(crate::Backend::RtlSdr));
        assert_eq!(auto_idle_stop_s(0, true), Some(AUTO_IDLE_STOP_S), "zero means the default");
        for m in [60, 120, 240, 360] {
            assert_eq!(auto_idle_stop_s(m, true), Some(f64::from(m) * 60.0), "{m} min on a Pluto");
        }
        assert_eq!(auto_idle_stop_s(AUTO_IDLE_STOP_NEVER, true), None, "DISABLE never stops");
        assert_eq!(AUTO_IDLE_STOP_LONG_CHOICES[0], (AUTO_IDLE_STOP_S / 60.0) as u32);
    }
}
