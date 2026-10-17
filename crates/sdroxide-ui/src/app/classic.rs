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
const CLASSIC_MAX_W: f32 = 420.0;

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
        egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("classic-column").show(
            ui,
            |ui| {
                self.classic_source_section(ui, cmds);
                self.classic_mode_section(ui, cmds);
                self.classic_radio_section(ui, cmds);
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
            assert!(
                w - max >= CLASSIC_PANADAPTER_MIN_W,
                "{w}: a column of {max} leaves the panadapter less than its floor"
            );
        }

        // And the width it opens at is always within the range it offers.
        assert!(classic_column_width(640.0).is_some_and(|(min, max)| min <= 280.0 && 280.0 <= max));
    }
}
