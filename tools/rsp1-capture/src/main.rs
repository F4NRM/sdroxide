//! Throwaway SoapySDR capture: stream CF32 interleaved (f32 I, f32 Q) to a file.
//!
//! Usage: rspcap <hz> <rate> <seconds> <out.bin> [gain]
use num_complex::Complex32;
use soapysdr::{Device, Direction};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let hz: f64 = a[1].parse().expect("hz");
    let rate: f64 = a[2].parse().expect("rate");
    let secs: f64 = a[3].parse().expect("seconds");
    let out = &a[4];
    let gain: f64 = a.get(5).and_then(|g| g.parse().ok()).unwrap_or(40.0);

    let dev = Device::new("driver=sdrplay").expect("open the radio");
    dev.set_sample_rate(Direction::Rx, 0, rate).expect("sample rate");
    dev.set_frequency(Direction::Rx, 0, hz, ()).expect("frequency");
    if dev.has_gain_mode(Direction::Rx, 0).unwrap_or(false) {
        let _ = dev.set_gain_mode(Direction::Rx, 0, false);
    }
    dev.set_gain(Direction::Rx, 0, gain).expect("gain");

    // **What the radio gave us, not what we asked for.**
    //
    // `set_sample_rate` can be refused — "invalid sample rate" on an RSP1 at
    // 48 kHz, while 2.048 Msps is accepted — and the radio then keeps its own
    // rate. A capture tool that carries on with the number it asked for writes
    // a file whose *length* is right and whose *frequency axis* is wrong by the
    // ratio of the two, which is the worst kind of wrong: nothing about the
    // file gives it away.
    let got_rate = dev.sample_rate(Direction::Rx, 0).expect("read the sample rate back");
    let got_freq = dev.frequency(Direction::Rx, 0).expect("read the frequency back");
    if (got_rate - rate).abs() > rate * 1e-6 {
        eprintln!(
            "WARNING: asked for {rate:.0} S/s, the radio is running {got_rate:.0} S/s.\n\
             Capturing at what it is actually doing. Every frequency in {out} will be\n\
             wrong by {ratio:.1}x if the requested rate is used to read it.",
            out = a[4],
            ratio = rate / got_rate,
        );
    }
    if (got_freq - hz).abs() > 1.0 {
        eprintln!("WARNING: asked for {hz:.1} Hz, the radio is tuned to {got_freq:.1} Hz");
    }

    let mut stream = dev.rx_stream::<Complex32>(&[0]).expect("stream");
    stream.activate(None).expect("activate");
    // The rate we are *actually* at, so the file holds the seconds asked for.
    let want = (got_rate * secs) as usize;
    let mut samples: Vec<Complex32> = Vec::with_capacity(want);
    let mut chunk = vec![Complex32::new(0.0f32, 0.0f32); 8192];
    while samples.len() < want {
        let mut bufs: Vec<&mut [Complex32]> = vec![&mut chunk];
        let n = stream.read(&mut bufs, 5_000_000).expect("read");
        if n == 0 {
            break;
        }
        samples.extend_from_slice(&chunk[..n]);
    }
    let mut bytes: Vec<u8> = Vec::with_capacity(samples.len() * 8);
    for s in &samples {
        bytes.extend_from_slice(&s.re.to_le_bytes());
        bytes.extend_from_slice(&s.im.to_le_bytes());
    }
    bytes.truncate((want * 8).min(bytes.len()));
    std::fs::write(out, &bytes).expect("write");
    eprintln!(
        "wrote {} bytes = {:.2} s at {got_rate} (asked {rate}), centred {:.4} MHz",
        bytes.len(),
        bytes.len() as f64 / 8.0 / got_rate,
        got_freq / 1e6,
    );
}
