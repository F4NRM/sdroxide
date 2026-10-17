//! **Classic UI** — every control in one column down the left.
//!
//! The layout SDR++ and SDR++Brown use, and therefore the one most short-wave
//! listeners already know: a tall column of sections, each folding away, with
//! the picture taking everything else. It is on `main` because it answers the
//! complaint this fork keeps hearing — two readers on r/ShortwavePlus, both SDR++
//! users, called the program *"too complicated, controls all over the place"* —
//! and the whole difference is that our controls live in a strip of boxes with
//! twelve menus behind them and theirs live in one visible column.
//!
//! **Same controls, same commands.** Not one thing here is new behaviour: the
//! Radio section is the RX menu's own body (`rx_controls`), inlined; the Mode
//! section is `mode_chip_grid`, the same matrix the band menu draws. That is
//! deliberate, and it is what keeps the two arrangements from drifting apart —
//! a second implementation of TUNE or of the squelch would be a second thing to
//! fix twice.
//!
//! Off by default, and desktop only: the strip is what this program's own
//! screens are built around, and a phone has no room for a column at all.

use eframe::egui::{self, RichText};
use sdroxide_types::Command;

/// The column's width range, in points.
///
/// **The floor is SDR++'s own** (their dotted rows need about this much before
/// a value and its `[−][+]` stop fitting on one line), and the ceiling is where a
/// section starts to look like a page rather than a column. Both are wide enough
/// that the sections below never have to reflow.
const CLASSIC_MIN_W: f32 = 232.0;

/// The widest the column may be dragged — *generous*, and that is the lesson
/// from the first version. It capped at 420, which is where the operator's drag
/// stopped, and a handle that refuses to move reads as a handle that does not
/// work however true the reason is. Our sections are the strip's own modules and
/// they want more room than SDR++'s rows do.
const CLASSIC_MAX_W: f32 = 640.0;

/// How much room the picture must keep, so opening the column on a narrow
/// window squeezes the waterfall rather than taking it away.
const CLASSIC_PANADAPTER_MIN_W: f32 = 360.0;

/// The width the column opens at, and may be dragged between, for a window this
/// wide.
///
/// Pure, so the arithmetic that stops the column eating the panadapter is a
/// thing a test can hold rather than a property of a running window. Returns
/// `None` when there is no room at all — the caller then leaves the column out
/// and the strip behaves as it always did.
pub(in crate::app) fn classic_column_width(window_w: f32) -> Option<(f32, f32)> {
    let room = window_w - CLASSIC_PANADAPTER_MIN_W;
    if room < CLASSIC_MIN_W {
        return None;
    }
    // The ceiling is the room that is left, capped by taste rather than by a
    // number the content forced — see `CLASSIC_MAX_W`.
    let max = room.min(CLASSIC_MAX_W);
    Some((CLASSIC_MIN_W, max))
}

impl super::SdroxideApp {
    /// The column: a scroll area of sections, each folding away.
    ///
    /// `egui::CollapsingHeader` rather than a hand-rolled one, because it is
    /// what this program already uses for a fold of settings (the Alerts and
    /// Announcements pages) and it keeps its own open-or-closed state between
    /// sessions. A second implementation would be a second thing to get wrong
    /// for no gain.
    pub(in crate::app) fn classic_column(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        // **Both axes, and that is what makes the handle work.** A vertical-only
        // scroll area still lets a row wider than the column widen the column
        // itself — the panel's width follows its content's minimum, so the
        // handle springs back and reads as broken. Scrolled on both axes, the
        // panel keeps the width it was given and a row that does not fit
        // scrolls, which also *says* the column is too narrow rather than
        // silently refusing to narrow.
        egui::ScrollArea::both().auto_shrink([false, false]).id_salt("classic-column").show(
            ui,
            |ui| {
                self.classic_source_section(ui, cmds);
                self.classic_bands_section(ui, cmds);
                self.classic_mode_section(ui, cmds);
                self.classic_radio_section(ui, cmds);
                // The strip's own modules, one fold each — the very bodies the
                // strip draws, called with the column's width. Nothing here is a
                // second implementation, which is what keeps the two
                // arrangements from drifting apart.
                let w = ui.available_width();
                self.classic_tuning_section(ui, cmds, w);
                self.classic_tx_section(ui, cmds, w);
                self.classic_display_section(ui, cmds, w);
                self.classic_system_section(ui, cmds, w);
            },
        );
    }

    /// **Source** — which radio this is, and what it is doing.
    ///
    /// SDR++'s first section is the front end itself: the device, the rate, the
    /// gain. Ours is chosen on Settings → Radio, so this reads rather than
    /// edits and puts the route beside the values. Saying what is configured and
    /// where to change it is the honest half of the section; a second gain
    /// slider here would be a second opinion about the same setting.
    fn classic_source_section(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        egui::CollapsingHeader::new(RichText::new("Source").strong()).default_open(true).show(
            ui,
            |ui| {
                let label = self
                    .caps
                    .as_ref()
                    .map(|c| c.label.clone())
                    .unwrap_or_else(|| "no radio".to_string());
                source_row(ui, "Radio", &label);
                source_row(ui, "Rate", &format!("{:.3} Msps", self.state.sample_rate / 1e6));
                source_row(ui, "Dial", &crate::app::frame::readout_text(self.state.rx_freq_hz()));
                source_row(ui, "Mode", self.state.rx[0].mode.label());
                if self.swl_mode() {
                    source_row(ui, "Receive", "listening screen");
                }
                ui.add_space(4.0);
                if crate::chrome::chip(ui, self.show_settings, "RADIO SETTINGS")
                    .on_hover_text(
                        "The backend, the sample rate, the aerial, the converter and this \
                         radio's own calibration — Settings → Radio, where they have always \
                         been. The column shows what they are set to rather than a second copy \
                         of the controls.",
                    )
                    .clicked()
                {
                    self.show_settings = true;
                }
                let _ = cmds;
            },
        );
    }

    /// **Bands** — the band list and its filters, in the column where an SDR++
    /// hand looks for the band plan. It is `band_menu_body`: the same body the
    /// docked column and the detached band window draw, so there is one band
    /// selector in the program and not three.
    fn classic_bands_section(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        egui::CollapsingHeader::new(RichText::new("Bands").strong()).default_open(true).show(
            ui,
            |ui| {
                self.band_menu_body(ui, cmds);
            },
        );
    }

    /// **Tuning** — the VFO, split and the RIT/XIT offsets: the strip's own
    /// module, drawn here.
    fn classic_tuning_section(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, w: f32) {
        egui::CollapsingHeader::new(RichText::new("Tuning").strong()).default_open(false).show(
            ui,
            |ui| {
                self.vfo_rit_module(ui, cmds, w);
                ui.add_space(4.0);
                self.rx_filter_module(ui, cmds, w);
            },
        );
    }

    /// **Transmit**, **Display**, **System** — the strip's own modules again,
    /// one fold each. The System fold is where the windows and the tools live,
    /// which is what SDR++ calls its plugin list.
    fn classic_tx_section(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, w: f32) {
        egui::CollapsingHeader::new(RichText::new("Transmit").strong()).default_open(false).show(
            ui,
            |ui| {
                self.tx_condensed(ui, cmds, w);
            },
        );
    }

    fn classic_display_section(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, w: f32) {
        egui::CollapsingHeader::new(RichText::new("Display").strong()).default_open(false).show(
            ui,
            |ui| {
                self.display_condensed(ui, cmds, w);
            },
        );
    }

    fn classic_system_section(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, w: f32) {
        egui::CollapsingHeader::new(RichText::new("Windows & tools").strong())
            .default_open(false)
            .show(ui, |ui| {
                self.windows_condensed(ui, w, cmds);
            });
    }

    /// **Mode** — the same matrix the band menu draws, in the column where an
    /// SDR++ hand looks for it.
    fn classic_mode_section(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        egui::CollapsingHeader::new(RichText::new("Mode").strong()).default_open(true).show(
            ui,
            |ui| {
                super::top_bar::mode_chip_grid(
                    ui,
                    self.state.rx[0].mode,
                    &super::top_bar::OPERATE_MODES,
                    self.state.band,
                    &self.state,
                    false,
                    cmds,
                );
            },
        );
    }

    /// **Radio** — the receiver's controls, which is the strip's RX menu body
    /// drawn here instead of in a popup.
    fn classic_radio_section(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        egui::CollapsingHeader::new(RichText::new("Radio").strong()).default_open(true).show(
            ui,
            |ui| {
                // `narrow = true` is not about width here: it is the flag that
                // tells the body it is being inlined rather than shown as a
                // popup, so it lays its sub-menus out flat instead of opening
                // them — a popup cannot open from inside a popup, which is the
                // same reason the compact strip inlines its picker.
                self.rx_controls(ui, cmds, true);
            },
        );
    }
}

/// One `label — value` line, the shape every row of the Source section takes.
fn source_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(11.0).color(crate::theme::gray(150)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).size(11.5).color(crate::theme::TEXT_STRONG()));
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The handle moves, and the mechanism is the scroll area.** egui's own
    /// note on `resizable` says the ui must *use* the space it was given — and
    /// that is necessary but not sufficient, which the first version of this
    /// test proved by being wrong. `take_available_space` sets a *minimum*; a
    /// child wider than the panel still drags the panel out to its own width,
    /// so the handle springs back and reads as broken.
    ///
    /// What fixes it is scrolling: inside a scroll area the content's minimum
    /// is the viewport's, so the panel keeps the width it was given and the row
    /// scrolls. Measured here at a width the content does not fit in.
    #[test]
    fn the_column_keeps_its_width_when_a_row_does_not_fit() {
        let width_with = |scroll: bool| {
            let ctx = egui::Context::default();
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 800.0));
            let input = egui::RawInput { screen_rect: Some(rect), ..Default::default() };
            let mut w = 0.0;
            for _ in 0..3 {
                let full = ctx.run_ui(input.clone(), |ui| {
                    let r = egui::Panel::left(egui::Id::new(("probe", scroll)))
                        .resizable(true)
                        .default_size(240.0)
                        .size_range(200.0..=640.0)
                        .show(ui, |ui| {
                            ui.take_available_space();
                            let row = |ui: &mut egui::Ui| {
                                // 12 wide labels: about 600 points in a 240-point panel.
                                for i in 0..12 {
                                    ui.label(RichText::new(format!("chip {i} is wide")).size(11.0));
                                }
                            };
                            if scroll {
                                egui::ScrollArea::both().auto_shrink([false, false]).show(ui, row);
                            } else {
                                ui.horizontal(row);
                            }
                        });
                    w = r.response.rect.width();
                });
                full.drop_without_applying_deltas();
            }
            w
        };
        let without = width_with(false);
        let with = width_with(true);
        assert!(
            (with - 240.0).abs() < 2.0,
            "the scrolling column took {with} points where it was asked for 240"
        );
        assert!(
            without > with + 100.0,
            "the unscrolled column did not widen with its content ({without} against {with}), \
             so this test is not measuring what it claims"
        );
    }

    /// **The column never eats the picture.** It is offered only where the
    /// panadapter keeps its floor, and its own ceiling is the room that is left
    /// — not a fixed number that a narrow window would overrun. This is the
    /// #643 shape: a column that grows past what it was given takes the thing
    /// beside it off the screen.
    #[test]
    fn the_column_yields_to_the_panadapter() {
        // A phone, and a window too narrow to spare it: no column at all.
        assert!(classic_column_width(360.0).is_none());
        assert!(classic_column_width(500.0).is_none());

        // Where it is offered, the range is inside the room left over.
        for w in [620.0, 960.0, 1280.0, 1920.0, 3840.0] {
            let (min, max) = classic_column_width(w).expect("room at {w}");
            assert!(min <= max, "{w}: range {min}..{max} is inverted");
            assert!(max <= CLASSIC_MAX_W, "{w}: opens wider than the ceiling");
            // And the ceiling is taste, not content: there is real room above
            // the widest it allows on a normal window, so a drag to the right
            // is a drag and not a refusal.
            assert!(
                w < 1280.0 || w - max >= CLASSIC_PANADAPTER_MIN_W,
                "{w}: the ceiling binds before the panadapter's floor does"
            );
            assert!(
                w - max >= CLASSIC_PANADAPTER_MIN_W,
                "{w}: a column of {max} leaves the panadapter less than its floor"
            );
        }

        // And the width it opens at is always within the range it offers.
        assert!(classic_column_width(640.0).is_some_and(|(min, max)| min <= 280.0 && 280.0 <= max));
    }
}
