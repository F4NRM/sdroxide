//! Lists the data services a DAB capture's FIC signals (FIG 0/2 with 32-bit
//! ids, 0/3, 0/8, 0/13, 1/5), so it can be seen whether an ensemble carries
//! SPI before any decoder is written for it.
//!
//! `cargo run --release -p sdroxide-dab --example dab_fig_probe -- FILE.wav OFFSET_HZ SECONDS`
//! FILE is a cf32 IQ WAV at 2.5 Msps; OFFSET_HZ is where the block sits
//! relative to the capture's centre.
use std::collections::BTreeSet;
use std::io::{Read, Seek, SeekFrom};

use num_complex::Complex32;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let path = &a[1];
    let offset: f64 = a.get(2).map(|s| s.parse().unwrap()).unwrap_or(0.0);
    let secs: f64 = a.get(3).map(|s| s.parse().unwrap()).unwrap_or(20.0);
    let rate = 2_500_000.0;
    let mut f = std::fs::File::open(path).unwrap();
    f.seek(SeekFrom::Start(252)).unwrap();
    let mut rs = sdroxide_dsp::ComplexResampler::new(rate, 2_048_000.0).unwrap();
    let mut ofdm = dabradio::ofdm::processor::OfdmProcessor::new();
    let mut seen = BTreeSet::new();
    let mut buf = vec![0u8; 8 * 65536];
    let mut n: u64 = 0;
    let step = -2.0 * std::f64::consts::PI * offset / rate;
    let mut out = Vec::new();
    while (n as f64) < secs * rate {
        if f.read_exact(&mut buf).is_err() {
            break;
        }
        let iq: Vec<Complex32> = buf
            .chunks_exact(8)
            .map(|b| {
                let z = Complex32::new(
                    f32::from_le_bytes(b[0..4].try_into().unwrap()),
                    f32::from_le_bytes(b[4..8].try_into().unwrap()),
                );
                let ph = step * n as f64;
                n += 1;
                z * Complex32::new(ph.cos() as f32, ph.sin() as f32)
            })
            .collect();
        out.clear();
        rs.push(&iq, &mut out);
        for frame in ofdm.process(&out) {
            let soft = dabradio::ofdm::decoder::dqpsk_decode(&frame.symbols);
            if soft.len() < 3 {
                continue;
            }
            let fibs = dabradio::fic::handler::process_fic(&soft[..3]);
            ofdm.set_fic_decode_ratio(((fibs.len() as f32 / 12.0) * 100.0).min(100.0) as u8);
            for fib in fibs {
                for line in figs(&fib) {
                    if seen.insert(line.clone()) {
                        println!("{line}");
                    }
                }
            }
        }
    }
}

fn figs(fib: &[u8; 32]) -> Vec<String> {
    let d = &fib[..30];
    let mut v = Vec::new();
    let mut p = 0;
    while p < 30 {
        let t = d[p] >> 5;
        let l = (d[p] & 31) as usize;
        if t == 7 || l == 0 || p + 1 + l > 30 {
            break;
        }
        let g = &d[p + 1..p + 1 + l];
        p += 1 + l;
        if t == 0 {
            let pd = (g[0] >> 5) & 1;
            let oe = (g[0] >> 6) & 1;
            let ext = g[0] & 31;
            match ext {
                2 if pd == 1 => v.push(format!("0/2 data: {:02X?}", &g[1..])),
                2 => {
                    // audio services: show components that are not plain audio
                    let b = &g[1..];
                    let mut i = 0;
                    while i + 3 <= b.len() {
                        let sid = u16::from_be_bytes([b[i], b[i + 1]]);
                        let nc = (b[i + 2] & 15) as usize;
                        let comps = &b[(i + 3).min(b.len())..(i + 3 + 2 * nc).min(b.len())];
                        v.push(format!("0/2 audio SId {sid:04X} comps {comps:02X?}"));
                        i += 3 + 2 * nc;
                    }
                }
                3 => v.push(format!("0/3 packet: {:02X?}", &g[1..])),
                8 => v.push(format!("0/8 (pd {pd}): {:02X?}", &g[1..])),
                13 => v.push(format!("0/13 (pd {pd} oe {oe}): {:02X?}", &g[1..])),
                _ => {}
            }
        } else if t == 1 {
            let ext = g[0] & 7;
            if ext == 5 || ext == 4 {
                let lab = String::from_utf8_lossy(&g[if ext == 5 { 5 } else { 2 }..]).to_string();
                v.push(format!("1/{ext}: {:02X?} {lab:?}", &g[1..5.min(g.len())]));
            }
        }
    }
    v
}
