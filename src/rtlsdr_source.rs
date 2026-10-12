//! An [`IqSource`] for an RTL2832U dongle driven by the native driver in
//! `sdroxide-rtlsdr` — no SoapySDR, no libusb.
//!
//! Two ways in, one source: [`RtlSdrSource::open`] takes a dongle on this
//! machine's USB bus, [`RtlSdrSource::connect`] one published over the network
//! by an `rtl_tcp` server. Everything below the constructors is shared,
//! because it is the same radio and the same samples — only the distance to
//! its registers differs.
//!
//! Receive only: the trait's transmit methods already default to errors, which
//! is the correct answer for this hardware.

use std::time::Duration;

use sdroxide_dsp::IqCorrect;
use sdroxide_radio::{Complex32, DC_BLOCK_HZ, IqSource, Result};
use sdroxide_rtlsdr::RtlSdrHandle;
use sdroxide_rtlsdr::tcp::proto::RspCmd;
use sdroxide_types::{RtlSdrAgc, RtlSdrConfig, RtlSdrHfMode, RtlTcpConfig};

/// How long the dongle may deliver nothing before the connection counts as
/// dead. Shorter than the HPSDR backend's five seconds: this is a local USB
/// device, so there is no network to be briefly slow.
const SILENCE_BEFORE_REOPEN: Duration = Duration::from_secs(3);

/// The same, for an `rtl_tcp` link — where there *is* a network to be briefly
/// slow. Matches the HPSDR backend's window, and is long enough that a
/// congested WiFi hop does not get read as a dead server and reconnected out
/// from under a stream that was about to recover.
const SILENCE_BEFORE_RECONNECT: Duration = Duration::from_secs(5);

pub struct RtlSdrSource {
    handle: RtlSdrHandle,
    center: f64,
    label: String,
    /// Last gain the operator asked for, and what the hardware snapped it to.
    gain_db: f64,
    agc: RtlSdrAgc,
    bias_tee: bool,
    /// How long a silence has to last before this source asks to be reopened.
    silence: Duration,
    /// Whether the dongle is on the other end of a network link. Only the
    /// operator-facing wording depends on it — a bias tee somewhere else is a
    /// different warning from one on the desk.
    remote: bool,
    /// Front-end DC and image correction, applied to every block as it arrives
    /// so the panadapter, the demodulators and the skimmers all see the same
    /// clean stream. `None` while the operator has it switched off — the
    /// samples then reach the engine exactly as the dongle produced them.
    /// See [`IqCorrect`].
    iq_correct: Option<IqCorrect>,
}

impl RtlSdrSource {
    /// A dongle on this machine's USB bus.
    pub fn open(cfg: &RtlSdrConfig, center_hz: f64) -> anyhow::Result<Self> {
        let handle = RtlSdrHandle::open(cfg, center_hz)?;
        Ok(Self::from_handle(
            handle,
            Settings {
                gain_db: cfg.tuner_gain_db,
                agc: cfg.agc,
                bias_tee: cfg.bias_tee,
                iq_correction: cfg.iq_correction,
                silence: SILENCE_BEFORE_REOPEN,
                remote: false,
            },
            center_hz,
        ))
    }

    /// A dongle published over the network by an `rtl_tcp` server.
    ///
    /// The far end owns the hardware, so everything here is a request rather
    /// than a readback — including the sample rate and the gain the stream is
    /// actually running at. That is a property of the protocol, not of this
    /// connection; see [`RtlSdrHandle::connect`].
    pub fn connect(cfg: &RtlTcpConfig, center_hz: f64) -> anyhow::Result<Self> {
        let handle = RtlSdrHandle::connect(cfg, center_hz)?;
        Ok(Self::from_handle(
            handle,
            Settings {
                gain_db: cfg.tuner_gain_db,
                agc: cfg.agc,
                bias_tee: cfg.bias_tee,
                iq_correction: cfg.iq_correction,
                silence: SILENCE_BEFORE_RECONNECT,
                remote: true,
            },
            center_hz,
        ))
    }

    fn from_handle(handle: RtlSdrHandle, s: Settings, center_hz: f64) -> Self {
        let label = format!("{} @ {:.3} Msps", handle.label, handle.sample_rate_hz / 1e6);
        tracing::info!("RTL-SDR source ready: {label}, center {center_hz:.0} Hz");
        RtlSdrSource {
            center: center_hz,
            label,
            gain_db: s.gain_db,
            agc: s.agc,
            bias_tee: s.bias_tee,
            silence: s.silence,
            remote: s.remote,
            iq_correct: s.iq_correction.then(|| IqCorrect::new(DC_BLOCK_HZ, handle.sample_rate_hz)),
            handle,
        }
    }

    pub fn sample_rate_hz(&self) -> f64 {
        self.handle.sample_rate_hz
    }

    /// Pass one `rsp_tcp` control through, and only over the network. Over USB
    /// there is no server to send it to, and the element names are shared
    /// because both interfaces are the same `IqSource`.
    fn set_rsp(&self, cmd: RspCmd, arg: u32) {
        if self.remote {
            self.handle.set_rsp(cmd, arg);
        }
    }

    pub fn tuner(&self) -> &str {
        &self.handle.tuner
    }

    pub fn is_blog_v4(&self) -> bool {
        self.handle.is_blog_v4
    }

    /// Whether this dongle is configured to reach below the tuner's 24 MHz
    /// floor, through an upconverter or direct sampling.
    pub fn hf_capable(&self) -> bool {
        self.handle.hf_capable
    }
}

/// The settings the two constructors have in common — the same knobs in
/// [`RtlSdrConfig`] and [`RtlTcpConfig`], plus what the link implies.
struct Settings {
    gain_db: f64,
    agc: RtlSdrAgc,
    bias_tee: bool,
    iq_correction: bool,
    silence: Duration,
    remote: bool,
}

impl IqSource for RtlSdrSource {
    /// The engine is transmitting and has stopped reading, or has started
    /// again. Passed straight through to the stream thread, which keeps
    /// receiving either way — this only decides whether a full ring is
    /// reported as an overrun or as the ordinary cost of an over. See
    /// [`IqSource::set_rx_paused`].
    fn set_rx_paused(&mut self, paused: bool) {
        self.handle.set_rx_paused(paused);
    }

    fn sample_rate(&self) -> f64 {
        self.handle.sample_rate_hz
    }

    fn center_hz(&self) -> f64 {
        self.center
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        self.handle.set_center_hz(hz);
        Ok(())
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        // Straight into the caller's block: the ring's interleaved floats and a
        // complex buffer are the same bytes, so there is nothing to unpack —
        // see `sdroxide_dsp::as_interleaved_mut`. The scratch vector and the
        // loop that copied out of it were a second pass over the whole stream
        // to change nothing but the type.
        let n = self.handle.rx_read(sdroxide_dsp::as_interleaved_mut(buf));
        let pairs = n / 2;
        if pairs == 0 {
            // Nothing yet — brief nap so the DSP loop doesn't spin hot.
            std::thread::sleep(Duration::from_millis(2));
            return Ok(0);
        }
        if let Some(iq) = self.iq_correct.as_mut() {
            iq.process(&mut buf[..pairs]);
        }
        Ok(pairs)
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    /// The tuner gain, plus a pseudo-element carrying the AGC mode.
    ///
    /// Routing AGC through `SetGain` rather than adding a `Command` variant
    /// keeps `Command`, `DeviceCaps` and the engine untouched for a setting
    /// only this backend has. The encoding lives with
    /// [`RtlSdrAgc::code`] so the two ends cannot drift apart.
    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        match name {
            RtlSdrConfig::TUNER_GAIN_ELEMENT => {
                self.gain_db = db;
                self.handle.set_gain_db(db);
            }
            RtlSdrConfig::AGC_ELEMENT => {
                self.agc = RtlSdrAgc::from_code(db.round().clamp(0.0, 3.0) as u8);
                self.handle.set_agc(self.agc);
            }
            RtlSdrConfig::PPM_ELEMENT => {
                self.handle.set_ppm(db.round().clamp(-1000.0, 1000.0) as i32);
            }
            RtlSdrConfig::HF_MODE_ELEMENT => {
                self.handle.set_hf_mode(RtlSdrHfMode::from_code(db.round().clamp(0.0, 2.0) as u8));
            }
            RtlSdrConfig::BIAS_TEE_ELEMENT => {
                self.bias_tee = db >= 0.5;
                self.handle.set_bias_tee(self.bias_tee);
            }
            // The `rsp_tcp` controls. Reachable only on the network interface,
            // and only against an SDRplay server — a plain `rtl_tcp` one does
            // not know these opcodes and ignores them in silence, which is what
            // this protocol does with everything it does not recognise. The
            // settings tab says so rather than pretending to have detected it.
            RtlTcpConfig::RSP_ANTENNA_ELEMENT => {
                self.set_rsp(RspCmd::SetAntenna, db.clamp(0.0, 2.0) as u32)
            }
            RtlTcpConfig::RSP_LNA_STATE_ELEMENT => {
                self.set_rsp(RspCmd::SetLnaState, db.clamp(0.0, 9.0) as u32)
            }
            RtlTcpConfig::RSP_IFGR_ELEMENT => {
                self.set_rsp(RspCmd::SetIfGainR, db.clamp(20.0, 59.0) as u32)
            }
            RtlTcpConfig::RSP_AGC_ELEMENT => self.set_rsp(RspCmd::SetAgc, u32::from(db >= 0.5)),
            RtlTcpConfig::RSP_AGC_SETPOINT_ELEMENT => {
                // The protocol carries an unsigned argument and the set point
                // is negative dBfs, so it goes out as its two's-complement bit
                // pattern — the same way ppm and gain already do.
                self.set_rsp(RspCmd::SetAgcSetPoint, db.clamp(-72.0, 0.0) as i32 as u32)
            }
            RtlTcpConfig::RSP_NOTCH_ELEMENT => {
                self.set_rsp(RspCmd::SetNotch, db.clamp(0.0, 15.0) as u32)
            }
            RtlTcpConfig::RSP_REF_OUT_ELEMENT => {
                self.set_rsp(RspCmd::SetRefOut, u32::from(db >= 0.5))
            }
            // Purely a host-side correction, so it switches on and off between
            // one block and the next with nothing to reconfigure on the dongle.
            // Switching it on starts from a fresh estimate rather than whatever
            // the loop was left holding, which may be from another band.
            RtlSdrConfig::IQ_CORRECTION_ELEMENT => {
                if db >= 0.5 {
                    match self.iq_correct.as_mut() {
                        Some(iq) => iq.reset(),
                        None => {
                            self.iq_correct =
                                Some(IqCorrect::new(DC_BLOCK_HZ, self.handle.sample_rate_hz));
                        }
                    }
                } else {
                    self.iq_correct = None;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn current_gains(&self) -> Vec<(String, f64)> {
        vec![
            (RtlSdrConfig::TUNER_GAIN_ELEMENT.to_string(), self.gain_db),
            (RtlSdrConfig::AGC_ELEMENT.to_string(), self.agc.code() as f64),
        ]
    }

    /// The gain the dongle is applying, so the S-meter stops adding it to the
    /// reading (issue #68).
    ///
    /// A dongle with tuner gain on delivers samples tens of dB hotter than the
    /// signal on the antenna, and the meter subtracts the front end's own gain
    /// before showing dBm. That subtraction is `rx_gain_db().unwrap_or(0.0)`, so
    /// a source that answers `None` has its whole gain added to the reading: an
    /// RTL-SDR's noise floor came out at S9+10 — an ordinary floor, reported
    /// ~30 dB too high. Only SDRplay and Pluto answered, so this was every
    /// RTL-SDR.
    ///
    /// **`None` whenever AGC is running.** The trait asks for the gain of the
    /// samples *being delivered now*, and under tuner or RTL digital AGC the
    /// hardware moves it on its own: `gain_db` is the last value the *operator*
    /// commanded and nothing rewrites it, so answering it would put a number on
    /// screen that the antenna never saw — a meter that jumps on a strong
    /// signal instead of reporting one. Manual gain is the only setting where
    /// the commanded value is the applied value, and it is also the setting for
    /// weak-signal work, where a correct S-number is the point.
    fn rx_gain_db(&self) -> Option<f32> {
        applied_gain_db(self.agc, self.gain_db)
    }

    /// A dongle that has been unplugged, or whose thread has died, is reported
    /// as needing a reopen so the engine reconnects on its own — which is what
    /// makes replugging one Just Work rather than needing Apply pressed. Over
    /// `rtl_tcp` the same rule reconnects to a server that was restarted, or
    /// that dropped this client for falling behind.
    fn needs_reopen(&self) -> bool {
        !self.handle.is_alive() || self.handle.silent_for() >= self.silence
    }

    /// Hand the dongle back before the engine opens its replacement. Without
    /// this, changing anything in Settings → Radio on a running RTL-SDR fails
    /// with "held by another program" — the other program being us.
    fn release(&mut self) {
        self.handle.release();
    }

    /// Surface what an operator needs to know but cannot see. A bias tee
    /// putting DC on the feedline is worth a standing on-screen reminder — the
    /// setting is persisted, so it survives a restart with nothing else to
    /// indicate it. More so over `rtl_tcp`, where the coax it is feeding is
    /// wherever the server is and may not be in the room.
    fn open_status(&self) -> Option<String> {
        self.bias_tee.then(|| {
            let where_ = if self.remote { " at the far end" } else { "" };
            format!("{}: bias tee is ON — ~4.5 V DC on the antenna coax{where_}", self.label)
        })
    }
}

/// The gain the meter should subtract, given the AGC setting and the gain the
/// dongle was last commanded to.
///
/// Split out from [`RtlSdrSource::rx_gain_db`] so the rule is testable without a
/// dongle: it is a decision about two numbers, and the decision is the whole of
/// issue #68.
fn applied_gain_db(agc: RtlSdrAgc, commanded_db: f64) -> Option<f32> {
    match agc {
        // Manual: the commanded value is the applied value.
        RtlSdrAgc::Manual => Some(commanded_db as f32),
        // Tuner or RTL AGC: the hardware moves the gain itself and never tells
        // us where it is, so there is no honest number to report. Answering the
        // commanded value here would put a figure on the meter that the
        // antenna never saw.
        _ => None,
    }
}

#[cfg(test)]
mod gain_tests {
    use super::applied_gain_db;
    use sdroxide_types::RtlSdrAgc;

    /// The fault: a manual-gain RTL-SDR reported its whole tuner gain as signal,
    /// so a normal noise floor read as S9+10 — about 30 dB too high (issue #68).
    #[test]
    fn manual_gain_is_reported_so_the_meter_can_subtract_it() {
        assert_eq!(
            applied_gain_db(RtlSdrAgc::Manual, 30.0),
            Some(30.0),
            "manual gain is known, so the meter must be able to subtract it"
        );
    }

    /// Under AGC the hardware drives the gain and we never learn its value.
    /// Claiming the commanded one would put a number on the meter that no
    /// antenna ever saw, so every AGC setting must decline.
    #[test]
    fn no_agc_setting_reports_a_gain_it_does_not_know() {
        for agc in [RtlSdrAgc::Tuner, RtlSdrAgc::Rtl, RtlSdrAgc::Both] {
            assert_eq!(
                applied_gain_db(agc, 30.0),
                None,
                "{agc:?} moves the gain itself; the commanded value is not the applied one"
            );
        }
    }

    /// Every AGC setting must answer, so a new one cannot silently fall through
    /// to reporting the commanded gain.
    #[test]
    fn every_agc_setting_is_decided() {
        assert_eq!(RtlSdrAgc::ALL.len(), 4, "the rule covers all four settings");
        for agc in RtlSdrAgc::ALL {
            let _ = applied_gain_db(agc, 20.0);
        }
    }

    /// The scale the reporter's numbers imply: a noise floor reported at S9+10
    /// is −63 dBm, which is what an RTL-SDR's samples read with ~30 dB of
    /// un-subtracted gain on them. Subtracting the gain brings it down by five
    /// S-units, to about S4, which is where a receiver's floor belongs. This is
    /// the arithmetic behind issue #68 and it is worth pinning, because "the
    /// meter looks wrong" is not otherwise checkable.
    #[test]
    fn subtracting_the_gain_lowers_the_floor_by_the_s_units_it_was_raised() {
        use sdroxide_types::Meters;

        let reading = |dbm: f32| {
            let m = Meters {
                s_dbm: dbm,
                adc_peak_dbfs: f32::NEG_INFINITY,
                pa_temp_c: None,
                adc_clip: 0.0,
                adc_overload: None,
                tx: None,
                stereo: false,
                tone: None,
                passband_dbfs: f32::NEG_INFINITY,
                puresignal: None,
            };
            m.s_units()
        };

        // S9+10, as reported on the issue.
        let (s, over) = reading(-63.0);
        assert_eq!(s, 9);
        assert_eq!(over, 10.0, "S9+10 is the reported reading");

        let gain = applied_gain_db(RtlSdrAgc::Manual, 30.0).expect("manual gain");
        let (corrected_s, _) = reading(-63.0 - gain);

        // 30 dB is five S-units of 6 dB, but the *display* drops by four: S9+10 and S9
        // are both shown as "S9" with an over-value beside it, so the starting
        // reading carries no unit of its own to subtract. That is the meter's
        // existing whole-S behaviour and not something this fix changes.
        assert_eq!(
            s as i32 - corrected_s as i32,
            4,
            "subtracting {gain} dB must lower the S-number"
        );
        assert!(
            (1..=6).contains(&corrected_s),
            "a noise floor belongs in single figures, not S{corrected_s}"
        );
    }
}
