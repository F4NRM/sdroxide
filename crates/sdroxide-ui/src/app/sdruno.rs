//! **The SDRuno workstation** — SDRuno's window set and control-page structure,
//! as a mode the installed desktop program can wear.
//!
//! The operator's own words, on the branch this was started: *"I want a clone
//! behaviour of sdruno, not a cheap ripoff. I want a clone. That should not be
//! available for the server version but only for desktop mode."* And, asked
//! whether the look or the structure was meant: *"it is the structure 100%."*
//!
//! So this is a **structural** mode, not a palette:
//!
//! - **Every window carries SDRuno's identity.** SDRuno names its windows
//!   `SDRuno MAIN`, `SDRuno RX CONTROL`, `SDRuno MAIN SP`, `SDRuno SCANNER` —
//!   the program's name and the window's role, on the window itself. Our
//!   detached windows carry only a role (`sdroxide — panadapter`), so a desk
//!   full of them does not read as one program. In this mode each one is
//!   named SDRuno-style, and the role names are SDRuno's own (MAIN SP, AUX SP,
//!   RX CONTROL).
//! - **A MAIN hub window**, which is what SDRuno's MAIN is: a small control
//!   surface whose buttons open the rest, rather than the whole workspace. It
//!   is drawn into the centre of the main window when the radio's own modules
//!   have all gone to their own windows.
//! - **A dense RX CONTROL page**: the controls our strip hides in menus —
//!   MODE, FILTER, AGC, STEP, NR/NB/AN — are promoted to visible captioned
//!   matrices, which is the shape of SDRuno's RX control. One press, one
//!   meaning, no menu.
//! - **Windows that ask to float.** On X11 a window can declare itself a
//!   utility window (`_NET_WM_WINDOW_TYPE_UTILITY`), which every tiling window
//!   manager in use honours by floating it; that is done here for every window
//!   this mode opens. Wayland gives a client no such call, so there the float
//!   is the operator's own compositor rule on the app-id — see the note at the
//!   bottom of this file and the manual.
//!
//! **Desktop and native only, by construction.** The mode is a way of arranging
//! *OS windows*, and there are none on a phone or in a browser: the toggle is
//! offered only where the program is the installed one and the layout tier is
//! Desktop, and the setting is local `config.toml`, deliberately not part of
//! [`sdroxide_types::ClientScreen`]. A remote client of `sdroxide --server`
//! therefore keeps its own arrangement and the station never sends one — which
//! is what "not for the server version" means here.

use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui::{self, RichText};
use sdroxide_types::{AgcMode, Command, DetachableModule, NrLevel, RxId};

use crate::theme;

/// Where the mode is in force *for the frame that is drawing*.
///
/// A process-wide flag rather than a field read at each site, for the same
/// reason [`crate::layout::set_tier`] is one: the free functions that build a
/// window's spec (`detached_spec`, `detached_viewport`) have no `&SdroxideApp`
/// to ask, and threading a `bool` through every one of them would put a
/// parameter on a dozen signatures that mean nothing outside this mode.
///
/// Station-wide by nature — the setting lives in `UiSettings`, which is one
/// station's `config.toml` — so every radio in the shell agrees on it.
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Publish whether the workstation is in force for the frame about to draw.
pub(in crate::app) fn set_active(on: bool) {
    ACTIVE.store(on, Ordering::Relaxed);
}

/// Whether the workstation is in force this frame.
pub(in crate::app) fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

/// Whether the mode can be used **here**: the installed program, on a screen
/// with a desk.
///
/// A browser has no second OS window, and a phone has no desk to arrange — so
/// the mode is off in both, and the setting cannot turn anything on there even
/// if a `config.toml` were copied across. A tablet lives on this side of the
/// line: it runs the installed program and it has windows to place, which is
/// the sense of "desktop mode" the mode is gated on rather than the `Layout`
/// picker's own value — an operator on `Auto` should not have to change a
/// setting to try it.
pub(in crate::app) fn available(tier: crate::layout::Tier) -> bool {
    !cfg!(target_arch = "wasm32") && tier != crate::layout::Tier::Phone
}

/// Whether a window this mode opens should ask its window manager to float it.
///
/// `Some(Utility)` where it should: on X11 this is `_NET_WM_WINDOW_TYPE_UTILITY`
/// and every tiling window manager floats it, which is exactly the "tell the
/// tiling WM I need to float" the operator asked for. `None` outside the mode,
/// so an ordinary undocked window keeps behaving as it always has.
pub(in crate::app) fn float_hint() -> Option<egui::X11WindowType> {
    active().then_some(egui::X11WindowType::Utility)
}

/// A window's own title in this mode: SDRuno's convention, the program's name
/// and the window's role. `format!` rather than a `&'static str` because the
/// role comes from a module or a tool id.
pub(in crate::app) fn window_title(role: &str) -> String {
    format!("SDRuno {role}")
}

/// SDRuno's name for a module's window, used for the title and the WINDOWS
/// menu. `None` is the plain `DetachableModule::label`.
pub(in crate::app) fn module_role(module: DetachableModule) -> &'static str {
    use DetachableModule as M;
    match module {
        M::Panadapter => "MAIN SP",
        M::AuxPanadapter => "AUX SP",
        M::Controls => "RX CONTROL",
        M::Panel => "DECODER",
        M::BandMenu => "BAND",
        M::Rotor => "ROTATOR",
    }
}

/// The captioned group SDRuno's control pages are built from: a small caption
/// in the dim accent, and the controls under it inside a hairline box.
///
/// The whole difference between our strip and SDRuno's RX control is this
/// shape. Ours are chips in rows with their state read off the label; SDRuno
/// labels the *group* and lets the buttons be tiny.
///
/// **The box is not stretched to the row's width, and must not be.**
/// `ui.set_min_width(ui.available_width())` here read like the way to make each
/// group fill its share, and inside a `horizontal_wrapped` row it silently ate
/// every sibling after the first: the render test found FILTER drawn and AGC,
/// STEP and NR absent. A wrapped row and a child that demands the whole row do
/// not mix. The groups size to their content, which is what the caption wants
/// anyway.
fn group<R>(ui: &mut egui::Ui, caption: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, theme::LINE()))
        .corner_radius(egui::CornerRadius::same(2))
        .inner_margin(egui::Margin::symmetric(6, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(caption).size(10.0).strong().color(theme::CYAN_DIM()));
            ui.add_space(3.0);
            add(ui)
        })
        .inner
}

/// One SDRuno function button: the vertical strip down the left of MAIN.
///
/// Returns whether it was pressed. A lit button is one whose window is already
/// out, which is the state SDRuno shows and the only way to tell at a glance
/// what is open.
fn function_button(ui: &mut egui::Ui, label: &str, lit: bool) -> bool {
    let w = ui.available_width().min(150.0).max(96.0);
    crate::chrome::chip_sized(
        ui,
        lit,
        RichText::new(label).size(12.0).strong(),
        egui::vec2(w, 22.0),
    )
    .clicked()
}

impl super::SdroxideApp {
    /// Open the workstation's window set on the **rising edge** of the mode.
    ///
    /// Once, not every frame: the point of the arrangement is that the operator
    /// then moves the windows where they want them, and a rule re-asserted each
    /// frame would drag a window the operator had deliberately docked back out.
    /// Turning the mode off changes nothing — the windows stay where they are,
    /// so the arrangement is not lost by trying the look on.
    pub(in crate::app) fn apply_sdruno_arrangement(&mut self) {
        if self.applied_sdruno == self.ui_settings.sdruno {
            return;
        }
        self.applied_sdruno = self.ui_settings.sdruno;
        if !self.sdruno_active() {
            return;
        }
        // SDRuno's desk: the spectrum (MAIN SP), the operating panel (the
        // decoder window) and the control surface (RX CONTROL) each in a window
        // of their own, which leaves the main window as MAIN — the hub. The
        // band panel is not undocked: in SDRuno it is part of RX CONTROL, and
        // that page carries it.
        for m in [DetachableModule::Panadapter, DetachableModule::Panel, DetachableModule::Controls]
        {
            self.ui_settings.set_detached(m, true);
        }
        crate::app::persist::persist_ui_settings(&self.ui_settings);
    }

    /// **SDRuno MAIN** — the hub, drawn where the workspace would otherwise be
    /// empty.
    ///
    /// SDRuno's MAIN is not the program's whole workspace; it is a small
    /// surface whose buttons open the rest, with the band keypad beside it.
    /// When every module of ours is in a window of its own the main window has
    /// exactly that role, so this is what it draws.
    pub(in crate::app) fn sdruno_main_hub(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
        ui.horizontal_top(|ui| {
            // The function buttons, down the left — SDRuno's OPT/SCAN/… strip.
            ui.vertical(|ui| {
                let cfg = self.ui_settings;
                let on = |m: DetachableModule| cfg.is_detached(m);
                ui.label(RichText::new("WINDOWS").size(10.0).strong().color(theme::CYAN_DIM()));
                let mut set: Option<(DetachableModule, bool)> = None;
                for m in DetachableModule::ALL {
                    if function_button(ui, module_role(m), on(m)) {
                        set = Some((m, !on(m)));
                    }
                }
                if let Some((m, want)) = set {
                    self.set_module_detached(m, want);
                }
                ui.add_space(8.0);
                ui.label(RichText::new("TOOLS").size(10.0).strong().color(theme::CYAN_DIM()));
                for (label, open) in [
                    ("SCANNER", &mut self.show_scanner),
                    ("SPOTS", &mut self.show_spots),
                    ("MEM PANEL", &mut self.show_memories),
                    ("LOGBOOK", &mut self.show_logbook),
                    ("BANDS", &mut self.show_bands),
                    ("SETT.", &mut self.show_settings),
                ] {
                    if function_button(ui, label, *open) {
                        *open = !*open;
                    }
                }
            });
            ui.separator();
            // The band keypad and the band/mode selector, side by side — the
            // same pair the console draws, which is SDRuno's RX control page
            // reached from MAIN.
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .id_salt("sdruno-main-hub-scroll")
                .show(ui, |ui| {
                    self.console_band_area(ui, cmds);
                });
        });
    }

    /// **SDRuno RX CONTROL** — the dense control page.
    ///
    /// Drawn under the ordinary strip inside the RX CONTROL window. The strip
    /// stays because it is the working surface — the frequency readout, the
    /// PTT, volume and squelch are on it and re-implementing them here would
    /// be a second copy to keep in step. What this adds is the half SDRuno
    /// shows from the first pixel and we hide in menus: every mode, every filter
    /// the mode is worked at, every AGC setting, every tuning step.
    pub(in crate::app) fn sdruno_rx_control(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let mode = self.state.rx[0].mode;
        let band = self.state.band;
        ui.spacing_mut().item_spacing = egui::vec2(5.0, 4.0);

        // MODE and DIGITAL as two captioned matrices — SDRuno's MODE row and
        // its DIGITAL button, but with the digital modes *visible* rather than
        // behind a dropdown, because a window can be as wide as this one.
        group(ui, "MODE", |ui| {
            super::top_bar::mode_chip_grid(
                ui,
                mode,
                &super::top_bar::OPERATE_MODES,
                band,
                &self.state,
                false,
                cmds,
            );
        });
        group(ui, "DIGITAL", |ui| {
            super::top_bar::mode_chip_grid(
                ui,
                mode,
                &sdroxide_types::Mode::DIGITAL,
                band,
                &self.state,
                false,
                cmds,
            );
        });

        ui.horizontal_wrapped(|ui| {
            // FILTER: the presets the mode is actually worked at, one press
            // each. The same table the strip's BW popup offers, without the
            // popup.
            let presets = mode.filter_presets();
            if !presets.is_empty() {
                ui.vertical(|ui| {
                    group(ui, "FILTER", |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for &(label, plo, phi) in presets {
                                let (plo, phi) = super::top_bar::preset_edges(
                                    mode,
                                    plo,
                                    phi,
                                    self.cw_pitch_hz(),
                                );
                                let rx0 = self.state.rx[0];
                                let on = (rx0.filter_lo - plo).abs() < 1.0
                                    && (rx0.filter_hi - phi).abs() < 1.0;
                                if crate::chrome::chip(ui, on, label).clicked() {
                                    self.set_rx_filter(plo, phi, cmds);
                                }
                            }
                        });
                    });
                });
            }

            // AGC: four settings, one press each.
            ui.vertical(|ui| {
                group(ui, "AGC", |ui| {
                    ui.horizontal(|ui| {
                        let cur = self.state.rx[0].agc;
                        for a in AgcMode::ALL {
                            if crate::chrome::chip(ui, a == cur, a.label()).clicked() {
                                cmds.push(Command::SetAgc { rx: RxId::Main, agc: a });
                            }
                        }
                    });
                });
            });

            // STEP: the tuning steps, one press each.
            ui.vertical(|ui| {
                group(ui, "STEP", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let cur = self.ui_settings.tune_step_hz;
                        for step in sdroxide_types::UiSettings::TUNE_STEPS_HZ {
                            let label = step_label(step);
                            let on = (cur - step).abs() < 0.5;
                            if crate::chrome::chip(ui, on, label).clicked() {
                                self.ui_settings.tune_step_hz = step;
                                crate::app::persist::persist_ui_settings(&self.ui_settings);
                            }
                        }
                    });
                });
            });

            // NR / NB / AN: the noise controls, visible rather than in the RX
            // menu. NR cycles its engines the way the strip's chip does.
            ui.vertical(|ui| {
                group(ui, "NR - NB - AN", |ui| {
                    ui.horizontal(|ui| {
                        let nr = self.state.rx[0].noise_reduction;
                        // Off, then the three strengths of whatever engine is
                        // chosen — a single row cannot usefully walk all
                        // sixteen variants, and the engine is a considered
                        // choice made once. Strength `s` maps through
                        // `with_strength`, so the matrix sets the level and
                        // leaves the engine alone.
                        for (label, strength) in [
                            ("NR OFF", None),
                            ("NR 1", Some(sdroxide_types::NrStrength::Low)),
                            ("NR 2", Some(sdroxide_types::NrStrength::Med)),
                            ("NR 3", Some(sdroxide_types::NrStrength::High)),
                        ] {
                            let on = match strength {
                                None => !nr.is_on(),
                                Some(s) => nr.is_on() && nr.strength() == Some(s),
                            };
                            if crate::chrome::chip(ui, on, label).clicked() {
                                let level = match strength {
                                    None => NrLevel::Off,
                                    Some(s) => nr.with_strength(s),
                                };
                                cmds.push(Command::SetNoiseReduction { rx: RxId::Main, level });
                            }
                        }
                        ui.separator();
                        let nb = self.state.noise_blanker;
                        if crate::chrome::chip(ui, nb, "NB").clicked() {
                            cmds.push(Command::SetNoiseBlanker(!nb));
                        }
                        let an = self.state.rx[0].auto_notch;
                        if crate::chrome::chip(ui, an, "AN").clicked() {
                            cmds.push(Command::SetAutoNotch { rx: RxId::Main, on: !an });
                        }
                    });
                });
            });
        });

        // BANDS: the band keypad, which is this program's own SDRuno band
        // panel, and the band list beside it.
        group(ui, "BANDS - MHz", |ui| {
            self.console_band_area(ui, cmds);
        });
    }
}

/// A tuning step as SDRuno labels them: kHz where it is whole, Hz below that.
fn step_label(hz: f64) -> String {
    if hz >= 1000.0 {
        let k = hz / 1000.0;
        if (k - k.round()).abs() < 1e-6 {
            format!("{}K", k.round() as i64)
        } else {
            format!("{k}K")
        }
    } else {
        format!("{}", hz as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_module_gets_an_sdruno_role_and_no_two_the_same() {
        let mut seen = std::collections::HashSet::new();
        for m in DetachableModule::ALL {
            let role = module_role(m);
            assert!(!role.is_empty(), "{m:?} has no role");
            assert!(seen.insert(role), "{role} is two modules' role");
        }
    }

    #[test]
    fn a_window_title_names_the_program_and_the_role() {
        assert_eq!(window_title("MAIN SP"), "SDRuno MAIN SP");
        assert_eq!(window_title("RX CONTROL"), "SDRuno RX CONTROL");
    }

    #[test]
    fn the_float_hint_is_only_offered_in_the_mode() {
        set_active(false);
        assert_eq!(float_hint(), None, "an ordinary window must not be floated");
        set_active(true);
        assert_eq!(
            float_hint(),
            Some(egui::X11WindowType::Utility),
            "the workstation's windows ask the WM to float them"
        );
        set_active(false);
    }

    #[test]
    fn the_mode_is_unavailable_in_a_browser_and_off_a_desktop() {
        assert!(!available(crate::layout::Tier::Phone), "a phone has no desk");
        #[cfg(not(target_arch = "wasm32"))]
        {
            assert!(available(crate::layout::Tier::Desktop));
            assert!(
                available(crate::layout::Tier::Tablet),
                "a tablet runs the installed program and has windows to place"
            );
        }
    }

    #[test]
    fn steps_read_as_sdruno_writes_them() {
        assert_eq!(step_label(10.0), "10");
        assert_eq!(step_label(1_000.0), "1K");
        assert_eq!(step_label(12_500.0), "12.5K");
    }
}
