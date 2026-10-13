//! The LISTEN window — the schedule on a map, and a comment on what is on it.
//!
//! The operator's description, which is the spec: *"a listen panel with a map on
//! in the right part and station info in the left part where they can
//! immediately add a comment and hit log."*
//!
//! **`SWL-HANDOVER.md` §12** has the design and what already exists. The short
//! version: the reception log (`swl_log.rs`) already has the comment and the
//! schedule pre-fill, and no window anywhere in the tree is map-driven — so this
//! is a map-first way to fill the same log, not a new log.


// ── the pure half of the LISTEN window ──────────────────────────────────────

/// What the LISTEN map is showing, and what it chose to show.
///
/// Split out from the drawing because this is the part with decisions in it —
/// which stations are worth a dot, and what happens when two of them land on the
/// same few pixels — and decisions are what need tests. The drawing only reads
/// this.
#[derive(Debug, Clone, Default)]
pub(in crate::app) struct ListenSet {
    /// The schedule stations in the span around the dial, as map marks.
    pub stations: Vec<crate::digi_map::DigiStation>,
    /// Previously logged stations, as map marks — **only** when the history
    /// toggle is on.
    pub logged: Vec<(f64, f64, (u8, u8, u8))>,
    /// How many schedule stations were considered and dropped, by reason.
    ///
    /// Shown on the window, because a listener looking at a map that is nearly
    /// empty is entitled to know whether the band is quiet or the picture is
    /// lying to them. An operator who cannot tell those apart stops trusting the
    /// map, which is worse than having no map.
    pub on_band: usize,
    pub dropped_no_position: usize,
    pub dropped_off_band: usize,
    /// Names of the stations nearest the dial, for the side list.
    pub near: Vec<String>,
}

/// Stations worth plotting, and the ones dropped, from a schedule and a dial.
///
/// **The span is the panadapter's**, not the band plan's: a listener with 10 kHz
/// of span wants the stations **in that window**, and a whole band would put a
/// thousand dots on screen for a fraction of them audible. Half a span either
/// side is drawn so a station just off the edge is a scroll away rather than
/// absent.
pub(in crate::app) fn listen_set(
    schedule: &[sdroxide_types::BroadcastStation],
    dial_hz: f64,
    span_hz: f64,
    home: Option<(f64, f64)>,
    history: bool,
    logged: &[sdroxide_types::SwlEntry],
) -> ListenSet {
    use sdroxide_types::BroadcastStation;
    let mut out = ListenSet::default();
    // A panadapter showing nothing has not settled yet; a floor keeps the map
    // useful in the moment after start-up rather than blank.
    let half = (span_hz / 2.0).max(4_000.0);

    let mut considered = 0usize;
    for s in schedule {
        if (s.freq_hz() - dial_hz).abs() > half {
            out.dropped_off_band += 1;
            continue;
        }
        considered += 1;
        let (Some(lat), Some(lon)) = (s.lat, s.lon) else {
            // No position means no dot. This is the schedule's own gap: a
            // transmitter given as a town and not coordinates, which Aoki's
            // file fills for 99.9% of rows and EiBi's for rather fewer.
            out.dropped_no_position += 1;
            continue;
        };
        // A decode of our own signal sits on our own locator and has no path to
        // draw, so it is not worth a mark either.
        if let Some(h) = home
            && (lat - h.0).abs() < 1e-9
            && (lon - h.1).abs() < 1e-9
        {
            continue;
        }
        out.stations.push(crate::digi_map::DigiStation {
            lat,
            lon,
            fade: 1.0,
            call: Some(std::sync::Arc::from(s.name.as_str())),
            band: None,
        });
    }
    out.on_band = considered;

    // The nearest few by frequency, for a side list the operator can click
    // without hunting on the map.
    let mut near: Vec<&BroadcastStation> = schedule
        .iter()
        .filter(|s| (s.freq_hz() - dial_hz).abs() <= half && s.lat.is_some())
        .collect();
    near.sort_by(|a, b| {
        (a.freq_hz() - dial_hz)
            .abs()
            .total_cmp(&(b.freq_hz() - dial_hz).abs())
    });
    out.near = near
        .into_iter()
        .take(12)
        .map(|s| s.name.clone())
        .collect();

    if history {
        for e in logged {
            let (Some(lat), Some(lon)) = (e.lat, e.lon) else { continue };
            out.logged.push((lat, lon, logged_tint(e.pirate)));
        }
    }
    out
}

/// A logged station's colour: a pirate in the alert red, an ordinary reception in
/// the log's own ink.
///
/// Two populations on one map have to be told apart at a glance, and the tint is
/// the mechanism the map already has — a logged station is passed as a *spot*,
/// which the map draws as a diamond rather than a dot. Shape says "logged", hue
/// says "pirate", and neither has to be read.
fn logged_tint(pirate: bool) -> (u8, u8, u8) {
    if pirate { (232, 64, 64) } else { (96, 176, 224) }
}

/// The station nearest a clicked map position, within `tol_deg`.
///
/// Clicking a map gives **coordinates, not a station** — and on a world map a
/// pixel is a degree or two, so "what did I click" is a nearest-neighbour
/// question with a tolerance. No match within the tolerance is `None` rather than
/// the closest one regardless of distance: clicking open ocean should select
/// nothing, not something 8,000 km away.
pub(in crate::app) fn station_near(
    schedule: &[sdroxide_types::BroadcastStation],
    at: (f64, f64),
    tol_deg: f64,
) -> Option<&sdroxide_types::BroadcastStation> {
    let mut best: Option<(f64, &sdroxide_types::BroadcastStation)> = None;
    for s in schedule {
        let (Some(lat), Some(lon)) = (s.lat, s.lon) else { continue };
        let d = ((lat - at.0).powi(2) + (lon - at.1).powi(2)).sqrt();
        if d <= tol_deg && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, s));
        }
    }
    best.map(|(_, s)| s)
}

#[cfg(test)]
mod tests {
    use super::{listen_set, station_near};
    use sdroxide_types::BroadcastStation;
    use sdroxide_types::SwlEntry;

    fn station(name: &str, khz: f64, lat: Option<f64>, lon: Option<f64>) -> BroadcastStation {
        BroadcastStation {
            name: name.into(),
            freq_khz: khz,
            site: String::new(),
            country: String::new(),
            lat,
            lon,
            ..Default::default()
        }
    }

    /// The span is the **panadapter's**, so a 10 kHz view with 40 stations in
    /// the band puts on screen the few inside the window — not all forty.
    #[test]
    fn only_the_span_around_the_dial_is_plotted() {
        let s = vec![
            station("in span", 6_190.0, Some(50.0), Some(5.0)),
            station("just outside", 6_196.0, Some(50.0), Some(6.0)),
            station("far away", 7_200.0, Some(50.0), Some(7.0)),
        ];
        let set = listen_set(&s, 6_190_000.0, 10_000.0, None, false, &[]);
        let names: Vec<&str> = set.stations.iter().filter_map(|d| d.call.as_deref()).collect();
        assert_eq!(names, vec!["in span"], "only the span: {names:?}");
        assert_eq!(set.dropped_off_band, 2);
        assert_eq!(set.on_band, 1);
    }

    /// A station with no coordinates cannot have a dot, and the count of them
    /// must be **reported** — a map that is empty because the schedule had no
    /// positions looks identical to a map that is empty because the band is
    /// quiet, and the operator cannot tell those apart without being told.
    #[test]
    fn a_station_without_a_position_is_dropped_and_counted() {
        let s = vec![
            station("placed", 6_190.0, Some(50.0), Some(5.0)),
            station("no position", 6_191.0, None, None),
            station("half placed", 6_192.0, Some(50.0), None),
        ];
        let set = listen_set(&s, 6_190_000.0, 20_000.0, None, false, &[]);
        assert_eq!(set.stations.len(), 1, "only the placed one is drawn");
        assert_eq!(
            set.dropped_no_position, 2,
            "a half-placed station is as unplottable as an unplaced one"
        );
        assert_eq!(set.on_band, 3, "all three were on the band");
    }

    /// **History is a toggle, off by default** — the operator's reason being
    /// that an experienced listening station does not want its history
    /// cluttering the band. This is that decision, pinned.
    #[test]
    fn the_history_is_off_unless_it_is_asked_for() {
        let logged = SwlEntry {
            station: "old catch".into(),
            lat: Some(48.0),
            lon: Some(11.0),
            ..Default::default()
        };
        let s = vec![station("on band", 6_190.0, Some(50.0), Some(5.0))];

        let off = listen_set(&s, 6_190_000.0, 20_000.0, None, false, &[logged.clone()]);
        assert!(off.logged.is_empty(), "history off must draw nothing of it");

        let on = listen_set(&s, 6_190_000.0, 20_000.0, None, true, &[logged]);
        assert_eq!(on.logged.len(), 1, "history on draws what was logged");
        assert_eq!(on.stations.len(), 1, "and does not disturb what is on now");
    }

    /// A logged station with no coordinates is skipped rather than drawn at
    /// 0,0 — which is the Gulf of Guinea, and reads as a real reception there.
    #[test]
    fn a_logged_station_without_coordinates_is_not_drawn_at_zero() {
        let logged = SwlEntry { station: "typed by hand".into(), ..Default::default() };
        let set = listen_set(&[], 6_190_000.0, 20_000.0, None, true, &[logged]);
        assert!(set.logged.is_empty(), "absent coordinates must not become 0,0");
    }

    /// A pirate reads differently from an ordinary reception, because "is this
    /// one a pirate" is the question a listener is asking when they open their
    /// own history.
    #[test]
    fn a_pirate_is_tinted_apart_from_an_ordinary_reception() {
        let mut pirate = SwlEntry {
            station: "pirate".into(),
            lat: Some(48.0),
            lon: Some(11.0),
            ..Default::default()
        };
        pirate.pirate = true;
        let ordinary = SwlEntry {
            station: "legal".into(),
            lat: Some(49.0),
            lon: Some(12.0),
            ..Default::default()
        };
        let set = listen_set(&[], 6_190_000.0, 20_000.0, None, true, &[pirate, ordinary]);
        assert_eq!(set.logged.len(), 2);
        assert_ne!(
            set.logged[0].2, set.logged[1].2,
            "a pirate and a legal station must not wear the same colour"
        );
    }

    /// Clicking the map gives coordinates, and a pixel on a world map is a
    /// degree or two — so this is nearest-neighbour **within a tolerance**.
    #[test]
    fn a_click_picks_the_station_under_it() {
        let s = vec![
            station("near the click", 6_190.0, Some(50.0), Some(5.0)),
            station("elsewhere", 6_190.0, Some(-33.0), Some(151.0)),
        ];
        let hit = station_near(&s, (50.05, 5.05), 2.0).expect("a click 0.07° away must hit");
        assert_eq!(hit.name, "near the click");
        // And the far one is never preferred just because it is in the list.
        let other = station_near(&s, (-33.05, 151.05), 2.0).expect("the other must hit too");
        assert_eq!(other.name, "elsewhere");
    }

    /// **Clicking open ocean selects nothing.** Without the tolerance, "the
    /// closest station" would happily return one 8,000 km away, and the side
    /// panel would fill in with a station the operator did not click.
    #[test]
    fn a_click_in_open_water_selects_nothing() {
        let s = vec![station("Berlin", 6_190.0, Some(52.5), Some(13.4))];
        // Mid-Atlantic.
        assert!(
            station_near(&s, (0.0, -30.0), 2.0).is_none(),
            "nothing is near 0,-30 and something was selected anyway"
        );
        // And a station with no position is never "near" anything.
        let s2 = vec![station("unplaced", 6_190.0, None, None)];
        assert!(station_near(&s2, (0.0, 0.0), 180.0).is_none());
    }

    /// Two stations on the same few pixels: the **closest** wins, deterministically.
    #[test]
    fn the_closest_of_two_nearby_stations_wins() {
        let s = vec![
            station("further", 6_190.0, Some(50.0), Some(5.0)),
            station("closer", 6_190.0, Some(50.05), Some(5.05)),
        ];
        let hit = station_near(&s, (50.06, 5.06), 2.0).unwrap();
        assert_eq!(hit.name, "closer");
        // Order of the schedule must not change the answer.
        let mut reversed = s.clone();
        reversed.reverse();
        assert_eq!(station_near(&reversed, (50.06, 5.06), 2.0).unwrap().name, "closer");
    }

    /// Our own locator gets no dot and no path — a reception of a local
    /// transmitter is a real thing to log, but as a map mark it sits on top of
    /// the home marker and hides it.
    #[test]
    fn a_station_on_our_own_locator_is_not_plotted_over_home() {
        let home = (52.0, 5.0);
        let s = vec![
            station("our own local tx", 6_190.0, Some(52.0), Some(5.0)),
            station("a real one", 6_191.0, Some(50.0), Some(6.0)),
        ];
        let set = listen_set(&s, 6_190_000.0, 20_000.0, Some(home), false, &[]);
        assert_eq!(set.stations.len(), 1, "the local one must not cover home");
    }
}
