//! SSTV modem: image ⇄ audio for the Scottie, Martin, and Robot modes.
//!
//! Transmit builds a per-mode timing plan and synthesises tones with a
//! continuous phase accumulator (the `psk.rs`/`rtty.rs` idiom). Receive runs an
//! FM discriminator to recover instantaneous frequency, detects the VIS
//! calibration header to pick the mode, then samples pixels line-by-line,
//! re-aligning on each 1200 Hz sync pulse for slant tolerance.
//!
//! Timing follows the canonical N7CXI spec (as used by PySSTV/QSSTV). Colour
//! maps to frequency by black = 1500 Hz, white = 2300 Hz; sync = 1200 Hz.

use std::f64::consts::TAU;

use sdroxide_types::SstvMode;

use crate::Complex32;
use crate::fir::{ComplexFir, bandpass_taps};

const BLACK_HZ: f64 = 1500.0;
const WHITE_HZ: f64 = 2300.0;
const SYNC_HZ: f64 = 1200.0;
const VIS_LEADER_HZ: f64 = 1900.0;
const VIS_BIT1_HZ: f64 = 1100.0;
const VIS_BIT0_HZ: f64 = 1300.0;

// ── FSK ID: the station's callsign, sent as tones after the picture ──
//
// The format is JE3HHT's, as published with MMSSTV and implemented by every
// SSTV program and unattended repeater that reads one. It is a 45.45 baud FSK
// stream — 22 ms a bit — of six-bit symbols, most significant bit first,
// carrying ASCII $20..$5F shifted down to $00..$3F. A whole ID is
// `$2A C1..CN $01 XSUM`, where the checksum is the XOR of the characters
// alone. It is preceded by a 300 ms tone and a 100 ms tone that give a receiver
// something to arm on, and the transition out of the second of those is what
// times every bit that follows.
//
// The point of sending it as tones rather than printing the callsign into the
// picture is that a machine can read it: an SSTV repeater logs and announces
// the station that just sent, which a banner across the top of a JPEG can never
// give it (issue #287).

/// The tone a `1` bit is sent on — and the ID's start bit.
const FSKID_ONE_HZ: f64 = 1900.0;
/// The tone a `0` bit is sent on, and the 100 ms block ahead of the start bit.
const FSKID_ZERO_HZ: f64 = 2100.0;
/// The 300 ms tone that opens the ID. (MMSSTV sends 1900 Hz here in its narrow
/// mode; this is the ordinary one.)
const FSKID_LEADER_HZ: f64 = 1500.0;
const FSKID_LEADER_S: f64 = 0.300;
const FSKID_SYNC_S: f64 = 0.100;
/// 45.45 baud.
const FSKID_BIT_S: f64 = 0.022;
/// Header symbol: "an ID starts here".
const FSKID_HEAD: u8 = 0x2A;
/// Terminator symbol, ahead of the checksum.
const FSKID_END: u8 = 0x01;
/// How many characters of the ID go on the air.
///
/// Not in the specification, which gives no limit. A cap belongs here all the
/// same: every character costs 132 ms of air time after a picture that has
/// already taken a minute or two, and an ID is a callsign — anything longer is
/// a mistake in a settings field rather than something to transmit.
const FSKID_MAX_CHARS: usize = 20;

/// The six-bit symbols of an FSK ID for `text`, ready to be keyed — header,
/// characters, terminator and checksum. Empty when there is nothing to send.
///
/// Characters are folded to upper case (the code space has no lower case) and
/// anything still outside ASCII `$20..$5F` is dropped rather than mangled: a
/// callsign with a stray accent sends the rest of itself instead of a symbol
/// the far end would print as noise.
#[must_use]
pub fn fsk_id_symbols(text: &str) -> Vec<u8> {
    let chars: Vec<u8> = text
        .trim()
        .to_ascii_uppercase()
        .bytes()
        .filter(|b| (0x20..=0x5F).contains(b))
        .take(FSKID_MAX_CHARS)
        .map(|b| b - 0x20)
        .collect();
    if chars.is_empty() {
        return Vec::new();
    }
    let xsum = chars.iter().fold(0u8, |a, &c| a ^ c);
    let mut out = Vec::with_capacity(chars.len() + 3);
    out.push(FSKID_HEAD);
    out.extend_from_slice(&chars);
    out.push(FSKID_END);
    out.push(xsum);
    out
}

/// The text of a complete FSK ID symbol stream, or `None` when it is not one.
///
/// The inverse of [`fsk_id_symbols`], and the receiver's whole acceptance test:
/// the header, the terminator and the checksum all have to agree before a
/// callsign is believed. That matters more here than it looks — the hunt runs
/// on whatever is on the frequency, so the framing is the only thing standing
/// between the operator and a callsign invented out of noise.
#[must_use]
pub fn fsk_id_text(symbols: &[u8]) -> Option<String> {
    let (head, rest) = symbols.split_first()?;
    if *head != FSKID_HEAD {
        return None;
    }
    let end = rest.iter().position(|&s| s == FSKID_END)?;
    let (chars, tail) = rest.split_at(end);
    if chars.is_empty() || chars.len() > FSKID_MAX_CHARS {
        return None;
    }
    // `tail[0]` is the terminator itself; the checksum is the symbol after it.
    let &xsum = tail.get(1)?;
    if chars.iter().fold(0u8, |a, &c| a ^ c) != xsum {
        return None;
    }
    Some(chars.iter().map(|&c| (c + 0x20) as char).collect::<String>().trim().to_string())
}

/// Frequency (Hz) for an 8-bit intensity, black→white.
fn value_to_hz(v: u8) -> f64 {
    BLACK_HZ + (v as f64 / 255.0) * (WHITE_HZ - BLACK_HZ)
}

/// Inverse of [`value_to_hz`], clamped to a byte.
fn hz_to_value(hz: f64) -> u8 {
    let v = ((hz - BLACK_HZ) / (WHITE_HZ - BLACK_HZ)) * 255.0;
    v.round().clamp(0.0, 255.0) as u8
}

// ───────────────────────────── mode timing ─────────────────────────────

/// A colour channel within a scan segment.
#[derive(Clone, Copy, PartialEq)]
enum Chan {
    R,
    G,
    B,
    /// Luma.
    Y,
    /// Luma of the *second* image row a PD line carries — see
    /// [`SstvMode::rows_per_line`]. Nothing else in the family has one.
    Y2,
    /// R-Y chroma (Cr).
    Cr,
    /// B-Y chroma (Cb).
    Cb,
}

/// One timed segment of a scan line.
#[derive(Clone, Copy)]
enum Seg {
    /// Constant tone for `dur` seconds at `hz`.
    Tone { hz: f64, dur: f64 },
    /// A pixel scan of `width` samples of channel `chan`, `px` seconds each.
    Scan { chan: Chan, width: u16, px: f64 },
}

/// Per-mode parameters used to build a line plan.
struct Timing {
    sync: f64,
    sync_hz: f64,
    sep: f64,
    sep_hz: f64,
    /// Colour-channel pixel time, seconds.
    px: f64,
}

fn scottie_timing(px: f64) -> Timing {
    Timing { sync: 0.009, sync_hz: SYNC_HZ, sep: 0.0015, sep_hz: 1500.0, px }
}

fn martin_timing(px: f64) -> Timing {
    Timing { sync: 0.004_862, sync_hz: SYNC_HZ, sep: 0.000_572, sep_hz: 1500.0, px }
}

/// The PD family's pixel time, seconds. Everything else about a PD line is
/// shared: a 20 ms sync, a 2.08 ms porch, and four full-width scans.
///
/// From JL Barber N7CXI's 2000 mode specification (cross-checked against
/// `windytan/slowrx`'s `modespec.c`). Each one reproduces the published line
/// time exactly — PD90 is 20 + 2.08 + 4 × 320 × 0.532 = 703.04 ms — which is
/// the check worth making on a transcribed table, because a pixel time that is
/// out by a percent still decodes into a picture, just a sheared one.
fn pd_pixel_time(mode: SstvMode) -> f64 {
    match mode {
        SstvMode::Pd50 => 0.000_286,
        SstvMode::Pd90 => 0.000_532,
        SstvMode::Pd120 => 0.000_190,
        SstvMode::Pd160 => 0.000_382,
        SstvMode::Pd180 => 0.000_286,
        SstvMode::Pd240 => 0.000_382,
        _ => 0.000_286, // Pd290
    }
}

/// The ordered segments for one scan line of `mode` at image width `w`.
/// Robot modes carry their (per-line-varying) chroma channel via `line`.
fn line_segments(mode: SstvMode, w: u16, line: u16) -> Vec<Seg> {
    use Chan::*;
    match mode {
        SstvMode::Scottie1 | SstvMode::Scottie2 | SstvMode::ScottieDx => {
            let px = match mode {
                SstvMode::Scottie1 => 0.000_432,
                SstvMode::Scottie2 => 0.000_275_2,
                _ => 0.001_08,
            };
            let t = scottie_timing(px);
            // Scottie order: sep · G · sep · B · SYNC · sep · R.
            vec![
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: G, width: w, px: t.px },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: B, width: w, px: t.px },
                Seg::Tone { hz: t.sync_hz, dur: t.sync },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: R, width: w, px: t.px },
            ]
        }
        SstvMode::Martin1 | SstvMode::Martin2 => {
            let px = if mode == SstvMode::Martin1 { 0.000_457_6 } else { 0.000_228_8 };
            let t = martin_timing(px);
            // Martin order: SYNC · porch · G · sep · B · sep · R · sep.
            //
            // Four separators, not three: a porch after the sync *and* one
            // after every scan, the last one included. Without that trailing
            // pulse the line comes to 445.874 ms against the published
            // 446.446 — 0.13 % short, which the receiver never notices because
            // it re-locks to the sync every line, and which shears a
            // transmitted picture by a third of a line by the bottom. Found by
            // pinning the line times against the published table rather than
            // by looking at the plan.
            vec![
                Seg::Tone { hz: t.sync_hz, dur: t.sync },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: G, width: w, px: t.px },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: B, width: w, px: t.px },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: R, width: w, px: t.px },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
            ]
        }
        SstvMode::Robot72 => {
            // Y full width; Cr, Cb half width. 300 ms/line.
            let cw = w / 2;
            vec![
                Seg::Tone { hz: SYNC_HZ, dur: 0.009 },
                Seg::Tone { hz: 1500.0, dur: 0.003 },
                Seg::Scan { chan: Y, width: w, px: 0.000_431_25 },
                Seg::Tone { hz: 1500.0, dur: 0.0045 },
                Seg::Tone { hz: 1900.0, dur: 0.0015 },
                Seg::Scan { chan: Cr, width: cw, px: 0.000_431_25 },
                Seg::Tone { hz: 2300.0, dur: 0.0045 },
                Seg::Tone { hz: 1900.0, dur: 0.0015 },
                Seg::Scan { chan: Cb, width: cw, px: 0.000_431_25 },
            ]
        }
        SstvMode::Robot36 => {
            // Y full width; one chroma per line, alternating even=Cr / odd=Cb
            // (4:2:0). 150 ms/line. Separator frequency signals which chroma.
            let cw = w / 2;
            let even = line % 2 == 0;
            let (chan, sep_hz) = if even { (Cr, 1500.0) } else { (Cb, 2300.0) };
            vec![
                Seg::Tone { hz: SYNC_HZ, dur: 0.009 },
                Seg::Tone { hz: 1500.0, dur: 0.003 },
                Seg::Scan { chan: Y, width: w, px: 0.000_275 },
                Seg::Tone { hz: sep_hz, dur: 0.0045 },
                Seg::Tone { hz: 1900.0, dur: 0.0015 },
                Seg::Scan { chan, width: cw, px: 0.000_275 },
            ]
        }
        SstvMode::WraaseSc2_180 | SstvMode::WraaseSc2_120 => {
            // Wraase SC-2: sync, a short porch, then R, G, B at full width and
            // no separators between them — the one common family that sends
            // red first.
            // Scan time / width, derived from the published line times so the
            // total comes out exactly: SC-2 180 is three 235.000 ms scans,
            // SC-2 120 three of 156.502506 ms. (slowrx's own pixel times are
            // rounded and reproduce neither of its own line times, which is
            // the sort of thing only a test on the total ever notices.)
            let px =
                if mode == SstvMode::WraaseSc2_180 { 0.235 / 320.0 } else { 0.156_502_506 / 320.0 };
            vec![
                Seg::Tone { hz: SYNC_HZ, dur: 0.005_522_5 },
                Seg::Tone { hz: 1500.0, dur: 0.000_5 },
                Seg::Scan { chan: R, width: w, px },
                Seg::Scan { chan: G, width: w, px },
                Seg::Scan { chan: B, width: w, px },
            ]
        }
        SstvMode::Pd50
        | SstvMode::Pd90
        | SstvMode::Pd120
        | SstvMode::Pd160
        | SstvMode::Pd180
        | SstvMode::Pd240
        | SstvMode::Pd290 => {
            // Two image rows per sync: this row's luma, then one pair of
            // chroma scans shared between the two, then the next row's luma.
            // The chroma is full width here, not halved — PD saves its air
            // time vertically rather than horizontally.
            let px = pd_pixel_time(mode);
            vec![
                Seg::Tone { hz: SYNC_HZ, dur: 0.020 },
                Seg::Tone { hz: 1500.0, dur: 0.002_08 },
                Seg::Scan { chan: Y, width: w, px },
                Seg::Scan { chan: Cr, width: w, px },
                Seg::Scan { chan: Cb, width: w, px },
                Seg::Scan { chan: Y2, width: w, px },
            ]
        }
    }
}

// BT.601-ish YUV used by the Robot modes (MMSSTV coefficients).
fn rgb_to_yuv(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let (r, g, b) = (r as f64, g as f64, b as f64);
    let y = 16.0 + (65.738 * r + 129.057 * g + 25.064 * b) / 256.0;
    let cr = 128.0 + (112.439 * r - 94.154 * g - 18.285 * b) / 256.0;
    let cb = 128.0 + (-37.945 * r - 74.494 * g + 112.439 * b) / 256.0;
    (
        y.round().clamp(0.0, 255.0) as u8,
        cr.round().clamp(0.0, 255.0) as u8,
        cb.round().clamp(0.0, 255.0) as u8,
    )
}

fn yuv_to_rgb(y: u8, cr: u8, cb: u8) -> (u8, u8, u8) {
    let y = y as f64 - 16.0;
    let cr = cr as f64 - 128.0;
    let cb = cb as f64 - 128.0;
    let r = 1.164 * y + 1.596 * cr;
    let g = 1.164 * y - 0.392 * cb - 0.813 * cr;
    let b = 1.164 * y + 2.017 * cb;
    (
        r.round().clamp(0.0, 255.0) as u8,
        g.round().clamp(0.0, 255.0) as u8,
        b.round().clamp(0.0, 255.0) as u8,
    )
}

// ─────────────────────────────── transmit ──────────────────────────────

/// SSTV transmitter: turns an RGB image into a stream of audio samples.
pub struct SstvTx {
    rate: f64,
    /// Flattened plan of (frequency, sample-count) tone runs. Scans are expanded
    /// to one entry per pixel up front — a 320×256 image is ~250 k entries, a
    /// few MB, produced once per transmission.
    plan: Vec<(f64, u32)>,
    idx: usize,
    left: u32,
    cur_hz: f64,
    phase: f64,
    total: u64,
    done: u64,
}

impl SstvTx {
    /// Build a transmitter for `mode` from interleaved RGB (`rgb.len() == w*h*3`)
    /// at output sample `rate`, with an optional transmit clock trim `ppm`
    /// (parts-per-million; stretches/compresses the image time-scale to null out
    /// slant against a receiver whose clock differs — tone frequencies are
    /// unaffected).
    pub fn new(mode: SstvMode, rgb: &[u8], w: u16, h: u16, rate: f64, ppm: f32) -> Self {
        let mut plan: Vec<(f64, u32)> = Vec::new();
        // Cumulative-exact sample clock: derive each element's integer sample
        // count from the running fractional time, so per-element rounding never
        // accumulates into image slant (e.g. Scottie 1's 0.432 ms pixel is
        // 20.736 samples at 48 kHz — rounding each to 21 would drift +1.3%).
        // The timing rate carries the ppm trim; the phase accumulator (in
        // `next_block`) uses the true `rate`, so the tone frequencies stay exact.
        let timing_rate = rate * (1.0 + ppm as f64 / 1_000_000.0);
        let mut emitted: i64 = 0;
        let mut t_exact: f64 = 0.0;
        let mut push = |plan: &mut Vec<(f64, u32)>, hz: f64, dur: f64| {
            t_exact += dur * timing_rate;
            let target = t_exact.round() as i64;
            let n = (target - emitted).max(0);
            emitted = target;
            if n > 0 {
                plan.push((hz, n as u32));
            }
        };
        let px_at = |x: usize, y: usize| -> (u8, u8, u8) {
            let i = (y * w as usize + x) * 3;
            (rgb[i], rgb[i + 1], rgb[i + 2])
        };

        // VIS calibration header.
        push(&mut plan, VIS_LEADER_HZ, 0.300);
        push(&mut plan, SYNC_HZ, 0.010);
        push(&mut plan, VIS_LEADER_HZ, 0.300);
        push(&mut plan, SYNC_HZ, 0.030); // start bit
        let code = mode.vis_code();
        let mut parity = 0u8;
        for bit in 0..7 {
            let one = (code >> bit) & 1 == 1;
            parity ^= one as u8;
            push(&mut plan, if one { VIS_BIT1_HZ } else { VIS_BIT0_HZ }, 0.030);
        }
        push(&mut plan, if parity == 1 { VIS_BIT1_HZ } else { VIS_BIT0_HZ }, 0.030);
        push(&mut plan, SYNC_HZ, 0.030); // stop bit

        // Scottie sends a 9 ms starting sync before the very first line.
        if matches!(mode, SstvMode::Scottie1 | SstvMode::Scottie2 | SstvMode::ScottieDx) {
            push(&mut plan, SYNC_HZ, 0.009);
        }

        // One pass per *transmitted* line, which is two image rows in the PD
        // family and one everywhere else.
        let rows = mode.rows_per_line().max(1) as usize;
        for y in (0..h as usize).step_by(rows) {
            // The second row a PD line carries; the last line of an
            // odd-height picture repeats the first rather than sending a row
            // that is not there.
            let y2 = (y + 1).min(h as usize - 1);
            for seg in line_segments(mode, w, y as u16) {
                match seg {
                    Seg::Tone { hz, dur } => push(&mut plan, hz, dur),
                    Seg::Scan { chan, width, px } => {
                        for x in 0..width as usize {
                            // Map the (possibly subsampled) scan x to a source column.
                            let sx = if width == w { x } else { (x * 2).min(w as usize - 1) };
                            let (r, g, b) = px_at(sx, y);
                            let v = match chan {
                                Chan::R => r,
                                Chan::G => g,
                                Chan::B => b,
                                Chan::Y => rgb_to_yuv(r, g, b).0,
                                Chan::Y2 => {
                                    let (r2, g2, b2) = px_at(sx, y2);
                                    rgb_to_yuv(r2, g2, b2).0
                                }
                                // A PD line's chroma belongs to both of its
                                // rows, so it is the mean of the two rather
                                // than the first one's — which is what makes
                                // the pair look like one picture instead of
                                // combed. On every other mode `rows` is 1 and
                                // this averages a row with itself.
                                Chan::Cr | Chan::Cb => {
                                    let (r2, g2, b2) = px_at(sx, y2);
                                    let a = rgb_to_yuv(r, g, b);
                                    let c = rgb_to_yuv(r2, g2, b2);
                                    let (u, v2) =
                                        if chan == Chan::Cr { (a.1, c.1) } else { (a.2, c.2) };
                                    ((u as u16 + v2 as u16) / 2) as u8
                                }
                            };
                            // Cumulative-exact per-pixel timing (no drift/slant).
                            push(&mut plan, value_to_hz(v), px);
                        }
                    }
                }
            }
        }

        let total: u64 = plan.iter().map(|&(_, n)| n as u64).sum();
        SstvTx { rate, plan, idx: 0, left: 0, cur_hz: 0.0, phase: 0.0, total, done: 0 }
    }

    /// Fill `out` with audio; returns the number of real samples written before
    /// the transmission ended (the rest of `out`, if any, is zeroed).
    pub fn next_block(&mut self, out: &mut [f32]) -> usize {
        let mut written = 0;
        for s in out.iter_mut() {
            if self.left == 0 {
                match self.plan.get(self.idx) {
                    Some(&(hz, n)) => {
                        self.cur_hz = hz;
                        self.left = n;
                        self.idx += 1;
                    }
                    None => {
                        *s = 0.0;
                        continue;
                    }
                }
            }
            self.phase += TAU * self.cur_hz / self.rate;
            if self.phase > TAU {
                self.phase -= TAU;
            }
            *s = (self.phase.sin() as f32) * 0.5;
            self.left -= 1;
            self.done += 1;
            written += 1;
        }
        written
    }

    /// True once every planned sample has been emitted.
    pub fn done(&self) -> bool {
        self.idx >= self.plan.len() && self.left == 0
    }

    /// Total number of audio samples this transmission will emit.
    pub fn total_samples(&self) -> u64 {
        self.total
    }

    /// Transmission progress, 0.0..=1.0.
    pub fn progress(&self) -> f32 {
        if self.total == 0 { 1.0 } else { (self.done as f32 / self.total as f32).clamp(0.0, 1.0) }
    }

    /// Append an FSK identification for `id` to the end of the transmission.
    ///
    /// Nothing is added for an empty (or unsendable) `id`, so a station with no
    /// callsign configured transmits exactly what it did before.
    ///
    /// After the picture rather than before it, which is where every program
    /// that sends one puts it: the receiver has by then finished decoding and
    /// gone back to hunting, and an unattended repeater has the whole frame in
    /// hand before it is told whose it was.
    ///
    /// The ID is *not* stretched by the transmit clock trim the picture carries.
    /// That trim exists to null out image slant against a particular receiver's
    /// sound card, which is a property of the two-dimensional picture; a bit
    /// stream a hundredth of a percent long decodes the same either way, and
    /// nothing on the far end is measuring it against the image.
    #[must_use]
    pub fn with_fsk_id(mut self, id: &str) -> Self {
        let symbols = fsk_id_symbols(id);
        if symbols.is_empty() {
            return self;
        }
        // The same cumulative-exact clock the picture plan uses, restarted for
        // the ID: each element's sample count comes from the running total, so
        // rounding 22 ms at 48 kHz (1056 samples exactly) or at 44.1 kHz
        // (970.2) cannot accumulate into a bit-clock drift the far end has to
        // chase.
        let mut emitted: i64 = 0;
        let mut t_exact: f64 = 0.0;
        let mut push = |plan: &mut Vec<(f64, u32)>, hz: f64, dur: f64| {
            t_exact += dur * self.rate;
            let target = t_exact.round() as i64;
            let n = (target - emitted).max(0);
            emitted = target;
            if n > 0 {
                plan.push((hz, n as u32));
            }
        };
        push(&mut self.plan, FSKID_LEADER_HZ, FSKID_LEADER_S);
        push(&mut self.plan, FSKID_ZERO_HZ, FSKID_SYNC_S);
        // The start bit. It is on the same tone as a `1`, so what marks it is
        // the transition out of the 100 ms block above — which is exactly what
        // the receiver times the rest of the stream from.
        push(&mut self.plan, FSKID_ONE_HZ, FSKID_BIT_S);
        for sym in symbols {
            for bit in (0..6).rev() {
                let one = (sym >> bit) & 1 == 1;
                push(&mut self.plan, if one { FSKID_ONE_HZ } else { FSKID_ZERO_HZ }, FSKID_BIT_S);
            }
        }
        self.total = self.plan.iter().map(|&(_, n)| n as u64).sum();
        self
    }
}

// ─────────────────────────────── receive ───────────────────────────────

/// A decoded output from the receiver.
pub enum SstvEvent {
    /// A VIS header identified the mode; a new image is starting.
    ModeDetected(SstvMode),
    /// A finished scan line: `rgb` is `3 * width` bytes at row `y`.
    Line { y: u16, rgb: Vec<u8> },
    /// The current image reached its last line.
    ImageComplete,
    /// A station identified itself in tones after its picture — the FSK ID
    /// every SSTV program and unattended repeater sends and reads.
    FskId(String),
    /// A header arrived, with good parity, for a mode this decoder does not
    /// have. `name` is the mode where the code is an assigned one.
    ///
    /// Worth an event rather than a discard: at that moment the receiver knows
    /// precisely what is being sent and precisely why no picture is coming,
    /// and it is the only moment anything does. Silence here is what made
    /// issue #421 read as a broken decoder rather than as an unimplemented
    /// mode.
    UnsupportedMode { code: u8, name: Option<&'static str> },
}

#[derive(PartialEq)]
enum RxPhase {
    /// Hunting for the VIS leader / decoding VIS.
    Hunt,
    /// Decoding image lines for `mode`.
    Image,
}

/// SSTV receiver. Feed audio with [`SstvRx::process`]; it emits [`SstvEvent`]s.
pub struct SstvRx {
    rate: f64,
    // Down-mix + baseband filter for the discriminator.
    mix_ph: f32,
    mix_inc: f32,
    lpf: ComplexFir,
    prev: Complex32,
    // Frequency (Hz) of the newest sample of the window below — the
    // estimate every decision in the hunt is made on.
    inst_hz: f64,
    // The last `win` lag-one products `z[n]·conj(z[n-1])`, and their sum.
    // The frequency is the angle of the *sum*, not the mean of the angles:
    // a product is weighted by the signal's power, so the clicks noise makes
    // where the amplitude dips — a phase jump of anything up to ±π, which is a
    // ±24 kHz "frequency" at 48 kHz — carry almost no weight instead of
    // dragging the estimate across the whole band. Per-sample angles were
    // what this used before, and they put a weak signal's sync pulses and
    // header out of reach long after the picture itself was readable.
    fast: Window,
    // Half the fast window: the estimate pushed at sample `n` describes
    // sample `n - half`, so every position the hunt records is taken back by
    // that much (see `cursor`).
    half: u64,
    // Tone detectors for the two things the hunt looks for: the 1200 Hz sync
    // tone (a start bit, a line's sync pulse) and the 1900 Hz header leader.
    //
    // Not the frequency estimate above, and the reason is the whole of the
    // weak-signal fault. A frequency measured over a band with noise in it is
    // pulled towards the middle of that band, in proportion to the noise: at
    // 2 dB in 2500 Hz a 1200 Hz start bit measures ~1360 Hz, which fails a
    // ±100 Hz test however long it is averaged. Asking instead how much of
    // the power is *on the tone* has no such bias — noise only lowers the
    // fraction, it does not move the tone — so the threshold can sit where a
    // weak signal still clears it.
    sync_tone: ToneWin,
    lead_tone: ToneWin,
    // Smoothed raw-input level (mean |audio|) for the UI activity meter.
    in_level: f32,
    have_prev: bool,

    phase: RxPhase,
    mode: SstvMode,
    // Rolling ring of recent instantaneous-frequency samples, so we can look
    // back over a whole line once its trailing sync arrives.
    hist: Vec<f64>,
    // The raw lag-one products beside it, same base and length, so a span —
    // a pixel, a header bit — can be measured as one sum rather than read off
    // a single sample. See `hz_over`.
    hist_d: Vec<Complex32>,
    // And the baseband samples themselves, for the tone measurements.
    hist_z: Vec<Complex32>,
    // Samples of history to guarantee (`HIST_S`); `hist` runs a little past it
    // between compactions. See `push_hist`.
    hist_cap: usize,
    // Absolute sample index of hist[0].
    hist_base: u64,
    sample_idx: u64,

    // VIS bit accumulation.
    vis_state: VisState,
    // The FSK ID hunt, which runs whenever no picture is being decoded.
    fsk_state: FskIdState,

    // Image decode bookkeeping.
    line: u16,
    // Sample index where the current line's decoding should start.
    line_start: u64,
    // Length of the current line, cached because `step_image` runs per sample.
    line_samples: u64,
    // Robot 4:2:0 chroma carried between lines.
    last_cr: Vec<u8>,
    last_cb: Vec<u8>,

    // Free-run (decode without VIS): lock onto a regular 1200 Hz sync cadence.
    // `expected` = a specific operator-selected mode, or `None` for auto (match
    // the cadence + sync length against every mode).
    expected: Option<SstvMode>,
    sync_run: u32,
    // Samples since the run last saw the sync tone, while it may yet resume.
    sync_gap: u32,
    // Recent sync pulses as (centre sample, pulse length in samples).
    sync_hist: Vec<(u64, u32)>,
}

/// The hunt for an FSK ID: what has been seen of one so far.
///
/// Deliberately a level trigger followed by a fixed clock, not a per-bit
/// tracking loop. The whole ID is under three seconds and the transmitter's
/// bit clock is its sound card's, so there is nothing to track — and a receiver
/// that resynchronised on every edge would be one more thing to go wrong on a
/// stream that is already protected by a header, a terminator and a checksum.
struct FskIdState {
    /// Consecutive samples of the 100 ms block that arms the hunt.
    arm_run: u32,
    /// Set once that block has run long enough to be one.
    armed: bool,
    /// Sample index of the transition out of it — bit zero's leading edge, and
    /// the origin every bit below is timed from.
    start: Option<u64>,
    /// Data bits taken so far (the start bit is not among them).
    bits: Vec<bool>,
    /// How many of them have been sampled, so each is taken exactly once.
    taken: u32,
    /// Consecutive samples since arming that were on neither tone.
    gap: u32,
}

impl FskIdState {
    fn reset() -> Self {
        FskIdState { arm_run: 0, armed: false, start: None, bits: Vec::new(), taken: 0, gap: 0 }
    }
}

struct VisState {
    // Running count of consecutive ~1900 Hz leader samples.
    leader: u32,
    // A full (>150 ms) leader has been seen at least once.
    leader_seen: bool,
    // Previous sample was ~1200 Hz (rising-edge detection).
    was_sync: bool,
    // Candidate start-bit sample indices awaiting a decode attempt. Both the
    // 10 ms break and the real 30 ms start bit become candidates; the break
    // decodes to VIS code 0 (rejected), so the real start bit wins.
    cands: Vec<u64>,
}

impl VisState {
    fn reset() -> Self {
        VisState { leader: 0, leader_seen: false, was_sync: false, cands: Vec::new() }
    }
}

/// A running sum of the last `len` lag-one products, and the frequency it
/// answers. See `SstvRx::fast`.
struct Window {
    ring: std::collections::VecDeque<Complex32>,
    sum: (f64, f64),
    len: usize,
    pushed: u32,
}

impl Window {
    fn new(len: usize) -> Self {
        Window {
            ring: std::collections::VecDeque::with_capacity(len + 1),
            sum: (0.0, 0.0),
            len,
            pushed: 0,
        }
    }

    /// Take one product; the window's frequency, or `None` while it holds no
    /// signal at all.
    fn push(&mut self, d: Complex32, rate: f64) -> Option<f64> {
        self.ring.push_back(d);
        self.sum.0 += d.re as f64;
        self.sum.1 += d.im as f64;
        if self.ring.len() > self.len {
            if let Some(old) = self.ring.pop_front() {
                self.sum.0 -= old.re as f64;
                self.sum.1 -= old.im as f64;
            }
        }
        // A running sum drifts by rounding; rebuild it now and then.
        self.pushed = self.pushed.wrapping_add(1);
        if self.pushed % 8192 == 0 {
            self.sum =
                self.ring.iter().fold((0.0, 0.0), |(r, i), c| (r + c.re as f64, i + c.im as f64));
        }
        (self.sum.0 != 0.0 || self.sum.1 != 0.0)
            .then(|| 1900.0 + self.sum.1.atan2(self.sum.0) * rate / TAU)
    }
}

/// Length of the sliding sync-tone detector. Shorter than every mode's sync
/// pulse (Martin's 4.9 ms is the shortest), so a pulse fills it.
const SYNC_WIN_S: f64 = 0.004;
/// Fraction of a line's sync window that has to be on the tone before the
/// line is re-timed by it; below that the nominal start stands.
const REALIGN_FRAC: f64 = 0.25;
/// Lines of a mode's grid behind the newest sync pulse the free-run lock
/// looks along, and how many of them have to carry a pulse: fewer when the
/// operator has said which mode it is.
const CADENCE_LINES: u32 = 8;
const CADENCE_AUTO: u32 = 2;
const CADENCE_FIXED: u32 = 1;
/// Seconds of audio history kept: eight lines of Wraase SC-2 180, five of
/// Scottie DX.
const HIST_S: f64 = 6.0;
/// The mean fraction of the power on the sync tone, measured where the
/// cadence says every sync pulse in the history should be, that a lock has to
/// reach. This is the test that carries a weak picture: one pulse at a time a
/// sync at -3 dB is missed as often as found, but measured across six lines at
/// once it stands well clear of noise, which averages near 0.08 here.
const GRID_FRAC: f64 = 0.25;
/// Seconds of sync pulses kept for that: eight lines of Scottie DX.
const CADENCE_KEEP_S: f64 = 9.0;
/// Fraction of the power that has to be on the sync tone to call it present.
/// Noise alone sits near 0.1 in a 4 ms window; a clean tone is 1.0; a tone at
/// 0 dB in the 2200 Hz the front end passes is about a half.
const SYNC_FRAC: f64 = 0.40;
/// The same test for the edge that makes a header candidate. Lower, because a
/// candidate still has to pass the start bit, seven bits and parity before it
/// is believed, where a free-run pulse has only the cadence behind it.
const VIS_EDGE_FRAC: f64 = 0.35;
/// The same for the 1900 Hz leader, over a 20 ms window, where noise alone
/// sits near 0.02.
const LEADER_FRAC: f64 = 0.2;

/// A sliding measure of how much of the power is on one tone. See
/// `SstvRx::sync_tone`.
struct ToneWin {
    step: f64,
    phase: f64,
    ring: std::collections::VecDeque<(Complex32, f32)>,
    sum: (f64, f64),
    pow: f64,
    len: usize,
    pushed: u32,
}

impl ToneWin {
    fn new(hz: f64, len: usize, rate: f64) -> Self {
        ToneWin {
            step: TAU * (hz - 1900.0) / rate,
            phase: 0.0,
            ring: std::collections::VecDeque::with_capacity(len + 1),
            sum: (0.0, 0.0),
            pow: 0.0,
            len,
            pushed: 0,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn push(&mut self, z: Complex32) {
        // Turn the tone down to DC; a sum then measures it.
        let (s, c) = (-self.phase).sin_cos();
        let y = Complex32::new(
            (z.re as f64 * c - z.im as f64 * s) as f32,
            (z.re as f64 * s + z.im as f64 * c) as f32,
        );
        self.phase = (self.phase + self.step).rem_euclid(TAU);
        let p = z.norm_sqr();
        self.ring.push_back((y, p));
        self.sum.0 += y.re as f64;
        self.sum.1 += y.im as f64;
        self.pow += p as f64;
        if self.ring.len() > self.len {
            if let Some((o, op)) = self.ring.pop_front() {
                self.sum.0 -= o.re as f64;
                self.sum.1 -= o.im as f64;
                self.pow -= op as f64;
            }
        }
        self.pushed = self.pushed.wrapping_add(1);
        if self.pushed % 8192 == 0 {
            self.sum = self
                .ring
                .iter()
                .fold((0.0, 0.0), |(r, i), (c, _)| (r + c.re as f64, i + c.im as f64));
            self.pow = self.ring.iter().map(|(_, p)| *p as f64).sum();
        }
    }

    /// The fraction of the window's power on the tone, 0.0..=1.0.
    fn frac(&self) -> f64 {
        if self.ring.len() < self.len || self.pow <= 0.0 {
            return 0.0;
        }
        (self.sum.0 * self.sum.0 + self.sum.1 * self.sum.1) / (self.len as f64 * self.pow)
    }
}

impl SstvRx {
    pub fn new(rate: f64) -> Self {
        let mix_hz = 1900.0f32;
        // Keep enough history for the slowest line + sync search, and for the
        // free-run lock to look back along several lines of a cadence before
        // it believes one (see `grid_frac`).
        let hist_cap = (rate * HIST_S) as usize;
        // 1.5 ms: well inside the shortest sync pulse any mode sends (Martin's
        // 4.9 ms), long enough to average a click away.
        let win = ((rate * 0.0015) as usize).max(1);
        SstvRx {
            rate,
            mix_ph: 0.0,
            mix_inc: (TAU as f32) * mix_hz / rate as f32,
            lpf: ComplexFir::new(bandpass_taps(129, -1100.0, 1100.0, rate)),
            prev: Complex32::new(0.0, 0.0),
            inst_hz: 1900.0,
            fast: Window::new(win),
            half: (win / 2) as u64,
            sync_tone: ToneWin::new(SYNC_HZ, ((rate * SYNC_WIN_S) as usize).max(1), rate),
            lead_tone: ToneWin::new(VIS_LEADER_HZ, ((rate * 0.020) as usize).max(1), rate),
            in_level: 0.0,
            have_prev: false,
            phase: RxPhase::Hunt,
            mode: SstvMode::Scottie1,
            hist: Vec::with_capacity(hist_cap + hist_cap / 4 + 1),
            hist_d: Vec::with_capacity(hist_cap + hist_cap / 4 + 1),
            hist_z: Vec::with_capacity(hist_cap + hist_cap / 4 + 1),
            hist_cap,
            hist_base: 0,
            sample_idx: 0,
            vis_state: VisState::reset(),
            fsk_state: FskIdState::reset(),
            line: 0,
            line_start: 0,
            line_samples: 0,
            last_cr: Vec::new(),
            last_cb: Vec::new(),
            expected: None,
            sync_run: 0,
            sync_gap: 0,
            sync_hist: Vec::new(),
        }
    }

    /// Set the mode used for free-run (no-VIS) decoding, or `None` for auto
    /// (detect the mode from the sync cadence).
    pub fn set_expected(&mut self, mode: Option<SstvMode>) {
        self.expected = mode;
    }

    /// Abandon whatever is being decoded and go back to hunting for a header.
    ///
    /// An SSTV receiver that has locked on is committed for the length of the
    /// mode it locked on to, and the slow modes are long: Scottie DX is four
    /// and a half minutes, PD290 nearly five. A false VIS — or a real one for a
    /// mode the transmitting station did not actually send — therefore takes
    /// the receiver off the air for as long as it takes to run out, and on a
    /// transponder where pictures follow one another that is several missed.
    /// This is the way back (issue #397).
    ///
    /// The history goes with it, deliberately. It holds the last seconds of the
    /// picture being abandoned, sync pulses and all, and the free-run hunt
    /// reads exactly that — left in place it would re-lock on the cadence of
    /// the transmission the operator has just asked to be rid of. The
    /// front-end filter, the level meter and the operator's mode selection are
    /// *not* touched: none of them is about this picture.
    pub fn restart(&mut self) {
        self.phase = RxPhase::Hunt;
        self.vis_state = VisState::reset();
        self.fsk_state = FskIdState::reset();
        self.line = 0;
        self.line_start = 0;
        self.line_samples = 0;
        self.last_cr.clear();
        self.last_cb.clear();
        self.sync_run = 0;
        self.sync_gap = 0;
        self.sync_hist.clear();
        self.hist.clear();
        self.hist_d.clear();
        self.hist_z.clear();
        self.hist_base = self.sample_idx;
    }

    /// The mode currently being decoded (or last detected).
    pub fn mode(&self) -> SstvMode {
        self.mode
    }

    /// Smoothed raw-input level (mean |sample|), for a UI activity meter so the
    /// operator can set their receive gain.
    pub fn level(&self) -> f32 {
        self.in_level
    }

    /// True while an image is being decoded (VIS locked).
    pub fn receiving(&self) -> bool {
        self.phase == RxPhase::Image
    }

    /// Fraction of the current image decoded, 0.0..=1.0.
    pub fn progress(&self) -> f32 {
        if self.phase != RxPhase::Image {
            return 0.0;
        }
        let (_, h) = self.mode.dimensions();
        (self.line as f32 / h.max(1) as f32).clamp(0.0, 1.0)
    }

    /// Feed audio; push any decoded events.
    pub fn process(&mut self, audio: &[f32], out: &mut Vec<SstvEvent>) {
        // Down-mix by 1900 Hz to complex baseband, then low-pass the whole block.
        let mut mixed = Vec::with_capacity(audio.len());
        for &a in audio {
            self.in_level += 0.001 * (a.abs() - self.in_level);
            let z = Complex32::new(a * self.mix_ph.cos(), -a * self.mix_ph.sin());
            self.mix_ph += self.mix_inc;
            if self.mix_ph > std::f32::consts::TAU {
                self.mix_ph -= std::f32::consts::TAU;
            }
            mixed.push(z);
        }
        let mut bb = Vec::with_capacity(audio.len());
        self.lpf.process(&mixed, &mut bb);

        for z in bb {
            // Frequency via the discriminator, averaged over the window.
            let d = if self.have_prev { z * self.prev.conj() } else { Complex32::new(0.0, 0.0) };
            self.prev = z;
            self.have_prev = true;
            let rate = self.rate;
            if let Some(hz) = self.fast.push(d, rate) {
                self.inst_hz = hz;
            }
            self.sync_tone.push(z);
            self.lead_tone.push(z);

            self.push_hist(self.inst_hz, d, z);
            self.sample_idx += 1;

            match self.phase {
                RxPhase::Hunt => self.step_hunt(out),
                RxPhase::Image => self.step_image(out),
            }
        }
    }

    fn push_hist(&mut self, hz: f64, d: Complex32, z: Complex32) {
        self.hist.push(hz);
        self.hist_d.push(d);
        self.hist_z.push(z);
        // Compact in blocks, never per sample. Trimming one element per push
        // memmoves the whole buffer (megabytes at 48 kHz) on every sample
        // — several GB/s of pointless copying that dwarfs the rest of the
        // demodulator and starves the audio and display running on the same
        // thread. Letting it overshoot by a quarter amortises that away; the
        // guarantee callers rely on (at least `hist_cap` samples of history) is
        // unchanged, since the buffer only ever holds *more* than before.
        if self.hist.len() > self.hist_cap + self.hist_cap / 4 {
            let drop = self.hist.len() - self.hist_cap;
            self.hist.drain(0..drop);
            self.hist_d.drain(0..drop);
            self.hist_z.drain(0..drop);
            self.hist_base += drop as u64;
        }
    }

    /// The windowed frequency centred on sample `idx`.
    fn hz_at(&self, idx: u64) -> f64 {
        let idx = idx + self.half;
        if idx < self.hist_base {
            return 1900.0;
        }
        let i = (idx - self.hist_base) as usize;
        self.hist.get(i).copied().unwrap_or(1900.0)
    }

    /// The frequency over samples `from..to`, measured as one sum.
    ///
    /// This is what a pixel and a header bit are read with. Reading one
    /// sample at the centre of each — which is what this did — throws away
    /// all but one of the thirty-odd samples a Wraase pixel spans, and all but
    /// one of the 1440 in a header bit, so the noise that decided a weak
    /// picture's header was the noise on one sample.
    fn hz_over(&self, from: f64, to: f64) -> f64 {
        let a = (from.max(0.0) as u64).max(self.hist_base);
        let b = (to.max(0.0) as u64).max(a + 1);
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for idx in a..b {
            if let Some(d) = self.hist_d.get((idx - self.hist_base) as usize) {
                re += d.re as f64;
                im += d.im as f64;
            }
        }
        if re == 0.0 && im == 0.0 {
            return 1900.0;
        }
        1900.0 + im.atan2(re) * self.rate / TAU
    }

    /// One past the newest sample the windowed estimate is centred on — the
    /// "now" every position the hunt records is measured against. The window
    /// runs half its length behind the newest audio.
    fn cursor(&self) -> u64 {
        self.sample_idx.saturating_sub(self.half)
    }

    /// How much of the power in samples `from..to` is on `hz`: 1.0 for a
    /// clean tone there, near zero for one a few bins away or for noise.
    fn tone_frac(&self, from: f64, to: f64, hz: f64) -> f64 {
        let a = (from.max(0.0) as u64).max(self.hist_base);
        let b = (to.max(0.0) as u64).max(a + 1);
        let w = TAU * (hz - 1900.0) / self.rate;
        let (mut re, mut im, mut p, mut n) = (0.0f64, 0.0f64, 0.0f64, 0u32);
        for idx in a..b {
            let Some(z) = self.hist_z.get((idx - self.hist_base) as usize) else { break };
            let ph = -w * (idx - a) as f64;
            let (s, c) = ph.sin_cos();
            let (zr, zi) = (z.re as f64, z.im as f64);
            re += zr * c - zi * s;
            im += zr * s + zi * c;
            p += zr * zr + zi * zi;
            n += 1;
        }
        if p <= 0.0 || n == 0 {
            return 0.0;
        }
        (re * re + im * im) / (n as f64 * p)
    }

    // ── FSK ID detection ──
    //
    // Runs alongside the VIS hunt rather than after the picture: the two cannot
    // be confused (VIS is 1100/1300 Hz around a 1200 Hz sync, the ID is
    // 1900/2100 with no sync at all), and hunting for it the whole time is what
    // lets a receiver tuned in late — or one whose picture decode never
    // started — still learn who is transmitting.
    fn step_fsk_id(&mut self, out: &mut Vec<SstvEvent>) {
        let near = |a: f64, b: f64| (a - b).abs() < 80.0;
        let bit_samples = FSKID_BIT_S * self.rate;

        // Arming: the 100 ms block on the zero tone. Two thirds of it is enough
        // — the leading edge is where the picture's last pixel ends, and on a
        // real signal that boundary is not clean.
        if self.fsk_state.start.is_none() {
            if near(self.inst_hz, FSKID_ZERO_HZ) {
                self.fsk_state.arm_run += 1;
                self.fsk_state.gap = 0;
                if self.fsk_state.arm_run as f64 > 0.066 * self.rate {
                    self.fsk_state.armed = true;
                }
                return;
            }
            if !self.fsk_state.armed {
                self.fsk_state.arm_run = 0;
                return;
            }
            // Armed and now on the one tone: this is the start bit's leading
            // edge, and the clock for everything after it.
            if near(self.inst_hz, FSKID_ONE_HZ) {
                self.fsk_state.start = Some(self.cursor());
                self.fsk_state.bits.clear();
                self.fsk_state.taken = 0;
                return;
            }
            // In between the two. A 200 Hz shift does not arrive instantly —
            // the discriminator and the filter ahead of it take a moment to
            // follow it — so the samples spanning the edge belong to neither
            // tone, and treating one of them as "this was not an ID after all"
            // threw the arming away a few samples before the start bit it was
            // waiting for. Anything longer than a bit or two is a real signal
            // that simply was not one.
            self.fsk_state.gap += 1;
            if self.fsk_state.gap as f64 > 3.0 * bit_samples {
                self.fsk_state = FskIdState::reset();
            }
            return;
        }

        let Some(start) = self.fsk_state.start else { return };
        // Bit 0 is the start bit and carries nothing, so the data begins at 1.
        let k = self.fsk_state.taken + 1;
        // Sampled at the middle of the bit, where the transitions either side
        // are furthest away.
        let at = start + ((k as f64 + 0.5) * bit_samples) as u64;
        // Strictly greater: `sample_idx` is one *past* the newest sample in the
        // history (it is bumped before the step runs), so waiting only for
        // equality asks `hz_at` for a sample that has not been stored yet — and
        // its "nothing here" answer is 1900 Hz, which is a perfectly good `1`.
        // Every bit read as one, and every ID decoded as $3F $3F $3F…
        if self.cursor() <= at {
            return;
        }
        self.fsk_state.taken += 1;
        let hz = self.hz_at(at);
        if near(hz, FSKID_ONE_HZ) {
            self.fsk_state.bits.push(true);
        } else if near(hz, FSKID_ZERO_HZ) {
            self.fsk_state.bits.push(false);
        } else {
            // Neither tone: the stream has ended (or was never one). Give up
            // and go back to arming rather than keying noise into the symbols.
            self.fsk_state = FskIdState::reset();
            return;
        }

        // A whole symbol has arrived; see whether the stream so far is an ID.
        if self.fsk_state.bits.len() % 6 != 0 {
            return;
        }
        let symbols: Vec<u8> = self
            .fsk_state
            .bits
            .chunks_exact(6)
            .map(|c| c.iter().fold(0u8, |a, &b| (a << 1) | u8::from(b)))
            .collect();
        // A header that is not the header can never become one, so a stream
        // that starts wrong is dropped at the first symbol instead of being
        // carried for the length of a callsign.
        if symbols[0] != FSKID_HEAD {
            self.fsk_state = FskIdState::reset();
            return;
        }
        if let Some(text) = fsk_id_text(&symbols) {
            out.push(SstvEvent::FskId(text));
            self.fsk_state = FskIdState::reset();
            return;
        }
        // Nothing yet, and nothing that can still become something: the
        // longest legal ID is the header, the cap, the terminator and the sum.
        if symbols.len() > FSKID_MAX_CHARS + 3 {
            self.fsk_state = FskIdState::reset();
        }
    }

    // ── VIS detection ──
    fn step_hunt(&mut self, out: &mut Vec<SstvEvent>) {
        self.step_fsk_id(out);
        let is_leader = self.lead_tone.frac() > LEADER_FRAC;
        let is_sync = self.sync_tone.frac() > VIS_EDGE_FRAC;
        // Tolerant leader accumulator: brief noise glitches decrement rather than
        // reset the run, so a real ~300 ms leader still arms through hiss. No
        // amplitude gate — the tone fraction is level-independent, so a clean
        // but quiet signal must still decode; the VIS code + parity check
        // rejects noise. (Noise alone puts about 2 % of a 20 ms window on the
        // leader tone, so 0.12 s of it over the threshold does not happen by
        // chance.)
        if is_leader {
            self.vis_state.leader = (self.vis_state.leader + 1).min((self.rate) as u32);
            if self.vis_state.leader as f64 > 0.12 * self.rate {
                self.vis_state.leader_seen = true;
            }
        } else {
            self.vis_state.leader = self.vis_state.leader.saturating_sub(3);
            // A start bit has to follow a leader that is still *recent*. The
            // flag used to be sticky for the whole hunt, so every 1200 Hz sync
            // pulse of every picture after it went on being offered as a
            // candidate header — harmless while an unrecognised code was
            // dropped in silence, and a stream of false "unsupported mode"
            // reports once one is not. The accumulator drains in about 100 ms,
            // which is well inside the 30 ms between the leader and the start
            // bit it has to survive.
            if self.vis_state.leader == 0 {
                self.vis_state.leader_seen = false;
            }
        }
        // Rising edge into a 1200 Hz pulse after a leader → candidate start bit.
        //
        // On a weak signal the header's own data bits — 1100 and 1300 Hz,
        // either side of the 1200 Hz this looks for — flicker in and out of
        // it many times, and every flicker is a rising edge. Eight places in
        // the queue, oldest dropped, meant the real start bit was pushed out
        // by its own data bits before they had finished arriving. So edges
        // closer than a few milliseconds to the last one are one edge, and the
        // queue is long enough for a whole header's worth.
        if is_sync && !self.vis_state.was_sync && self.vis_state.leader_seen {
            // The window has just filled with the tone: its start is the edge.
            let now = self.sample_idx.saturating_sub(self.sync_tone.len() as u64);
            let near_last = self
                .vis_state
                .cands
                .last()
                .is_some_and(|&c| now.saturating_sub(c) < (0.005 * self.rate) as u64);
            if !near_last {
                self.vis_state.cands.push(now);
                if self.vis_state.cands.len() > 64 {
                    self.vis_state.cands.remove(0);
                }
            }
        }
        self.vis_state.was_sync = is_sync;

        // Try the oldest candidate once its 8 VIS bits have elapsed.
        let bit = 0.030 * self.rate;
        if let Some(&start) = self.vis_state.cands.first() {
            if (self.cursor() as f64) >= start as f64 + 9.5 * bit {
                self.vis_state.cands.remove(0);
                let mut code = 0u8;
                let mut parity = 0u8;
                // Every VIS bit is 1100 or 1300 Hz. A candidate whose bit
                // slots are up at the 1900 Hz leader is not a header at all —
                // which is what the *break* pulse in the middle of the
                // calibration header looks like, since it is a rising edge
                // into 1200 Hz after a leader just like the start bit is. It
                // used to be decoded anyway, reading the leader as eight zero
                // bits with matching parity; harmless while an unknown code
                // was silently dropped, and a false "unsupported mode" report
                // the moment one is not.
                // The start bit itself has to be the sync tone, measured over
                // the bit — the edge that made this a candidate may be noise.
                let (s0, s1) = (start as f64 + 0.15 * bit, start as f64 + 0.85 * bit);
                let on_sync = self.tone_frac(s0, s1, SYNC_HZ);
                let mut looks_like_vis = on_sync > self.tone_frac(s0, s1, VIS_LEADER_HZ)
                    && on_sync > self.tone_frac(s0, s1, VIS_BIT1_HZ)
                    && on_sync > self.tone_frac(s0, s1, VIS_BIT0_HZ);
                // Each bit is whichever of its two tones holds more of the
                // power over the middle 70 % of it, clear of both edges — and
                // a bit holding less of either than the leader is not a bit.
                let mut read_bit = |k: f64| -> u8 {
                    let c = start as f64 + (k + 0.5) * bit;
                    let (a, b) = (c - 0.35 * bit, c + 0.35 * bit);
                    let one = self.tone_frac(a, b, VIS_BIT1_HZ);
                    let zero = self.tone_frac(a, b, VIS_BIT0_HZ);
                    if one.max(zero) < self.tone_frac(a, b, VIS_LEADER_HZ) {
                        looks_like_vis = false;
                    }
                    u8::from(one > zero)
                };
                for b in 0..7 {
                    if read_bit(1.0 + b as f64) == 1 {
                        code |= 1 << b; // 1100 Hz = 1
                        parity ^= 1;
                    }
                }
                let pbit = read_bit(8.0);
                if looks_like_vis && parity == pbit {
                    match SstvMode::from_vis(code) {
                        Some(mode) => {
                            // Image data begins after the stop bit (start + 10
                            // bits); Scottie prefixes a 9 ms starting sync
                            // before line 0.
                            let mut first = start as f64 + 10.0 * bit;
                            if matches!(
                                mode,
                                SstvMode::Scottie1 | SstvMode::Scottie2 | SstvMode::ScottieDx
                            ) {
                                first += 0.009 * self.rate;
                            }
                            self.begin_image(mode, first as u64, out);
                        }
                        // A header that checks out for a mode we cannot draw.
                        // Said once per header rather than swallowed — see
                        // `SstvEvent::UnsupportedMode`. Code 0 is not a mode
                        // anyone has ever been assigned, so a candidate that
                        // reads as one is a misread and says nothing.
                        None if code != 0 && self.preceded_by_leader(start) => {
                            out.push(SstvEvent::UnsupportedMode {
                                code,
                                name: SstvMode::unsupported_name(code),
                            })
                        }
                        None => {}
                    }
                }
            }
        }

        // No VIS yet? Try to lock onto the sync cadence of the selected mode.
        self.try_freerun(out);
    }

    /// Whether the 20 ms before `start` really is the 1900 Hz leader.
    ///
    /// Only asked before *reporting* an unrecognised code, never before
    /// decoding a recognised one: a missed report costs a line of explanation,
    /// and a missed decode costs the picture.
    ///
    /// The hunt pushes a candidate at every rising edge into 1200 Hz while a
    /// leader is anywhere in recent memory, and the VIS bits themselves sweep
    /// through 1200 Hz on their way from 1300 to 1100 — so a header's own data
    /// bits produce candidates of their own, a couple of which read as a code
    /// with matching parity. They are harmless as decodes (their code is not a
    /// mode) and actively wrong as reports, where the last one to arrive would
    /// overwrite the real mode's name. A start bit is preceded by the leader;
    /// a data bit is preceded by another data bit.
    ///
    /// Sampled at several points and decided by majority, so noise on one of
    /// them does not cost the explanation.
    fn preceded_by_leader(&self, start: u64) -> bool {
        let mut near = 0u32;
        for k in 1..=5u64 {
            let back = (k as f64 * 0.004 * self.rate) as u64;
            let idx = start.saturating_sub(back);
            if (self.hz_at(idx) - VIS_LEADER_HZ).abs() < 120.0 {
                near += 1;
            }
        }
        near >= 3
    }

    /// Total samples per scan line for `mode`, at line index `line` (only the
    /// Robot modes vary by line, and then only in which chroma they carry —
    /// the duration is the same either way).
    fn line_period_samples(&self, mode: SstvMode, line: u16) -> f64 {
        let (w, _) = mode.dimensions();
        line_segments(mode, w, line)
            .iter()
            .map(|s| match s {
                Seg::Tone { dur, .. } => *dur * self.rate,
                Seg::Scan { width, px, .. } => *width as f64 * *px * self.rate,
            })
            .sum()
    }

    /// How many earlier sync pulses sit on `mode`'s line grid behind the newest
    /// one, and the mean length of the pulses that do (the newest included).
    ///
    /// Anchored on the newest pulse and counted by line, not by consecutive
    /// pair: on a weak signal noise adds pulses of its own between the real
    /// ones, and counting consecutive gaps read every interloper as two broken
    /// gaps — so a cadence a phone locks onto was never seen. Counting each
    /// line of the grid once, and asking for several, is also what keeps a
    /// handful of noise pulses (or a header's own break and start bit) from
    /// passing for a picture.
    fn cadence_fit(&self, mode: SstvMode) -> (u32, f64) {
        let period = self.line_period_samples(mode, 0);
        let (_, sdur) = self.sync_span(mode, mode.dimensions().0, 0);
        // A pulse the wrong length for this mode is not one of its syncs.
        let fits = |run: u32| ((run as f64 - sdur).abs() / sdur) <= 0.4;
        let Some(&(anchor, arun)) = self.sync_hist.last() else { return (0, 0.0) };
        if !fits(arun) {
            return (0, 0.0);
        }
        let mut lines = [false; CADENCE_LINES as usize + 1];
        let (mut sum, mut n) = (arun as f64, 1.0);
        for &(c, run) in &self.sync_hist[..self.sync_hist.len() - 1] {
            let gap = anchor as f64 - c as f64;
            let k = (gap / period).round();
            if !fits(run)
                || k < 1.0
                || k > CADENCE_LINES as f64
                || (gap - k * period).abs() > period * 0.005
            {
                continue;
            }
            if !lines[k as usize] {
                lines[k as usize] = true;
                sum += run as f64;
                n += 1.0;
            }
        }
        (lines.iter().filter(|&&l| l).count() as u32, sum / n)
    }

    /// The mean fraction of the power on the sync tone at every place `mode`'s
    /// sync pulses would be, counting back a line at a time from the pulse
    /// centred on `centre`, over the history there is (at least three lines).
    ///
    /// The pulses the hunt saw are only how a cadence is *proposed*; this is
    /// how it is tested — on all of the lines at once, including the ones
    /// whose pulse was too weak to be seen on its own.
    fn grid_frac(&self, centre: u64, mode: SstvMode) -> f64 {
        let period = self.line_period_samples(mode, 0);
        let (_, sdur) = self.sync_span(mode, mode.dimensions().0, 0);
        let (mut sum, mut n) = (0.0, 0u32);
        for k in 0..CADENCE_LINES {
            let c = centre as f64 - k as f64 * period;
            if c - sdur < self.hist_base as f64 {
                break;
            }
            sum += self.tone_frac(c - sdur * 0.5, c + sdur * 0.5, SYNC_HZ);
            n += 1;
        }
        if n < 3 { 0.0 } else { sum / n as f64 }
    }

    /// Free-run lock: when 1200 Hz sync pulses arrive at a regular line cadence,
    /// start decoding (no VIS needed — handles tuning into a picture already in
    /// progress). With a fixed `expected` mode it locks to that; in auto it picks
    /// the mode whose line period *and* sync length best fit the cadence.
    fn try_freerun(&mut self, out: &mut Vec<SstvEvent>) {
        let is_sync = self.sync_tone.frac() > SYNC_FRAC;
        if is_sync {
            // A dip out of the band that did not last is the pulse still.
            self.sync_run += 1 + self.sync_gap;
            self.sync_gap = 0;
            return;
        }
        if self.sync_run > 0 && (self.sync_gap as f64) < 0.002 * self.rate {
            self.sync_gap += 1;
            return;
        }
        let run = self.sync_run;
        let gap = self.sync_gap;
        self.sync_run = 0;
        self.sync_gap = 0;
        // Plausible sync length across all modes: 5.5 ms (Wraase SC-2) through
        // 9 ms (Scottie/Robot) to **20 ms** (the whole PD family), with a
        // margin either side. The old ceiling was 14 ms, which excluded every
        // PD mode — so a PD picture tuned into mid-transmission could never
        // free-run lock however long it ran (issue #421).
        //
        // A window has to be mostly inside the pulse before it clears the
        // threshold, so the run it reports is short of the pulse by about
        // half a window. That is put back first, so the floor is a pulse
        // length and not a window-dependent one.
        let lag = (self.sync_tone.len() / 2) as u64;
        // The pulse ended where the gap after it began, and the window runs
        // half its own length behind the audio.
        let center = self.sample_idx.saturating_sub(lag + gap as u64 + (run / 2) as u64);
        let run = run + lag as u32;
        if (run as f64) < 0.003 * self.rate || (run as f64) > 0.028 * self.rate {
            return;
        }
        self.sync_hist.push((center, run));
        // Several lines of the slowest mode, however many noise pulses arrive
        // among them.
        let keep_from = center.saturating_sub((CADENCE_KEEP_S * self.rate) as u64);
        self.sync_hist.retain(|&(c, _)| c >= keep_from);
        if self.sync_hist.len() > 256 {
            self.sync_hist.remove(0);
        }

        // Pick the mode to lock: the fixed one, or the best auto match. A
        // mode the operator chose needs fewer lines than one guessed from the
        // cadence alone.
        let locked = match self.expected {
            Some(m) => (self.cadence_fit(m).0 >= CADENCE_FIXED
                && self.grid_frac(center, m) >= GRID_FRAC)
                .then_some(m),
            None => {
                let mut best: Option<SstvMode> = None;
                let mut best_frac = GRID_FRAC;
                for &m in &SstvMode::ALL {
                    let (hits, mean_run) = self.cadence_fit(m);
                    if hits < CADENCE_AUTO {
                        continue;
                    }
                    let (_, sdur) = self.sync_span(m, m.dimensions().0, 0);
                    if (mean_run - sdur).abs() / sdur > 0.4 {
                        continue; // sync length must also match (Scottie vs Martin)
                    }
                    let frac = self.grid_frac(center, m);
                    if frac >= best_frac {
                        best_frac = frac;
                        best = Some(m);
                    }
                }
                best
            }
        };
        if let Some(mode) = locked {
            let (soff, sdur) = self.sync_span(mode, mode.dimensions().0, 0);
            let line_start = (center as f64 - (soff + sdur * 0.5)).max(0.0) as u64;
            self.sync_hist.clear();
            self.begin_image(mode, line_start, out);
        }
    }

    fn begin_image(&mut self, mode: SstvMode, first_line_start: u64, out: &mut Vec<SstvEvent>) {
        self.mode = mode;
        self.phase = RxPhase::Image;
        self.line = 0;
        self.line_start = first_line_start;
        self.line_samples = self.line_period_samples(mode, 0) as u64;
        let (w, _) = mode.dimensions();
        self.last_cr = vec![128u8; (w / 2) as usize];
        self.last_cb = vec![128u8; (w / 2) as usize];
        // Reset free-run tracking so it re-locks cleanly for the next picture.
        self.sync_run = 0;
        self.sync_gap = 0;
        self.sync_hist.clear();
        out.push(SstvEvent::ModeDetected(mode));
    }

    // ── image line decode ──
    fn step_image(&mut self, out: &mut Vec<SstvEvent>) {
        // Decode a line once its full duration of history is available. This
        // runs on every sample, so the line length comes from the cached value
        // rather than rebuilding the segment plan (and its allocation) here.
        let line_samples = self.line_samples;
        if self.cursor() < self.line_start + line_samples + (0.02 * self.rate) as u64 {
            return;
        }

        // Re-align each line to its 1200 Hz sync pulse (corrects timing error
        // and clock slant on real off-air signals).
        let (w, h) = self.mode.dimensions();
        let start = self.realign_sync(self.line_start, self.mode, w, self.line);
        // One transmitted line is two picture rows in the PD family — see
        // `SstvMode::rows_per_line` — so this hands back a row at a time and
        // the caller sees the same stream of `Line` events either way.
        for (n, rgb) in self.decode_line(self.mode, w, self.line, start).into_iter().enumerate() {
            let y = self.line + n as u16;
            if y < h {
                out.push(SstvEvent::Line { y, rgb });
            }
        }

        self.line += self.mode.rows_per_line().max(1);
        self.line_start = start + line_samples;
        if self.line >= h {
            out.push(SstvEvent::ImageComplete);
            self.phase = RxPhase::Hunt;
            self.vis_state = VisState::reset();
        } else {
            self.line_samples = self.line_period_samples(self.mode, self.line) as u64;
        }
    }

    /// Offset (in samples) from a line's start to the centre of its 1200 Hz
    /// sync pulse, plus the pulse duration in samples.
    fn sync_span(&self, mode: SstvMode, w: u16, line: u16) -> (f64, f64) {
        let mut t = 0.0;
        for seg in line_segments(mode, w, line) {
            match seg {
                Seg::Tone { hz, dur } => {
                    let d = dur * self.rate;
                    if (hz - SYNC_HZ).abs() < 1.0 {
                        return (t, d);
                    }
                    t += d;
                }
                Seg::Scan { width, px, .. } => t += width as f64 * px * self.rate,
            }
        }
        (0.0, 0.009 * self.rate)
    }

    /// Correct the line-start sample index by locking to the line's 1200 Hz sync
    /// pulse: find the mean position of near-1200 Hz samples in a window around
    /// where the sync is expected, then back-compute the line start.
    fn realign_sync(&self, nominal: u64, mode: SstvMode, w: u16, line: u16) -> u64 {
        let (soff, sdur) = self.sync_span(mode, w, line);
        let centre_off = soff + sdur * 0.5;
        let expected = nominal as f64 + centre_off;
        // A pulse-long window slid across ±12 ms of where the sync should be,
        // and the place where most of its power is on the sync tone wins: a
        // matched filter. The old way counted samples whose frequency read
        // under 1380 Hz, and on a weak signal noise drags a 1200 Hz pulse's
        // reading up past that, so the line was left on its nominal start and
        // a picture with any clock error slanted away.
        let len = (sdur as usize).max(1);
        let win = (0.012 * self.rate) as i64;
        let first = (expected as i64 - win - len as i64 / 2).max(self.hist_base as i64) as u64;
        let last = (expected as i64 + win + len as i64 / 2).max(0) as u64;
        let w = TAU * (SYNC_HZ - 1900.0) / self.rate;
        let mut ys: Vec<(f64, f64, f64)> = Vec::new();
        for idx in first..last {
            let Some(z) = self.hist_z.get((idx - self.hist_base) as usize) else { break };
            let (sn, cs) = (-w * (idx - first) as f64).sin_cos();
            let (zr, zi) = (z.re as f64, z.im as f64);
            ys.push((zr * cs - zi * sn, zr * sn + zi * cs, zr * zr + zi * zi));
        }
        if ys.len() <= len {
            return nominal;
        }
        let (mut re, mut im, mut p) = (0.0, 0.0, 0.0);
        for y in &ys[..len] {
            re += y.0;
            im += y.1;
            p += y.2;
        }
        let mut best = (0.0f64, 0usize);
        for i in 0..=ys.len() - len {
            if i > 0 {
                let (o, n) = (ys[i - 1], ys[i + len - 1]);
                re += n.0 - o.0;
                im += n.1 - o.1;
                p += n.2 - o.2;
            }
            if p > 0.0 {
                let f = (re * re + im * im) / (len as f64 * p);
                if f > best.0 {
                    best = (f, i);
                }
            }
        }
        // Trust the correction only when a real sync pulse is present.
        if best.0 > REALIGN_FRAC {
            let centre = first as f64 + best.1 as f64 + len as f64 * 0.5;
            (centre - centre_off).max(0.0) as u64
        } else {
            nominal
        }
    }

    /// Decode one transmitted line into its picture rows: one for every mode
    /// but the PD family, two for that.
    fn decode_line(&mut self, mode: SstvMode, w: u16, line: u16, start: u64) -> Vec<Vec<u8>> {
        let mut r = vec![0u8; w as usize];
        let mut g = vec![0u8; w as usize];
        let mut b = vec![0u8; w as usize];
        let mut y = vec![0u8; w as usize];
        let mut y2 = vec![0u8; w as usize];
        let mut cr = self.last_cr.clone();
        let mut cb = self.last_cb.clone();
        // A PD line's chroma is full width, so the buffers this mode's rows
        // came in with (sized for the Robot modes' half-width chroma) are the
        // wrong shape for it.
        if mode.rows_per_line() > 1 {
            cr = vec![128u8; w as usize];
            cb = vec![128u8; w as usize];
        }

        let mut t = start as f64;
        for seg in line_segments(mode, w, line) {
            match seg {
                Seg::Tone { dur, .. } => t += dur * self.rate,
                Seg::Scan { chan, width, px } => {
                    let step = px * self.rate;
                    for x in 0..width as usize {
                        // The whole pixel window, measured as one.
                        let p0 = t + x as f64 * step;
                        let v = hz_to_value(self.hz_over(p0, p0 + step));
                        let cri = x.min(cr.len().saturating_sub(1));
                        let cbi = x.min(cb.len().saturating_sub(1));
                        match chan {
                            Chan::R => r[x] = v,
                            Chan::G => g[x] = v,
                            Chan::B => b[x] = v,
                            Chan::Y => y[x] = v,
                            Chan::Y2 => y2[x] = v,
                            Chan::Cr => cr[cri] = v,
                            Chan::Cb => cb[cbi] = v,
                        }
                    }
                    t += width as f64 * step;
                }
            }
        }

        let robot = matches!(mode, SstvMode::Robot72 | SstvMode::Robot36);
        if robot {
            // Robot 36 sends one chroma per line and the other row's is
            // carried over; PD sends both, every line, and has nothing to
            // remember.
            self.last_cr = cr.clone();
            self.last_cb = cb.clone();
        }
        let pd = mode.rows_per_line() > 1;

        // One row for most modes, two for PD — where the second row is the
        // same chroma with the second luma scan over it.
        let mut rows: Vec<Vec<u8>> = Vec::with_capacity(if pd { 2 } else { 1 });
        for luma in [&y, &y2].into_iter().take(if pd { 2 } else { 1 }) {
            let mut rgb = vec![0u8; w as usize * 3];
            for x in 0..w as usize {
                let (rr, gg, bb) = if robot {
                    let cx = (x / 2).min(cr.len() - 1);
                    yuv_to_rgb(y[x], cr[cx], cb[cx])
                } else if pd {
                    yuv_to_rgb(luma[x], cr[x], cb[x])
                } else {
                    (r[x], g[x], b[x])
                };
                rgb[x * 3] = rr;
                rgb[x * 3 + 1] = gg;
                rgb[x * 3 + 2] = bb;
            }
            rows.push(rgb);
        }
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The symbol stream against the published format, character by character.
    ///
    /// Worth spelling out rather than only round-tripping: a codec tested
    /// against nothing but its own inverse agrees with itself whatever it does,
    /// and what has to be true here is that it agrees with MMSSTV. `$2A`, then
    /// ASCII less `$20`, then `$01`, then the XOR of the characters alone.
    #[test]
    fn the_symbols_are_the_published_ones() {
        assert_eq!(
            fsk_id_symbols("OE1XYZ"),
            vec![
                0x2A, // header
                0x2F, // O
                0x25, // E
                0x11, // 1
                0x38, // X
                0x39, // Y
                0x3A, // Z
                0x01, // terminator
                0x2F ^ 0x25 ^ 0x11 ^ 0x38 ^ 0x39 ^ 0x3A,
            ]
        );
        // Lower case has no code of its own and is folded, not dropped...
        assert_eq!(fsk_id_symbols("oe1xyz"), fsk_id_symbols("OE1XYZ"));
        // ...and a character with no code at all is left out rather than
        // mangled into one that would print as noise at the far end.
        assert_eq!(fsk_id_symbols("OE1XYZ\u{00fc}"), fsk_id_symbols("OE1XYZ"));
        // Nothing to identify with is nothing to send.
        assert!(fsk_id_symbols("").is_empty());
        assert!(fsk_id_symbols("   ").is_empty());
    }

    #[test]
    fn a_stream_that_does_not_check_out_is_not_a_callsign() {
        let good = fsk_id_symbols("OE1XYZ");
        assert_eq!(fsk_id_text(&good).as_deref(), Some("OE1XYZ"));
        // One bit of one character wrong: the checksum is the whole point.
        let mut bad = good.clone();
        bad[3] ^= 1;
        assert_eq!(fsk_id_text(&bad), None);
        // No header, no terminator, nothing after the terminator: each on its
        // own is enough to refuse.
        assert_eq!(fsk_id_text(&good[1..]), None);
        assert_eq!(fsk_id_text(&good[..good.len() - 2]), None);
        assert_eq!(fsk_id_text(&good[..good.len() - 1]), None);
    }

    /// The whole of issue #287: a picture, then the callsign in tones, decoded
    /// off the audio by the receiver rather than by the encoder's own inverse.
    #[test]
    fn the_id_comes_back_off_the_air() {
        let rate = 48_000.0;
        let mode = SstvMode::Robot36;
        let (w, h) = mode.dimensions();
        let rgb = vec![128u8; w as usize * h as usize * 3];
        let mut tx = SstvTx::new(mode, &rgb, w, h, rate, 0.0).with_fsk_id("OE1XYZ");
        let mut rx = SstvRx::new(rate);
        let mut events = Vec::new();
        let mut block = vec![0.0f32; 4096];
        let mut heard = None;
        let mut guard = 0;
        while !tx.done() && guard < 40_000 {
            let n = tx.next_block(&mut block);
            rx.process(&block[..n], &mut events);
            for e in events.drain(..) {
                if let SstvEvent::FskId(id) = e {
                    heard = Some(id);
                }
            }
            guard += 1;
        }
        assert_eq!(heard.as_deref(), Some("OE1XYZ"));
    }

    /// A station with nothing to identify with transmits exactly what it used
    /// to — no leader, no tail, not one sample.
    #[test]
    fn no_callsign_adds_no_air_time() {
        let rate = 48_000.0;
        let mode = SstvMode::Robot36;
        let (w, h) = mode.dimensions();
        let rgb = vec![64u8; w as usize * h as usize * 3];
        let plain = SstvTx::new(mode, &rgb, w, h, rate, 0.0).total_samples();
        assert_eq!(SstvTx::new(mode, &rgb, w, h, rate, 0.0).with_fsk_id("").total_samples(), plain);
        // And one that has something to send costs the leader plus six bits a
        // character, which is about two and a half seconds for a callsign.
        let with = SstvTx::new(mode, &rgb, w, h, rate, 0.0).with_fsk_id("OE1XYZ").total_samples();
        let added = (with - plain) as f64 / rate;
        let want = FSKID_LEADER_S + FSKID_SYNC_S + FSKID_BIT_S * (1.0 + 6.0 * 9.0);
        assert!((added - want).abs() < 0.01, "the ID added {added:.3} s, expected {want:.3} s");
    }

    /// Issue #397: a receiver committed to a four-minute mode has to be able
    /// to let go of it.
    ///
    /// Locked on to Scottie DX and then restarted, it must be hunting again —
    /// and it must *stay* hunting on the audio that is still arriving from the
    /// transmission it abandoned, because the free-run detector reads the
    /// history buffer and the history buffer was full of that picture's sync
    /// pulses. Then a new header on the same receiver has to start a picture,
    /// which is the half that says the restart re-armed rather than merely
    /// stopped.
    #[test]
    fn a_restart_lets_go_of_the_picture_and_hunts_again() {
        let rate = 48_000.0;
        let (w, h) = SstvMode::ScottieDx.dimensions();
        let rgb = vec![96u8; w as usize * h as usize * 3];
        let mut tx = SstvTx::new(SstvMode::ScottieDx, &rgb, w, h, rate, 0.0);
        let mut rx = SstvRx::new(rate);
        let mut events = Vec::new();
        let mut block = vec![0.0f32; 4096];

        // Far enough in to be decoding lines, not merely to have seen the VIS.
        while !rx.receiving() || rx.progress() < 0.02 {
            let n = tx.next_block(&mut block);
            assert!(n > 0, "the transmission ran out before the picture started");
            rx.process(&block[..n], &mut events);
            events.clear();
        }
        assert_eq!(rx.mode(), SstvMode::ScottieDx);

        rx.restart();
        assert!(!rx.receiving(), "the picture was not let go of");
        assert_eq!(rx.progress(), 0.0);

        // Another second of the abandoned transmission: a receiver that kept
        // its history would lock straight back on to the cadence it is hearing.
        for _ in 0..12 {
            let n = tx.next_block(&mut block);
            rx.process(&block[..n], &mut events);
            events.clear();
        }
        assert!(!rx.receiving(), "it re-locked on the transmission it was told to abandon");

        // ...and the next station's header still starts a picture.
        let (w2, h2) = SstvMode::Robot36.dimensions();
        let rgb2 = vec![32u8; w2 as usize * h2 as usize * 3];
        let mut next = SstvTx::new(SstvMode::Robot36, &rgb2, w2, h2, rate, 0.0);
        let mut detected = None;
        let mut guard = 0;
        while detected.is_none() && !next.done() && guard < 2_000 {
            let n = next.next_block(&mut block);
            rx.process(&block[..n], &mut events);
            for e in events.drain(..) {
                if let SstvEvent::ModeDetected(m) = e {
                    detected = Some(m);
                }
            }
            guard += 1;
        }
        assert_eq!(detected, Some(SstvMode::Robot36), "the restarted receiver heard nothing");
    }

    /// End-to-end: encode a small gradient, decode it back, and check the VIS
    /// mode was recovered and the image roughly matches.
    #[test]
    fn scottie1_loopback_recovers_mode() {
        let rate = 48_000.0;
        let mode = SstvMode::Scottie1;
        let (w, h) = mode.dimensions();
        // Simple vertical gradient so a rough decode is easy to sanity-check.
        let mut rgb = vec![0u8; w as usize * h as usize * 3];
        for yy in 0..h as usize {
            for xx in 0..w as usize {
                let i = (yy * w as usize + xx) * 3;
                let v = (xx * 255 / w as usize) as u8;
                rgb[i] = v;
                rgb[i + 1] = v;
                rgb[i + 2] = v;
            }
        }
        let mut tx = SstvTx::new(mode, &rgb, w, h, rate, 0.0);
        let mut rx = SstvRx::new(rate);
        let mut events = Vec::new();
        let mut block = vec![0.0f32; 4096];
        let mut detected = None;
        let mut lines = 0;
        let mut guard = 0;
        while !tx.done() && guard < 20_000 {
            let n = tx.next_block(&mut block);
            rx.process(&block[..n], &mut events);
            for e in events.drain(..) {
                match e {
                    SstvEvent::ModeDetected(m) => detected = Some(m),
                    SstvEvent::Line { .. } => lines += 1,
                    SstvEvent::ImageComplete
                    | SstvEvent::FskId(_)
                    | SstvEvent::UnsupportedMode { .. } => {}
                }
            }
            guard += 1;
        }
        // Flush any tail.
        rx.process(&[0.0; 48_000], &mut events);
        for e in events.drain(..) {
            if let SstvEvent::ModeDetected(m) = e {
                detected = Some(m);
            } else if let SstvEvent::Line { .. } = e {
                lines += 1;
            }
        }
        assert_eq!(detected, Some(mode), "VIS mode should be recovered");
        assert!(lines > (h as usize) / 2, "should decode most lines, got {lines}");
    }

    /// Every mode, encoder into decoder: the VIS is read back, the picture
    /// comes out the right height, and the pixels are roughly what went in.
    ///
    /// One test over `SstvMode::ALL` rather than one per mode, because what
    /// has to hold is a property of the table: a mode added to it with a pixel
    /// time transcribed wrongly still produces a picture, just a sheared one,
    /// and the only thing that catches that is decoding it back and looking at
    /// where the colours landed. It is also what proves the PD family's
    /// two-rows-per-line plumbing, which nothing else in the file exercises.
    #[test]
    fn every_mode_round_trips_through_its_own_decoder() {
        let rate = 48_000.0;
        for mode in [
            SstvMode::WraaseSc2_180,
            SstvMode::Martin1,
            SstvMode::Robot36,
            SstvMode::Pd120,
            SstvMode::Scottie1,
        ] {
            let (w, h) = mode.dimensions();
            // Three vertical bars — red, green, blue — so a decode that has
            // the channels or the timing wrong cannot pass by accident.
            let mut rgb = vec![0u8; w as usize * h as usize * 3];
            for yy in 0..h as usize {
                for xx in 0..w as usize {
                    let i = (yy * w as usize + xx) * 3;
                    let band = xx * 3 / w as usize;
                    rgb[i + band.min(2)] = 220;
                }
            }
            let mut tx = SstvTx::new(mode, &rgb, w, h, rate, 0.0);
            let mut rx = SstvRx::new(rate);
            let mut events = Vec::new();
            let mut block = vec![0.0f32; 8192];
            let mut detected = None;
            let mut got = vec![0u8; w as usize * h as usize * 3];
            let mut lines = 0usize;
            let mut complete = false;
            let mut guard = 0;
            while !tx.done() && guard < 400_000 {
                let n = tx.next_block(&mut block);
                rx.process(&block[..n], &mut events);
                for e in events.drain(..) {
                    match e {
                        SstvEvent::ModeDetected(m) => detected = Some(m),
                        SstvEvent::Line { y, rgb: row } => {
                            lines += 1;
                            let at = y as usize * w as usize * 3;
                            if at + row.len() <= got.len() {
                                got[at..at + row.len()].copy_from_slice(&row);
                            }
                        }
                        SstvEvent::ImageComplete => complete = true,
                        _ => {}
                    }
                }
                guard += 1;
            }
            // Trailing silence, as a real receiver always has: a line is only
            // decoded once 20 ms of audio past its end has arrived, so without
            // this the *last* line of every picture is never emitted and the
            // image never completes. Both are what the engine's save and the
            // panel's "complete" state rest on, so both are asserted.
            rx.process(&vec![0.0f32; 48_000], &mut events);
            for e in events.drain(..) {
                match e {
                    SstvEvent::Line { y, rgb: row } => {
                        lines += 1;
                        let at = y as usize * w as usize * 3;
                        if at + row.len() <= got.len() {
                            got[at..at + row.len()].copy_from_slice(&row);
                        }
                    }
                    SstvEvent::ImageComplete => complete = true,
                    _ => {}
                }
            }
            assert_eq!(detected, Some(mode), "{} VIS not recovered", mode.label());
            assert_eq!(lines, h as usize, "{}: decoded {lines} of {h} rows", mode.label());
            assert!(complete, "{}: ImageComplete never fired", mode.label());
            // Sample the middle of each colour bar, a third of the way down,
            // and check the right channel is the dominant one.
            let yy = h as usize / 3;
            for (band, chan) in [(0usize, 0usize), (1, 1), (2, 2)] {
                let xx = (band * 2 + 1) * w as usize / 6;
                let i = (yy * w as usize + xx) * 3;
                let px = [got[i], got[i + 1], got[i + 2]];
                let others = (0..3).filter(|&c| c != chan).map(|c| px[c]).max().unwrap();
                assert!(
                    px[chan] > 100 && px[chan] as i32 > others as i32 + 40,
                    "{}: bar {band} decoded as {px:?}, expected channel {chan} to dominate",
                    mode.label()
                );
            }
        }
    }

    /// The published line times, recomputed from the segment plan. A pixel
    /// time transcribed with a digit out still makes a picture; it is the
    /// *total* that gives it away, so that is what is pinned.
    #[test]
    fn the_line_times_are_the_published_ones() {
        let rx = SstvRx::new(1_000_000.0); // µs per sample: read the plan directly
        for (mode, ms) in [
            (SstvMode::Scottie1, 428.22),
            (SstvMode::Scottie2, 277.692),
            (SstvMode::ScottieDx, 1_050.3),
            (SstvMode::Martin1, 446.446),
            (SstvMode::Martin2, 226.798),
            (SstvMode::Robot72, 300.0),
            (SstvMode::Robot36, 150.0),
            (SstvMode::WraaseSc2_180, 711.0225),
            (SstvMode::WraaseSc2_120, 475.53),
            (SstvMode::Pd50, 388.16),
            (SstvMode::Pd90, 703.04),
            (SstvMode::Pd120, 508.48),
            (SstvMode::Pd160, 804.416),
            (SstvMode::Pd180, 754.24),
            (SstvMode::Pd240, 1_000.0),
            (SstvMode::Pd290, 937.28),
        ] {
            let got = rx.line_period_samples(mode, 0) / 1000.0;
            assert!(
                (got - ms).abs() < 0.05,
                "{}: line is {got:.4} ms, published {ms} ms",
                mode.label()
            );
        }
    }

    /// The PD family is the only one that carries two picture rows per sync.
    #[test]
    fn only_the_pd_modes_carry_two_rows_a_line() {
        for m in SstvMode::ALL {
            let want = matches!(
                m,
                SstvMode::Pd50
                    | SstvMode::Pd90
                    | SstvMode::Pd120
                    | SstvMode::Pd160
                    | SstvMode::Pd180
                    | SstvMode::Pd240
                    | SstvMode::Pd290
            );
            assert_eq!(m.rows_per_line() == 2, want, "{}", m.label());
        }
    }

    /// A header for a mode this build does not have is reported, not
    /// swallowed — the whole of issue #421's "the decoder isn't starting
    /// reception" with a perfect signal on the waterfall.
    #[test]
    fn an_unimplemented_mode_says_so_instead_of_going_quiet() {
        let rate = 48_000.0;
        // Pasokon P3's VIS, sent by hand: leader, break, leader, start bit,
        // seven data bits LSB first, parity, stop.
        let code = 0x71u8;
        let mut audio: Vec<f32> = Vec::new();
        let mut phase = 0.0f64;
        let mut tone = |hz: f64, dur: f64, audio: &mut Vec<f32>| {
            for _ in 0..(dur * rate) as usize {
                phase += TAU * hz / rate;
                audio.push((phase.sin() as f32) * 0.5);
            }
        };
        tone(1900.0, 0.300, &mut audio);
        tone(1200.0, 0.010, &mut audio);
        tone(1900.0, 0.300, &mut audio);
        tone(1200.0, 0.030, &mut audio);
        let mut parity = 0u8;
        for bit in 0..7 {
            let one = (code >> bit) & 1 == 1;
            parity ^= one as u8;
            tone(if one { 1100.0 } else { 1300.0 }, 0.030, &mut audio);
        }
        tone(if parity == 1 { 1100.0 } else { 1300.0 }, 0.030, &mut audio);
        tone(1200.0, 0.030, &mut audio);
        tone(1500.0, 0.500, &mut audio);

        let mut rx = SstvRx::new(rate);
        let mut events = Vec::new();
        for chunk in audio.chunks(4096) {
            rx.process(chunk, &mut events);
        }
        let reported: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                SstvEvent::UnsupportedMode { code, name } => Some((*code, *name)),
                _ => None,
            })
            .collect();
        // Exactly one report: the break pulse in the middle of the header is
        // also a rising edge into 1200 Hz after a leader, and must not be
        // mistaken for a second header.
        assert_eq!(reported, vec![(0x71, Some("Pasokon P3"))]);
        assert!(
            !events.iter().any(|e| matches!(e, SstvEvent::ModeDetected(_))),
            "nothing may be decoded for a mode we do not have"
        );
    }

    #[test]
    fn vis_codes_roundtrip() {
        for m in SstvMode::ALL {
            assert_eq!(SstvMode::from_vis(m.vis_code()), Some(m));
        }
    }

    #[test]
    fn tx_ppm_scales_duration() {
        let rate = 48_000.0;
        let mode = SstvMode::Martin1;
        let (w, h) = mode.dimensions();
        let rgb = vec![128u8; w as usize * h as usize * 3];
        let base = SstvTx::new(mode, &rgb, w, h, rate, 0.0).total_samples() as f64;
        // +10 000 ppm = +1% longer transmission.
        let trimmed = SstvTx::new(mode, &rgb, w, h, rate, 10_000.0).total_samples() as f64;
        assert!((trimmed / base - 1.01).abs() < 0.0005, "ratio {}", trimmed / base);
    }

    /// Free-run: feed the RX the picture audio *after* the VIS header (as if we
    /// tuned in mid-transmission). With the mode pre-selected it should lock onto
    /// the sync cadence and decode.
    #[test]
    fn freerun_decodes_without_vis() {
        let rate = 48_000.0;
        let mode = SstvMode::Scottie1;
        let (w, h) = mode.dimensions();
        let mut rgb = vec![0u8; w as usize * h as usize * 3];
        for yy in 0..h as usize {
            for xx in 0..w as usize {
                let i = (yy * w as usize + xx) * 3;
                rgb[i] = (xx * 255 / w as usize) as u8;
            }
        }
        // Render the whole transmission to a buffer.
        let mut tx = SstvTx::new(mode, &rgb, w, h, rate, 0.0);
        let mut audio = Vec::new();
        let mut block = vec![0.0f32; 4096];
        let mut guard = 0;
        while !tx.done() && guard < 20_000 {
            let n = tx.next_block(&mut block);
            audio.extend_from_slice(&block[..n]);
            guard += 1;
        }
        // Skip past the VIS (~1.1 s) so only image data is fed → forces free-run.
        // Use auto (`None`): the RX must identify the mode from the sync cadence.
        let skip = (rate * 1.1) as usize;
        let mut rx = SstvRx::new(rate);
        rx.set_expected(None);
        let mut events = Vec::new();
        let mut detected = None;
        let mut lines = 0;
        for chunk in audio[skip.min(audio.len())..].chunks(4096) {
            rx.process(chunk, &mut events);
            for e in events.drain(..) {
                match e {
                    SstvEvent::ModeDetected(m) => detected = Some(m),
                    SstvEvent::Line { .. } => lines += 1,
                    SstvEvent::ImageComplete
                    | SstvEvent::FskId(_)
                    | SstvEvent::UnsupportedMode { .. } => {}
                }
            }
        }
        assert_eq!(detected, Some(mode), "free-run should lock the selected mode");
        assert!(lines > 40, "free-run should decode many lines, got {lines}");
    }

    // ── weak signals ──
    //
    // A report from the air: on 80 m a Wraase SC2-180 picture that a phone
    // held to the speaker decoded — noisy, but a picture — sat here on
    // "audio present · no SSTV header yet", and RESTART RX did not help. These
    // put the same thing on the bench with white noise, measured in the 2500 Hz
    // every weak-signal figure is quoted in. The receiver they replaced lost
    // the header from about +10 dB and the free-run lock from about +2 dB.

    /// White noise for the tests below, the same every run.
    struct Noise(u64);

    impl Noise {
        fn uniform(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }

        fn gauss(&mut self) -> f64 {
            let u1 = self.uniform().max(1e-300);
            let u2 = self.uniform();
            (-2.0 * u1.ln()).sqrt() * (TAU * u2).cos()
        }
    }

    /// A whole transmission — three colour bars, or a flat grey — with a
    /// second of silence either side.
    fn on_air(mode: SstvMode, rate: f64, grey: bool) -> Vec<f32> {
        let (w, h) = mode.dimensions();
        let mut rgb = vec![if grey { 128u8 } else { 0 }; w as usize * h as usize * 3];
        if !grey {
            for yy in 0..h as usize {
                for xx in 0..w as usize {
                    rgb[(yy * w as usize + xx) * 3 + (xx * 3 / w as usize).min(2)] = 220;
                }
            }
        }
        let mut tx = SstvTx::new(mode, &rgb, w, h, rate, 0.0);
        let mut audio = vec![0.0f32; rate as usize];
        let mut block = vec![0.0f32; 8192];
        while !tx.done() {
            let n = tx.next_block(&mut block);
            audio.extend_from_slice(&block[..n]);
        }
        audio.extend(std::iter::repeat_n(0.0, rate as usize));
        audio
    }

    /// `audio` with noise added at `snr_db` in 2500 Hz.
    fn with_noise(audio: &[f32], snr_db: f64, rate: f64, seed: u64) -> Vec<f32> {
        let on: Vec<f64> = audio.iter().filter(|x| **x != 0.0).map(|x| *x as f64).collect();
        let signal = on.iter().map(|x| x * x).sum::<f64>() / on.len().max(1) as f64;
        let in_2500 = signal / 10f64.powf(snr_db / 10.0);
        let sigma = (in_2500 * (rate / 2.0) / 2500.0).sqrt();
        let mut noise = Noise(seed);
        audio.iter().map(|x| ((*x as f64 + sigma * noise.gauss()) * 0.2) as f32).collect()
    }

    /// The first mode the receiver announces over `audio`, if any.
    fn first_mode(audio: &[f32], rate: f64) -> Option<SstvMode> {
        let mut rx = SstvRx::new(rate);
        let mut events = Vec::new();
        for chunk in audio.chunks(4096) {
            rx.process(chunk, &mut events);
            for e in events.drain(..) {
                if let SstvEvent::ModeDetected(m) = e {
                    return Some(m);
                }
            }
        }
        None
    }

    #[test]
    fn a_weak_header_is_still_read() {
        let rate = 48_000.0;
        for mode in [SstvMode::WraaseSc2_180, SstvMode::Martin1, SstvMode::Scottie1] {
            let audio = on_air(mode, rate, false);
            // The silence, the header, and a little picture: the header alone
            // has to be enough, before any free-run lock could have a cadence.
            let header = &audio[..(rate * 2.5) as usize];
            for seed in 1..=3u64 {
                let noisy = with_noise(header, -3.0, rate, seed * 7919);
                assert_eq!(
                    first_mode(&noisy, rate).map(|m| m.label()),
                    Some(mode.label()),
                    "{} header at -3 dB (seed {seed}) was not read",
                    mode.label()
                );
            }
        }
    }

    #[test]
    fn a_weak_picture_is_locked_without_its_header() {
        let rate = 48_000.0;
        let mode = SstvMode::WraaseSc2_180;
        let audio = on_air(mode, rate, false);
        // Tuned in after the header, with sixteen lines' worth to lock on.
        let from = (rate * 3.2) as usize;
        let picture = &audio[from..from + (rate * 12.0) as usize];
        for seed in 1..=4u64 {
            let noisy = with_noise(picture, -3.0, rate, seed * 104_729);
            assert_eq!(
                first_mode(&noisy, rate).map(|m| m.label()),
                Some(mode.label()),
                "SC2-180 picture at -3 dB (seed {seed}) never locked"
            );
        }
    }

    /// Mid-grey is the leader's own 1900 Hz, so a grey picture reads to the
    /// header hunt as one long leader. It must not keep the free-run lock off
    /// the sync pulses running through it.
    #[test]
    fn a_grey_picture_still_locks_without_its_header() {
        let rate = 48_000.0;
        let mode = SstvMode::Martin1;
        let audio = on_air(mode, rate, true);
        let from = (rate * 3.0) as usize;
        let noisy = with_noise(&audio[from..from + (rate * 6.0) as usize], 0.0, rate, 99);
        assert_eq!(first_mode(&noisy, rate).map(|m| m.label()), Some(mode.label()));
    }

    /// The other side of all of the above: noise alone is not a picture.
    #[test]
    fn noise_alone_starts_no_picture() {
        let rate = 48_000.0;
        let mut noise = Noise(12_345);
        let hiss: Vec<f32> =
            (0..(rate * 300.0) as usize).map(|_| (0.1 * noise.gauss()) as f32).collect();
        let mut rx = SstvRx::new(rate);
        let mut events = Vec::new();
        for chunk in hiss.chunks(4096) {
            rx.process(chunk, &mut events);
            for e in events.drain(..) {
                if let SstvEvent::ModeDetected(m) = e {
                    panic!("300 s of noise started a {} picture", m.label());
                }
            }
        }
    }

    // ── Sensitivity ──
    //
    // What the receiver makes of a picture in white noise, as a function of
    // signal-to-noise ratio. The sweep below is the yardstick every change to
    // the demodulator is measured against: a change that does not move these
    // numbers is not a sensitivity improvement, however reasonable it looks.

    /// The bandwidth the signal-to-noise ratios here are quoted in: the SSB
    /// voice channel SSTV is received through.
    const SNR_BANDWIDTH_HZ: f64 = 3000.0;

    /// The most rows a free-run lock may number wrongly: the pulses it takes
    /// to be sure of the cadence, and the line it came in partway through.
    const FREERUN_MAX_ROWS_LATE: usize = 16;

    /// What one reception in noise came to.
    struct Reception {
        /// The mode the receiver locked on to, if any.
        mode: Option<SstvMode>,
        /// Whether it locked before the picture began — off the VIS, that is,
        /// rather than off the picture's sync pulses once the VIS was missed.
        /// A free-run lock rescues the picture but not its first lines, and on
        /// Robot 36 not always its colours.
        by_vis: bool,
        /// Picture rows delivered.
        rows: usize,
        /// Mean absolute error of the delivered rows against what was sent,
        /// in levels (0–255) per channel.
        err: f64,
    }

    /// A picture with something in every channel and at every scale: a ramp
    /// across in red, a ramp down in green, and blocks in blue — so a decoder
    /// that swaps channels, loses the timing or smears the detail cannot pass
    /// by accident, and the colour-difference modes get edges in chroma.
    fn test_card(w: usize, h: usize) -> Vec<u8> {
        let mut rgb = vec![0u8; w * h * 3];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 3;
                rgb[i] = (x * 255 / w.max(1)) as u8;
                rgb[i + 1] = (y * 255 / h.max(1)) as u8;
                rgb[i + 2] = if (x / 16 + y / 16) % 2 == 0 { 40 } else { 215 };
            }
        }
        rgb
    }

    /// Send `mode` through white Gaussian noise at `snr_db` (in
    /// [`SNR_BANDWIDTH_HZ`]) and see what comes out. With `header` the whole
    /// transmission is sent and the receiver has to find the VIS; without it
    /// the header is cut off and it has to lock on the sync cadence alone.
    /// Noise runs throughout, including half a second before the signal and
    /// two seconds after it, so the hunt sees noise before it sees a picture.
    fn receive_in_noise(mode: SstvMode, snr_db: f64, seed: u64, header: bool) -> Reception {
        let rate = 48_000.0;
        let (w, h) = mode.dimensions();
        let (wu, hu) = (w as usize, h as usize);
        let sent = test_card(wu, hu);
        // The transmitter's tone is 0.5 peak, so 0.125 of power; white noise
        // of variance σ² puts σ²·B/(rate/2) of it in B.
        let noise_power = 0.125 / 10f64.powf(snr_db / 10.0);
        let sigma = (noise_power * (rate / 2.0) / SNR_BANDWIDTH_HZ).sqrt();

        let mut tx = SstvTx::new(mode, &sent, w, h, rate, 0.0);
        let mut audio = Vec::new();
        let mut block = vec![0.0f32; 8192];
        while !tx.done() {
            let n = tx.next_block(&mut block);
            audio.extend_from_slice(&block[..n]);
        }
        // The calibration header and VIS are 0.94 s; past 1.1 s is picture.
        let skip = if header { 0 } else { (1.1 * rate) as usize };
        let lead = (0.5 * rate) as usize;
        let tail = (2.0 * rate) as usize;
        let mut rng = Noise(seed | 1);
        let air: Vec<f32> = std::iter::repeat_n(0.0, lead)
            .chain(audio[skip.min(audio.len())..].iter().copied())
            .chain(std::iter::repeat_n(0.0, tail))
            .map(|s| (s as f64 + sigma * rng.gauss()) as f32)
            .collect();

        let mut rx = SstvRx::new(rate);
        rx.set_expected(None);
        let mut got = vec![0u8; sent.len()];
        let mut have = vec![false; hu];
        let mut locked = None;
        let mut by_vis = false;
        // The VIS is decoded before its stop bit, 30 ms ahead of the picture;
        // the quickest free-run lock takes two whole lines.
        let picture = lead as f64 + if header { 0.94 * rate } else { 0.0 };
        let mut fed = 0usize;
        let mut events = Vec::new();
        // Fed in small blocks so a lock is timed to within one of them.
        for chunk in air.chunks(512) {
            rx.process(chunk, &mut events);
            fed += chunk.len();
            for e in events.drain(..) {
                match e {
                    // The first lock is the one that counts: a receiver that
                    // locks on noise first has missed the picture.
                    SstvEvent::ModeDetected(m) if locked.is_none() => {
                        locked = Some(m);
                        by_vis = header && (fed as f64) < picture + 0.05 * rate;
                    }
                    SstvEvent::Line { y, rgb } if locked == Some(mode) => {
                        let at = y as usize * wu * 3;
                        if at + rgb.len() <= got.len() {
                            got[at..at + rgb.len()].copy_from_slice(&rgb);
                            have[y as usize] = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        let rows = have.iter().filter(|&&b| b).count();
        // Error of the delivered rows against the rows sent `off` further
        // down. A receiver that locks on the sync cadence cannot know which
        // line it came in on, and numbers the first one it decodes row 0 — so
        // without the header the picture is compared where it fits best. With
        // it the rows are where they are.
        let err_at = |off: usize| {
            let (mut total, mut n) = (0u64, 0u64);
            for y in (0..hu.saturating_sub(off)).filter(|&y| have[y]) {
                let (g, s) = (&got[y * wu * 3..][..wu * 3], &sent[(y + off) * wu * 3..][..wu * 3]);
                total += g.iter().zip(s).map(|(&a, &b)| a.abs_diff(b) as u64).sum::<u64>();
                n += (wu * 3) as u64;
            }
            if n == 0 { f64::NAN } else { total as f64 / n as f64 }
        };
        let err = if header {
            err_at(0)
        } else {
            (0..=FREERUN_MAX_ROWS_LATE)
                .map(err_at)
                .filter(|e| e.is_finite())
                .fold(f64::NAN, f64::min)
        };
        Reception { mode: locked, by_vis, rows, err }
    }

    /// The harness itself, on a signal far above the noise: every row comes
    /// back, close to what was sent, by either route in. (Close, not exact:
    /// Robot 36 halves its chroma, and the blocks' colour edges cost it about
    /// 8 levels with no noise at all.) If this fails the
    /// sweep below is measuring the harness, not the receiver.
    #[test]
    fn a_strong_signal_comes_through_the_noise_harness_intact() {
        let mode = SstvMode::Robot36;
        let (_, h) = mode.dimensions();
        for header in [true, false] {
            let r = receive_in_noise(mode, 60.0, 1, header);
            assert_eq!(r.mode, Some(mode), "header {header}: locked {:?}", r.mode);
            assert_eq!(r.by_vis, header, "header {header}: locked off the VIS {}", r.by_vis);
            assert!(r.rows > h as usize * 9 / 10, "header {header}: {} rows", r.rows);
            assert!(r.err < 12.0, "header {header}: error {:.1}", r.err);
        }
    }

    /// One signal-to-noise ratio of a sweep, and every reception at it.
    type SweepPoint = (f64, Vec<Reception>);

    /// The sensitivity sweep: for a few representative modes, by each route
    /// in, the share of receptions that locked on the right mode, the rows
    /// they delivered and how far those rows were from what was sent, from
    /// well below the point where anything decodes to well above it.
    ///
    /// Slow, so not part of the ordinary run:
    ///
    /// ```text
    /// cargo test --release -p sdroxide-dsp sstv::tests::sensitivity_sweep -- --ignored --nocapture
    /// ```
    ///
    /// `SSTV_SWEEP_MODES` (labels, comma-separated) and `SSTV_SWEEP_SEEDS`
    /// narrow or widen it.
    #[test]
    #[ignore = "slow: a sensitivity measurement, run by hand"]
    fn sensitivity_sweep() {
        let modes: Vec<SstvMode> = match std::env::var("SSTV_SWEEP_MODES") {
            Ok(list) => SstvMode::ALL
                .into_iter()
                .filter(|m| list.split(',').any(|l| l.trim().eq_ignore_ascii_case(m.label())))
                .collect(),
            Err(_) => {
                vec![SstvMode::Robot36, SstvMode::Martin1, SstvMode::Scottie1, SstvMode::Pd120]
            }
        };
        let seeds: u64 =
            std::env::var("SSTV_SWEEP_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(4);
        let snrs: Vec<f64> = (0..=20).map(|k| k as f64 * 2.0).collect();

        let jobs: Vec<(SstvMode, bool)> =
            modes.iter().flat_map(|&m| [(m, true), (m, false)]).collect();
        let results: Vec<(SstvMode, bool, Vec<SweepPoint>)> = std::thread::scope(|s| {
            let handles: Vec<_> = jobs
                .iter()
                .map(|&(mode, header)| {
                    let snrs = &snrs;
                    s.spawn(move || {
                        let rows = snrs
                            .iter()
                            .map(|&snr| {
                                let runs = (0..seeds)
                                    .map(|k| receive_in_noise(mode, snr, 0x5EED + 7919 * k, header))
                                    .collect();
                                (snr, runs)
                            })
                            .collect();
                        (mode, header, rows)
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        for (mode, header, rows) in results {
            let (_, h) = mode.dimensions();
            let route = if header { "VIS" } else { "free-run" };
            println!("\n{} via {route}  (SNR in {SNR_BANDWIDTH_HZ} Hz)", mode.label());
            println!("  SNR dB   locked   by VIS   rows %   error");
            for (snr, runs) in rows {
                let n = runs.len() as f64;
                let locked = runs.iter().filter(|r| r.mode == Some(mode)).count();
                let by_vis = runs.iter().filter(|r| r.mode == Some(mode) && r.by_vis).count();
                let rows_pct =
                    runs.iter().map(|r| r.rows as f64).sum::<f64>() / n / h as f64 * 100.0;
                let errs: Vec<f64> = runs.iter().map(|r| r.err).filter(|e| e.is_finite()).collect();
                let err = if errs.is_empty() {
                    "    -".to_string()
                } else {
                    format!("{:5.1}", errs.iter().sum::<f64>() / errs.len() as f64)
                };
                let vis =
                    if header { format!("{by_vis:>2}/{:<2}", runs.len()) } else { "-".into() };
                println!(
                    "  {snr:6.1}   {locked:>2}/{:<2}    {vis:>6}   {rows_pct:5.1}   {err}",
                    runs.len()
                );
            }
        }
    }
}
