//! The aircraft-navigation tables: airports, and radio navigation aids.
//!
//! Baked into the binary from `assets/airdata/`, built by
//! `assets/airdata/make_airdata.py` — the same arrangement as the Earth rasters
//! beside them. **Source: OurAirports, which is public domain** (the Unlicense),
//! so there is nothing to attribute and nothing to pay: see the house rule on
//! paid and keyed sources in `AGENTS.md`. Nothing here fetches anything, which
//! is the point — the maps work with the network unplugged.
//!
//! Two tables rather than one because they answer two questions. An airport is
//! a *place*, and the map wants the few a person could fly to. A navaid is a
//! *radio station*, and the map wants all of them — 6 700 are NDBs below
//! 1 800 kHz, which are receivable on an ordinary medium-wave receiver, so the
//! navaid table is a listening target list as much as a map layer.
//!
//! The format is a magic, a count, then fixed-size records — see the generator
//! for the layout, which is the only place it is written down twice.
//!
//! # Why these are `f64` positions from `f32` bytes
//!
//! The file stores `f32`, because a map position does not need more — about a
//! metre at the equator. They are widened once on load so everything downstream
//! works in the same precision as the rest of the geometry code, rather than
//! every caller remembering which tables are short.

// **Nothing draws from these yet.** The tables are the data half of the survey
// map's airport and navaid layers; the drawing is the next step, and until it
// lands every type here is dead code and warns as such on every target. An
// allow rather than eighteen warnings, because a tree that warns trains
// everyone to read past warnings — and these are exactly the warnings that
// would hide a real one. Take this off with the first consumer.
use core::sync::atomic::{AtomicU8, Ordering};

const NAVAIDS: &[u8] = include_bytes!("../assets/airdata/navaids.bin");
const AIRPORTS: &[u8] = include_bytes!("../assets/airdata/airports.bin");

/// The kind of radio navigation aid — which is also its symbol on a chart, and
/// which bands it can be heard on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavaidKind {
    /// Non-directional beacon. **Medium wave**, 190–1750 kHz: the one an SWL
    /// can hear on a broadcast receiver.
    Ndb,
    /// VHF omnidirectional range, 108–118 MHz.
    Vor,
    /// A VOR with distance-measuring equipment on the same site.
    VorDme,
    /// A VOR paired with a military TACAN: one station, both services.
    Vortac,
    /// TACAN alone — military, UHF.
    Tacan,
    /// Distance-measuring equipment alone.
    Dme,
    NdbDme,
}

impl NavaidKind {
    fn from_index(i: u8) -> NavaidKind {
        match i {
            0 => NavaidKind::Ndb,
            1 => NavaidKind::Vor,
            2 => NavaidKind::VorDme,
            3 => NavaidKind::Vortac,
            4 => NavaidKind::Tacan,
            5 => NavaidKind::Dme,
            _ => NavaidKind::NdbDme,
        }
    }

    /// Whether this aid transmits below 1 800 kHz, where a short-wave listener
    /// can hear it on a medium-wave receiver.
    pub fn is_audible_on_mf(self) -> bool {
        matches!(self, NavaidKind::Ndb | NavaidKind::NdbDme)
    }

    /// The three or four letters a chart writes beside the symbol.
    ///
    /// `allow(dead_code)`: the map writes the *ident* beside a navaid today,
    /// and this is what the data block and the hover will say when the nearest
    /// aid is named there — the next step of the layer this table was added for.
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            NavaidKind::Ndb => "NDB",
            NavaidKind::Vor => "VOR",
            NavaidKind::VorDme => "VOR/DME",
            NavaidKind::Vortac => "VORTAC",
            NavaidKind::Tacan => "TACAN",
            NavaidKind::Dme => "DME",
            NavaidKind::NdbDme => "NDB/DME",
        }
    }
}

/// One radio navigation aid.
#[derive(Debug, Clone, PartialEq)]
pub struct Navaid {
    pub ident: String,
    pub name: String,
    pub kind: NavaidKind,
    /// ISO country code, for a flag or a filter.
    pub country: String,
    /// Published frequency in kHz — the unit both kinds are published in, so
    /// 291 is an NDB and 116_300 is a VOR. 0 means the source published none.
    pub freq_khz: u32,
    pub lat: f64,
    pub lon: f64,
}

/// One airport a person could actually fly to.
#[derive(Debug, Clone, PartialEq)]
pub struct Airport {
    /// The ICAO or local ident, e.g. `EHAM`.
    pub ident: String,
    pub name: String,
    /// The three-letter code on a bag tag — empty for most small fields.
    pub iata: String,
    pub icao: String,
    /// A large airport, as opposed to a medium one. The two are kept apart
    /// because the map draws them differently and a zoomed-out view wants only
    /// the large ones.
    pub large: bool,
    pub lat: f64,
    pub lon: f64,
    pub elev_ft: i16,
}

/// Read a NUL-padded field, losing the padding.
fn field(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn f32_at(b: &[u8], at: usize) -> f64 {
    f64::from(f32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]))
}

/// The record size the generator writes, asserted against the count on load.
const NAVAID_LEN: usize = 36;
const AIRPORT_LEN: usize = 46;

fn parse_navaids(blob: &[u8]) -> Vec<Navaid> {
    let Some(rest) = blob.strip_prefix(b"SDXNAV01") else {
        eprintln!("sdroxide: navaids.bin is not a navaid table");
        return Vec::new();
    };
    let count = usize::from(u16_at(rest, 0));
    let body = &rest[2..];
    if body.len() < count * NAVAID_LEN {
        eprintln!("sdroxide: navaids.bin is short of its own count");
        return Vec::new();
    }
    (0..count)
        .map(|i| {
            let r = &body[i * NAVAID_LEN..(i + 1) * NAVAID_LEN];
            Navaid {
                ident: field(&r[0..5]),
                name: field(&r[5..21]),
                kind: NavaidKind::from_index(r[21]),
                country: field(&r[22..24]),
                freq_khz: u32_at(r, 24),
                lat: f32_at(r, 28),
                lon: f32_at(r, 32),
            }
        })
        .collect()
}

fn parse_airports(blob: &[u8]) -> Vec<Airport> {
    let Some(rest) = blob.strip_prefix(b"SDXAPT01") else {
        eprintln!("sdroxide: airports.bin is not an airport table");
        return Vec::new();
    };
    let count = usize::from(u16_at(rest, 0));
    let body = &rest[2..];
    if body.len() < count * AIRPORT_LEN {
        eprintln!("sdroxide: airports.bin is short of its own count");
        return Vec::new();
    }
    (0..count)
        .map(|i| {
            let r = &body[i * AIRPORT_LEN..(i + 1) * AIRPORT_LEN];
            Airport {
                ident: field(&r[0..8]),
                name: field(&r[8..28]),
                iata: field(&r[28..31]),
                icao: field(&r[31..35]),
                large: r[35] == 0,
                lat: f32_at(r, 36),
                lon: f32_at(r, 40),
                elev_ft: i16::from_le_bytes([r[44], r[45]]),
            }
        })
        .collect()
}

/// Every navigation aid, read once.
pub fn navaids() -> &'static [Navaid] {
    static ONCE: std::sync::OnceLock<Vec<Navaid>> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| parse_navaids(NAVAIDS))
}

/// Every large and medium airport, read once.
pub fn airports() -> &'static [Airport] {
    static ONCE: std::sync::OnceLock<Vec<Airport>> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| parse_airports(AIRPORTS))
}

/// Whether the tables were read at all — a data file that failed its magic
/// leaves empty tables, and empty tables look exactly like "no airports near
/// here" on a map. A screen that can tell the difference can say so.
pub fn loaded() -> bool {
    static STATE: AtomicU8 = AtomicU8::new(0);
    match STATE.load(Ordering::Relaxed) {
        1 => true,
        2 => false,
        _ => {
            let ok = !navaids().is_empty() && !airports().is_empty();
            STATE.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
            ok
        }
    }
}

/// The nearest navaid of any kind to a position, within `max_km`.
///
/// `allow(dead_code)`: added with the tables and used by the aircraft data
/// block, which names the nearest aid — the step after the map layer. Tested
/// here in the meantime, so it is not untried code waiting for a caller.
///
/// A linear scan, and deliberately: 11 000 records is a few hundred
/// microseconds, and this is asked once per aircraft per *fix* rather than once
/// per frame. It becomes a grid the day something asks it sixty times a second.
#[allow(dead_code)]
pub fn nearest_navaid(lat: f64, lon: f64, max_km: f64) -> Option<(&'static Navaid, f64)> {
    let mut best: Option<(&Navaid, f64)> = None;
    for n in navaids() {
        let d = sdroxide_types::distance_km((lat, lon), (n.lat, n.lon));
        if d <= max_km && best.is_none_or(|(_, b)| d < b) {
            best = Some((n, d));
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tables are the world's, and they read: the numbers come from the
    /// generator's own output, so a rebuild that drops a table fails here
    /// rather than showing an empty map.
    #[test]
    fn the_tables_are_the_worlds() {
        assert!(loaded(), "the assets did not load — check make_airdata.py");
        assert_eq!(navaids().len(), 11_008);
        assert_eq!(airports().len(), 5_280);
    }

    /// **A navaid either exists where and as the source says it does, or the
    /// parser is wrong.** Amsterdam's Schiphol VOR and a named NDB, both read
    /// back field by field against OurAirports' own rows.
    #[test]
    fn a_known_navaid_reads_back_field_for_field() {
        // Filtered by country because `SPL` is **two** navaids: Schiphol's here
        // and Split's in Croatia. The first draft of this test asserted a
        // frequency from memory, and the source row says 108 400 — the same
        // lesson as the FSK441 and RATT entries: read the row, do not recall it.
        let vor = navaids()
            .iter()
            .find(|n| n.ident == "SPL" && n.country == "NL")
            .expect("Schiphol's VOR is in the table");
        assert_eq!(vor.kind, NavaidKind::VorDme);
        assert_eq!(vor.freq_khz, 108_400);
        assert_eq!(vor.name, "Schiphol");
        assert!((vor.lat - 52.332).abs() < 0.01, "lat {}", vor.lat);
        assert!((vor.lon - 4.750).abs() < 0.01, "lon {}", vor.lon);
        assert!(
            navaids().iter().any(|n| n.ident == "SPL" && n.country == "HR"),
            "the ident is not unique, which is why the lookup filters on country"
        );

        let ndb = navaids()
            .iter()
            .find(|n| n.ident == "1A")
            .expect("Williams Harbour's NDB is in the table");
        assert_eq!(ndb.kind, NavaidKind::Ndb);
        assert_eq!(ndb.freq_khz, 373);
        assert_eq!(ndb.name, "Williams Harbour");
    }

    /// The listening half: most of the table is receivable on a medium-wave
    /// receiver, which is what makes it worth shipping for an SWL rather than
    /// only for the air picture.
    #[test]
    fn most_of_the_navaids_are_audible_on_medium_wave() {
        let mf: Vec<&Navaid> = navaids().iter().filter(|n| n.kind.is_audible_on_mf()).collect();
        assert!(mf.len() > 6_000, "only {} MF navaids — is the table truncated?", mf.len());
        assert!(
            mf.iter().all(|n| n.freq_khz < 1_800),
            "an MF navaid is published on a VHF frequency"
        );
    }

    /// Nothing in either table is off the Earth, and nothing is a NaN — the
    /// two ways a float format fails quietly and puts a symbol in the sea.
    #[test]
    fn every_position_is_on_the_earth() {
        for n in navaids() {
            assert!(
                (-90.0..=90.0).contains(&n.lat) && (-180.0..=180.0).contains(&n.lon),
                "{} is at {}, {}",
                n.ident,
                n.lat,
                n.lon
            );
        }
        for a in airports() {
            assert!(
                (-90.0..=90.0).contains(&a.lat) && (-180.0..=180.0).contains(&a.lon),
                "{} is at {}, {}",
                a.ident,
                a.lat,
                a.lon
            );
        }
    }

    /// A file that is not one of ours is refused rather than half-read.
    #[test]
    fn a_foreign_file_is_refused() {
        assert!(parse_navaids(b"not a table").is_empty());
        assert!(parse_airports(b"not a table").is_empty());
        // Right magic, wrong length: the count says more than is there.
        assert!(parse_navaids(b"SDXNAV01\xff\xffshort").is_empty());
    }

    /// The nearest-aid lookup finds a station that is really there and refuses
    /// to invent one that is not.
    #[test]
    fn the_nearest_navaid_is_where_it_should_be() {
        // Amsterdam, and the middle of the Atlantic.
        let (n, _) = nearest_navaid(52.3, 4.8, 60.0).expect("something is near Amsterdam");
        assert_eq!(n.country, "NL");
        assert!(nearest_navaid(0.0, -30.0, 60.0).is_none(), "found a navaid mid-Atlantic");
    }
}
