//! Auto mode — the app-side half of the unattended FT8/FT4/FT2 sequencer.
//!
//! The policy (which station to answer, whether the mode and band allow it) is
//! pure and lives in [`sdroxide_types::auto`]. This is the loop that runs it
//! once a frame: it keeps the operator's input clock, honours the transmit
//! watchdog as a pause rather than a stop, and puts the chosen action on the
//! command channel to the engine.
//!
//! It runs app-wide rather than inside the FT8 panel for a reason: an
//! unattended run must not depend on which pane or tab is on screen. The engine
//! does the sequencing; this only ever starts the next contact.

use sdroxide_types::auto;
use sdroxide_types::{Band, Command, QsoStep};

use super::SdroxideApp;

impl SdroxideApp {
    /// Whether this station may key the band the dial is on, by the same
    /// band-level rule the engine's key-down gate applies: an amateur
    /// allocation always, 11 m once the operator has opened it, and the
    /// broadcast and general-coverage bands never.
    ///
    /// The engine still has the final say at key-down — the device's own range,
    /// the offset against the band plan — but this is the question auto mode
    /// has to answer before it offers to arm at all.
    pub(in crate::app) fn auto_tx_permitted(&self) -> bool {
        let band = Band::containing(self.state.rx_freq_hz());
        auto::band_may_transmit(band, !self.state.oob_tx, sdroxide_types::cb_tx_allowed())
    }

    /// The inactivity stop for this radio, in seconds, from its own setting,
    /// or `None` when it is switched off (a PlutoSDR set to DISABLE).
    fn auto_idle_stop_s(&self) -> Option<f64> {
        let (min, long) = self
            .radio_cfg
            .as_ref()
            .map_or((0, false), |c| (c.auto_idle_stop_min, auto::has_long_idle_stop(c.backend)));
        auto::auto_idle_stop_s(min, long)
    }

    /// Run auto mode for this frame. Cheap and a no-op unless it is armed.
    pub(in crate::app) fn tick_auto_mode(
        &mut self,
        ctx: &eframe::egui::Context,
        now: f64,
        cmds: &mut Vec<Command>,
    ) {
        // Any input event at all counts as the operator being present. A
        // minimized or unfocused window produces none, which is exactly what
        // the inactivity stop is for.
        if !ctx.input(|i| i.events.is_empty()) {
            self.auto_last_activity = now;
        }
        if !self.auto_mode {
            return;
        }
        // The engine's own view of the mode, band and watchdog, not the UI's
        // editable copies: this is what will actually key.
        let Some((mode, step, watchdog, tx_watchdog, status_tx_next, my_call, my_grid)) =
            self.digi_status.as_ref().map(|s| {
                (
                    s.mode,
                    s.step,
                    s.config.tx_watchdog_min,
                    s.tx_watchdog,
                    s.tx_next,
                    s.config.my_call.clone(),
                    s.config.my_grid.clone(),
                )
            })
        else {
            return;
        };
        let tx_permitted = self.auto_tx_permitted();
        if let Some(reason) = auto::auto_block_reason(mode, watchdog, tx_permitted) {
            self.stop_auto(format!("auto stopped: {reason}"), cmds);
            return;
        }
        if let Some(stop_s) = self.auto_idle_stop_s()
            && now - self.auto_last_activity >= stop_s
        {
            let mins = (stop_s / 60.0).round() as u32;
            self.stop_auto(format!("auto stopped: {mins} minutes with no operator input"), cmds);
            return;
        }
        // Only start a contact from a state that is genuinely free. `Idle` is
        // the obvious one; `Confirming` is the engine holding a *logged*
        // contact for a few minutes in case the far end repeats its final
        // message, and waiting out that hold would idle an unattended run for
        // five minutes a contact. It is free once no re-send is owed — while
        // one is, the engine is about to transmit and must be left to it.
        let free = step == QsoStep::Idle || (step == QsoStep::Confirming && !status_tx_next);
        if !free {
            self.auto_resume_at = None;
            return;
        }
        if now < self.auto_cooldown_until {
            return;
        }
        // The transmit watchdog is the pacing, not the end: after it trips,
        // wait one watchdog span and try again.
        if tx_watchdog {
            let span = f64::from(watchdog) * 60.0;
            let resume = *self.auto_resume_at.get_or_insert(now + span);
            if now < resume {
                return;
            }
        }
        self.auto_resume_at = None;

        match self.auto_target(&my_call, &my_grid) {
            Some(t) => {
                self.auto_tried.insert(t.call.to_ascii_uppercase());
                self.auto_note = format!("auto: answering {}", t.call);
                cmds.push(Command::DigiStartQso {
                    from: t.call,
                    grid: t.grid,
                    snr: t.snr_db,
                    audio_hz: t.audio_hz,
                    wait_for_cq: false,
                });
            }
            None => {
                self.auto_note = "auto: calling CQ".into();
                cmds.push(Command::DigiCallCq);
            }
        }
        self.auto_cooldown_until = now + auto::AUTO_COOLDOWN_S;
    }

    /// Keep the log index current and run the pure selection against it.
    ///
    /// The index is borrowed directly rather than cloned: it is the whole log
    /// as sets, and cloning it every frame would be the one expensive thing in
    /// this path.
    ///
    /// The ignore list is asked again here even though the ingress filter
    /// already drops new decodes, because a row that was in the list *before*
    /// the operator muted that station is still in it. Muting is an
    /// instruction not to answer somebody, and auto mode answers on its own
    /// initiative — so it honours the list for a decode it would otherwise
    /// pick out of the ones already held.
    ///
    /// The list is borrowed, not copied: the rule is handed to the picker as a
    /// predicate so it can drop those rows in place. An earlier cut built a
    /// filtered `Vec` of the whole decode list instead, which meant cloning up
    /// to 200 rows every frame for the sake of the few that were muted.
    fn auto_target(&mut self, my_call: &str, my_grid: &str) -> Option<auto::AutoTarget> {
        let len = self.qso_log.len();
        if self.log_index_cache.as_ref().map(|(l, _)| *l) != Some(len) {
            self.log_index_cache = Some((len, sdroxide_types::LogIndex::build(&self.qso_log)));
        }
        let ix = &self.log_index_cache.as_ref().expect("just filled").1;
        if self.session_ignored.is_empty() {
            return auto::pick_cq(
                &self.digi_decodes,
                ix,
                my_call,
                my_grid,
                &self.auto_tried,
                |_| false,
            );
        }
        auto::pick_cq(&self.digi_decodes, ix, my_call, my_grid, &self.auto_tried, |d| {
            crate::app::ignore::is_ignored(&self.session_ignored, d.from.as_deref())
        })
    }

    /// Disarm auto mode, leaving `note` as the reason shown on the toggle.
    ///
    /// Disarming is a kill switch, not just a stop-selecting: it also tells
    /// the engine to stop the QSO and abort any burst in flight. An unattended
    /// run that has gone wrong must be stoppable in one click, and leaving the
    /// contact in hand to sequence on its own is exactly the "runs wild" this
    /// exists to end.
    pub(in crate::app) fn stop_auto(&mut self, note: String, cmds: &mut Vec<Command>) {
        if self.auto_mode {
            cmds.push(Command::DigiStopQso);
            cmds.push(Command::DigiAbortTx);
        }
        self.disarm_auto(note);
    }

    /// Clear auto mode without sending anything, for the case where the stop
    /// command is already on its way (STOP QSO, STOP TX, a bound Abort TX).
    pub(in crate::app) fn disarm_auto(&mut self, note: String) {
        if self.auto_mode {
            self.auto_mode = false;
            self.auto_resume_at = None;
            self.auto_note = note;
        }
    }

    /// Arm auto mode now. Returns `false` (doing nothing) when it may not run.
    pub(in crate::app) fn arm_auto(&mut self, now: f64) -> bool {
        let Some(s) = self.digi_status.as_ref() else { return false };
        let tx_permitted = self.auto_tx_permitted();
        if auto::auto_block_reason(s.mode, s.config.tx_watchdog_min, tx_permitted).is_some() {
            return false;
        }
        self.auto_mode = true;
        self.auto_last_activity = now;
        self.auto_cooldown_until = 0.0;
        self.auto_resume_at = None;
        self.auto_note = "auto: hunting".into();
        true
    }
}
