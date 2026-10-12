//! The DAB / DAB+ panel: the ensemble, and the services it carries.
//!
//! A service list is the whole of what an operator does with DAB — tune a
//! channel, see what is on it, pick a station — so that is the panel. Received
//! audio plays through the ordinary speaker path, so there is no waveform to
//! draw here.
//!
//! The header says what the receiver is doing, for the reason the ADS-B panel's
//! does: an empty list has several quite different causes — a quiet channel, a
//! receiver not tuned to Band III, a stream too narrow to hold an ensemble — and
//! only one of them is anything to do with the decoder.

use eframe::egui::{self, RichText};
use sdroxide_types::{Command, DabService, DabStatus};

/// Is the **CLEAR** chip offered at all?
///
/// Split out so the rule can be pinned without a click: a chip drawn with
/// nothing to clear, or drawn during a sweep where stopping would write the
/// sweep's findings straight back over the clearing, is the inert control this
/// fork keeps having to unpick. Both cases return false, so the chip is simply
/// not there rather than sitting there doing nothing.
fn dab_clear_offered(scanning: bool, found_len: usize) -> bool {
    !scanning && found_len > 0
}

/// A Band III frequency as the panel shows it: MHz to the kHz, `197.648`.
///
/// One decimal rounded every block to the nearest 100 kHz, so 8B read `197.6`
/// where the published frequency — and the one a DAB radio shows — is
/// 197.648 MHz, and the operator had nothing to check the dial against.
fn dab_mhz(hz: f64) -> String {
    format!("{:.3}", hz / 1e6)
}

use crate::app::SdroxideApp;
use crate::theme;

/// The width at which the pane is split three ways.
///
/// Below it the columns stack instead, because a service label and a channel
/// chip sharing a third of a phone's width each is a column of clipped text
/// and a row of nothing — the two lists are the whole panel, and they are both
/// worth more full width than a third of a wide one.
const DAB_SPLIT_WIDTH: f32 = 560.0;

/// The four columns — CHANNELS, STATIONS, STATION INFO and the VU meter — sit
/// together on the left and take **at most half** the panel's width, so the
/// right half stays free. Never narrower than this, though, unless the panel
/// itself is: four columns in less than this are clipped text.
const DAB_GROUP_MIN: f32 = 600.0;

/// How the grouped columns are laid out for a panel `width` wide: the group's
/// width, the VU meter's width, and whether the meter is the compact one.
///
/// Half the panel when that holds the full meter. When it does not, the meter
/// narrows and loses its plate first; only then does the group take more than
/// half, and never less than [`DAB_GROUP_MIN`] less what the narrow meter saved.
pub(in crate::app) fn dab_group_layout(width: f32) -> (f32, f32, bool) {
    use crate::widgets::vu_meter::{WIDTH, WIDTH_COMPACT};
    let half = width * 0.5;
    if half >= DAB_GROUP_MIN {
        (half, WIDTH, false)
    } else {
        let floor = (DAB_GROUP_MIN - (WIDTH - WIDTH_COMPACT)).min(width);
        (half.max(floor), WIDTH_COMPACT, true)
    }
}

/// The widths of CHANNELS, STATIONS and STATION INFO inside a group `group`
/// wide, after the VU meter (`vu_w`) and the gaps have taken theirs.
pub(in crate::app) fn dab_column_widths(group: f32, vu_w: f32) -> [f32; 3] {
    let rest = (group - vu_w - 3.0 * DAB_COL_GAP).max(0.0);
    [rest * 0.28, rest * 0.40, rest * 0.32]
}

const DAB_COL_GAP: f32 = 10.0;

/// The status describes `channel` — the lane's window sits on that block —
/// and not the one the lane has just left. A sweep retunes every few seconds
/// and the first status after a retune is still the old block's, so without
/// this a station heard on 8A would be remembered on 8B.
pub(in crate::app) fn dab_status_is_for(st: &DabStatus, channel: &str) -> bool {
    sdroxide_types::DAB_BAND_III
        .iter()
        .find(|(n, _)| *n == channel)
        .is_some_and(|(_, hz)| (st.window_center_hz - hz).abs() < 1_000.0)
}

/// The stations the STATIONS column lists for `channel`: the ones the ensemble
/// is naming now, then the ones remembered from before that it has not named
/// yet — so a channel picked from the list shows its stations at once. Each
/// once, by service id.
pub(in crate::app) fn dab_station_rows(
    live: &[DabService],
    remembered: &[sdroxide_types::DabKnownStation],
    channel: &str,
    hidden: &[(String, String)],
) -> Vec<DabService> {
    let gone = |id: &str| hidden.iter().any(|(c, s)| c == channel && s == id);
    let mut out: Vec<DabService> = live.iter().filter(|s| !gone(&s.service_id)).cloned().collect();
    for k in remembered.iter().filter(|k| k.channel == channel) {
        if !gone(&k.service_id) && !out.iter().any(|s| s.service_id == k.service_id) {
            out.push(DabService {
                service_id: k.service_id.clone(),
                label: k.label.clone(),
                subchannel: None,
                bitrate: None,
                protection: None,
            });
        }
    }
    out
}

/// The channels the picker offers: what a running scan has found *so far*,
/// then what is already remembered, and a couple of common blocks before the
/// first scan ever runs.
///
/// A sweep used to hand its findings over only when it finished, so an operator
/// watching a chip that said SCANNING… watched an unchanged list and reasonably
/// concluded the scan was doing nothing. The live findings lead because they are
/// the freshest answer to the only question being asked — what is on the air
/// here — and the remembered list follows so a partial sweep is never a
/// downgrade of what was already known.
pub(in crate::app) fn dab_channel_choices(
    remembered: &[String],
    scanning: Option<&[String]>,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for name in scanning.unwrap_or_default().iter().chain(remembered.iter()) {
        if !out.iter().any(|n| n == name) {
            out.push(name.clone());
        }
    }
    if out.is_empty() {
        out.extend(["8B", "11C", "12B", "12C"].iter().map(|s| s.to_string()));
    }
    out
}

/// Whether the pane is split into its three columns at this width, or the two
/// lists stacked full width.
pub(in crate::app) fn dab_is_split(width: f32) -> bool {
    width >= DAB_SPLIT_WIDTH
}

/// The service the right-hand column describes: the one actually playing, or —
/// before the audio has started, or after it has stopped — the one the operator
/// last chose.
///
/// "Playing" wins over "chosen" deliberately. A click sets the choice and the
/// audio follows a moment later, and a panel that described the choice would
/// then contradict what the operator is hearing; worse, once a station is
/// playing and the choice is changed, the *old* choice would be the one
/// described while the *new* one is the one heard.
pub(in crate::app) fn dab_focus_service<'a>(
    st: &'a DabStatus,
    chosen: &str,
) -> Option<&'a DabService> {
    let id = st.playing.as_deref().unwrap_or(chosen);
    if id.is_empty() {
        return None;
    }
    st.services.iter().find(|s| s.service_id == id)
}

/// Whether a block is carrying a multiplex, and so worth offering as a channel.
///
/// Services are the test, not the ensemble's name. A multiplex whose FIC
/// carries services but no ensemble label is a station the operator can plainly
/// see, and a sweep that required a name walked past it — the reported "the scan
/// found a station on a channel yet did not put the channel in the list".
pub(in crate::app) fn dab_block_has_multiplex(st: &DabStatus) -> bool {
    st.ensemble.is_some() || !st.services.is_empty()
}

/// The technical description of a service, as one line of `id · bitrate ·
/// protection · sub` with whatever the FIC actually named.
pub(in crate::app) fn dab_service_extra(s: &DabService) -> String {
    let mut extra = s.service_id.clone();
    if let Some(b) = s.bitrate {
        extra.push_str(&format!(" · {b} kbps"));
    }
    if let Some(p) = &s.protection {
        extra.push_str(&format!(" · {p}"));
    }
    if let Some(c) = s.subchannel {
        extra.push_str(&format!(" · sub {c}"));
    }
    extra
}

impl SdroxideApp {
    pub(in crate::app) fn dab_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let st: DabStatus = match self.dab_status.as_ref() {
            Some(s) => (**s).clone(),
            None => {
                ui.label(RichText::new("starting the DAB receiver…").weak());
                return;
            }
        };

        // Where the panel ends, captured *before* the lines above are drawn so
        // the lists get what is left of it rather than all of it measured from
        // part-way down. The arithmetic this replaces was
        // `panel_h - cursor + cursor`, which collapses to the whole `panel_h`
        // on every frame: the lists were laid out taller than the space left
        // for them, ran past the bottom of the panel, and a list that runs past
        // the bottom has no working scrollbar — the operator saw a cut-off list
        // and a wheel that did nothing. This is the shape `adsb_panel` uses.
        let content_bottom = ui.cursor().top() + panel_h - 26.0;

        // What, and where. Above the columns rather than inside any of them,
        // because it is the same whichever way the pane is split and it is the
        // line that answers "am I listening to the thing I meant to".
        ui.horizontal_wrapped(|ui| {
            if let Some(e) = &st.ensemble {
                ui.label(RichText::new(e).strong());
            } else if st.unavailable.is_none() {
                ui.label(
                    RichText::new("no ensemble named yet — is a DAB transmission on this channel?")
                        .weak(),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} MHz · {} frames · {} fibs",
                        dab_mhz(st.window_center_hz),
                        st.frames,
                        st.fibs
                    ))
                    .size(10.0)
                    .color(theme::gray(140)),
                );
            });
        });
        if let Some(why) = &st.unavailable {
            ui.label(RichText::new(why).color(theme::ALERT()).size(11.0));
        }
        // The width warning is **not here**: it was advice the operator had to
        // hunt for in the panel, and it now lives in the radio tab beside the
        // rate it is about, where it can be acted on. See
        // [`crate::app::settings::mod`]'s radio tab and [`crate::chrome::advisory`].
        ui.add_space(4.0);

        let avail_h = (content_bottom - ui.cursor().top()).max(80.0);
        let width = ui.available_width();
        ui.allocate_ui_with_layout(
            egui::vec2(width, avail_h.max(80.0)),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                if dab_is_split(width) {
                    // Grouped on the left, at most half the panel: CHANNELS,
                    // STATIONS, STATION INFO and the VU meter, each a fixed
                    // width, and the right half left free.
                    let (group, vu_w, compact) = dab_group_layout(width);
                    let [w_ch, w_st, w_info] = dab_column_widths(group, vu_w);
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = DAB_COL_GAP;
                        let col = |ui: &mut egui::Ui, w: f32, f: &mut dyn FnMut(&mut egui::Ui)| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(w, avail_h),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.set_width(w);
                                    f(ui)
                                },
                            );
                        };
                        col(ui, w_ch, &mut |ui| self.dab_channels(ui, cmds, &st, avail_h - 60.0));
                        col(ui, w_st, &mut |ui| self.dab_services(ui, cmds, &st, avail_h - 20.0));
                        col(ui, w_info, &mut |ui| self.dab_info(ui, &st));
                        self.dab_vu_meter(ui, &st, avail_h, vu_w, compact);
                    });
                } else {
                    // Stacked, the two lists share what is left of the panel:
                    // each asking for all of it would put the second one past
                    // the bottom and lose its scrollbar. And the right-hand
                    // columns are dropped rather than given a sliver of a
                    // phone: the info is on the service's own row, and a meter
                    // a finger wide says nothing.
                    let half = (avail_h / 2.0 - 4.0).max(60.0);
                    self.dab_channels(ui, cmds, &st, half);
                    self.dab_services(ui, cmds, &st, half);
                }
            },
        );
    }

    /// The VU meter column: the decoded audio's level, left and right.
    fn dab_vu_meter(
        &mut self,
        ui: &mut egui::Ui,
        st: &DabStatus,
        avail_h: f32,
        vu_w: f32,
        compact: bool,
    ) {
        let now = ui.input(|i| i.time);
        let dt = self.dab_vu_at.map_or(0.0, |t| (now - t) as f32);
        self.dab_vu_at = Some(now);
        let input = st.playing.as_ref().and(st.audio_rms_db);
        self.dab_vu.step(input, dt);
        if input.is_some() || self.dab_vu.is_live() {
            // The meter moves between status messages, so it asks for its own
            // frames while there is anything to move.
            crate::repaint::after_ms(ui.ctx(), 33);
        }
        let h = avail_h.clamp(160.0, 420.0);
        crate::widgets::vu_meter::stereo(ui, &self.dab_vu, egui::vec2(vu_w, h), compact)
            .on_hover_text(
                "The level of the station playing, left and right, before the volume \
             control. 0 VU is −18 dBFS; the red segments are the loud end.",
            );
    }

    /// Left column: the blocks this scan found, and the sweep that finds them.
    ///
    /// `list_h` is the height the list is allowed, so the scroll area is bounded
    /// by its own rect rather than by whatever height happens to be left over —
    /// which is what gives it a working scrollbar.
    fn dab_channels(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        st: &DabStatus,
        list_h: f32,
    ) {
        ui.label(RichText::new("CHANNELS").size(10.0).color(theme::CYAN_DIM()));

        // The sweep, and a toggle. Sweeping every Band III block is how an
        // operator learns which carry a transmission where they are; the ones
        // that do become the list below. While it runs the chip is the way out,
        // so it has to be live — it used to carry `&& !scanning`, which made a
        // button reading SCANNING… inert.
        let scanning = self.dab_scan.is_some();
        if crate::chrome::chip(ui, scanning, if scanning { "SCANNING…" } else { "SCAN" })
            .on_hover_text(if scanning {
                "Stop the sweep here. The channels it has already found stay in the list."
            } else {
                "Walk every Band III block and keep the ones that carry an ensemble. \
                 What is found becomes the list below, and is remembered."
            })
            .clicked()
        {
            if let Some(scan) = self.dab_scan.take() {
                // A sweep stopped half way has still learned something, and
                // discarding it would punish the operator for stopping early.
                // Keep it and remember it, exactly as finishing does.
                self.state.dab.found = scan.found;
                cmds.push(Command::SetDabConfig(self.state.dab.clone()));
            } else {
                self.dab_scan =
                    Some(crate::app::frame::DabScan { at: 0, since: None, found: Vec::new() });
                // A station cleared with ✕ comes back with the next sweep.
                self.dab_hidden.clear();
                // Start the sweep on the first block at once.
                self.state.dab.channel = sdroxide_types::DAB_BAND_III[0].0.to_string();
                cmds.push(Command::SetVfo {
                    vfo: self.state.active_vfo,
                    hz: sdroxide_types::DAB_BAND_III[0].1,
                });
                cmds.push(Command::SetDabConfig(self.state.dab.clone()));
            }
        }

        // **Clear the list**, asked for on the fork's #18 as "Add clear list".
        //
        // The CHANNELS list is what the sweep found and it is *remembered*, so
        // it only ever grows: every ensemble the receiver has ever come across
        // stays, with nothing to remove one. That is the right default for a
        // listener — re-scanning costs minutes — and the wrong one with no way
        // out, so this is the way out.
        //
        // Two rules, both so the chip can never be the inert thing this fork
        // keeps finding: it is **only offered when there is something to clear**,
        // and it is **not offered during a sweep**, where stopping would write
        // the sweep's findings straight back over the clearing. Pressing SCAN
        // finds them again, which is why this needs no confirmation.
        if dab_clear_offered(scanning, self.state.dab.found.len())
            && crate::chrome::chip(ui, false, "CLEAR")
                .on_hover_text("Forget the channels found so far. SCAN will find them again.")
                .clicked()
        {
            self.state.dab.found.clear();
            cmds.push(Command::SetDabConfig(self.state.dab.clone()));
            self.client_settings_status =
                Some("channel list cleared — SCAN will find them again".into());
        }

        // Where the sweep is, so a two-minute walk is not a silent one.
        if let Some(scan) = self.dab_scan.as_ref() {
            let n = sdroxide_types::DAB_BAND_III.len();
            let at = scan.at.min(n - 1);
            ui.add(
                egui::ProgressBar::new((at + 1) as f32 / n as f32).desired_height(12.0).text(
                    RichText::new(format!(
                        "{} · {}/{n} · {} found",
                        sdroxide_types::DAB_BAND_III[at].0,
                        at + 1,
                        scan.found.len()
                    ))
                    .size(9.0),
                ),
            );
        }

        let channels = dab_channel_choices(
            &self.state.dab.found,
            self.dab_scan.as_ref().map(|s| s.found.as_slice()),
        );
        egui::ScrollArea::vertical()
            .id_salt("dab_channels_scroll")
            .auto_shrink([false, false])
            .max_height(list_h)
            .min_scrolled_height(list_h)
            .show(ui, |ui| {
                for name in &channels {
                    let here = self.state.dab.channel == *name;
                    let freq = sdroxide_types::DAB_BAND_III
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, hz)| *hz);
                    let label = match freq {
                        Some(hz) => format!("{name}  {}", dab_mhz(hz)),
                        None => name.clone(),
                    };
                    ui.horizontal(|ui| {
                        if !scanning
                            && ui
                                .small_button(RichText::new("×").size(10.0))
                                .on_hover_text(
                                    "Clear this channel and the stations remembered on it. \
                                     SCAN finds it again.",
                                )
                                .clicked()
                        {
                            self.state.dab.forget_channel(name);
                            cmds.push(Command::SetDabConfig(self.state.dab.clone()));
                        }
                        let r = crate::chrome::chip(ui, here, label).on_hover_text(
                            "Tune this block. A new channel carries a different service \
                             list, so the station chosen is dropped with it.",
                        );
                        if r.clicked() {
                            // Picking a channel is the operator saying "stop
                            // here". A sweep left running retunes away on its
                            // very next tick (~100 ms), so the pick snapped
                            // straight back and read as a dead control — the one
                            // thing a channel pick must never do. This was the
                            // reported bug: no way out of a scan but waiting out
                            // all 38 blocks.
                            self.dab_scan = None;
                            self.state.dab.channel = name.clone();
                            // The service id belonged to the ensemble being
                            // left. Carried across it is not a harmless default:
                            // the receiver holds a request its FIC can never
                            // resolve, and the channel would sit silent with a
                            // station lit that does not exist here.
                            self.state.dab.service_id.clear();
                            cmds.push(Command::SetDabConfig(self.state.dab.clone()));
                            // Put the dial on the channel, as the ADS-B lane
                            // does with 1090 MHz: the decoder is fed from a
                            // window there, and an operator who picked a channel
                            // should see the receiver move to it rather than
                            // have to tune twice.
                            if let Some(hz) = freq {
                                cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz });
                            }
                        }
                    });
                }
            });

        // What the receiver is doing, in the column that owns the choice.
        if let Some(why) = st.degraded.as_ref().filter(|_| st.services.is_empty()) {
            ui.label(RichText::new(why).color(theme::YELLOW()).size(10.0));
        }
    }

    /// Middle column: the ensemble's services, the one playing marked, each a
    /// press to listen.
    fn dab_services(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        st: &DabStatus,
        list_h: f32,
    ) {
        ui.label(RichText::new("STATIONS").size(10.0).color(theme::CYAN_DIM()));
        let channel = self.state.dab.channel.clone();
        // What is named now leads; what was heard here before follows, so a
        // channel picked from the list shows its stations at once.
        let live: &[DabService] = if dab_status_is_for(st, &channel) { &st.services } else { &[] };
        let rows = dab_station_rows(live, &self.state.dab.stations, &channel, &self.dab_hidden);
        if rows.is_empty() {
            ui.add_space(2.0);
            ui.label(
                RichText::new(if st.unavailable.is_some() {
                    "the receiver cannot hold a DAB ensemble here"
                } else {
                    "nothing decoded yet — the FIC takes a moment after the channel is in tune"
                })
                .size(10.0)
                .weak(),
            );
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("dab_services_scroll")
            .auto_shrink([false, false])
            .max_height(list_h)
            .min_scrolled_height(list_h)
            .show(ui, |ui| {
                for s in &rows {
                    let playing = st.playing.as_deref() == Some(s.service_id.as_str());
                    let heard_now = live.iter().any(|l| l.service_id == s.service_id);
                    let label = if s.label.trim().is_empty() {
                        format!("({})", s.service_id)
                    } else {
                        s.label.clone()
                    };
                    ui.horizontal(|ui| {
                        if ui
                            .small_button(RichText::new("×").size(10.0))
                            .on_hover_text(
                                "Clear this station from the list. The next SCAN brings it \
                                 back if the ensemble still carries it.",
                            )
                            .clicked()
                        {
                            self.state.dab.forget_station(&channel, &s.service_id);
                            self.dab_hidden.push((channel.clone(), s.service_id.clone()));
                            cmds.push(Command::SetDabConfig(self.state.dab.clone()));
                        }
                        if crate::chrome::chip(ui, playing, label).clicked() {
                            self.state.dab.service_id = s.service_id.clone();
                            cmds.push(Command::SetDabConfig(self.state.dab.clone()));
                        }
                    });
                    ui.label(
                        RichText::new(if heard_now {
                            dab_service_extra(s)
                        } else {
                            format!("{} · remembered", s.service_id)
                        })
                        .size(9.0)
                        .color(theme::gray(140)),
                    );
                }
            });
    }

    /// Right column: the chosen station's own numbers, and the decoder's state.
    ///
    /// A service list says *what* is on the air and not one of its numbers, so
    /// the numbers a listener would otherwise have to look up — the programme's
    /// bitrate, its protection profile, which sub-channel it rides in — are
    /// given a home of their own instead of being the small grey afterthought
    /// under a list that is also the only way to play something.
    fn dab_info(&self, ui: &mut egui::Ui, st: &DabStatus) {
        ui.label(RichText::new("STATION INFO").size(10.0).color(theme::CYAN_DIM()));
        let Some(s) = dab_focus_service(st, &self.state.dab.service_id) else {
            ui.add_space(2.0);
            ui.label(
                RichText::new(if st.services.is_empty() {
                    "no station yet"
                } else {
                    "no station chosen — press one in the list"
                })
                .size(10.0)
                .weak(),
            );
            return;
        };

        ui.add_space(2.0);
        let name =
            if s.label.trim().is_empty() { format!("({})", s.service_id) } else { s.label.clone() };
        ui.label(RichText::new(name).strong());

        let playing = st.playing.as_deref() == Some(s.service_id.as_str());
        ui.label(
            RichText::new(if playing { "▶ playing" } else { "chosen" })
                .size(10.0)
                .color(if playing { theme::CYAN_DIM() } else { theme::gray(140) }),
        );

        ui.add_space(4.0);
        let row = |ui: &mut egui::Ui, k: &str, v: String| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("{k}:")).size(10.0).color(theme::gray(140)));
                ui.label(RichText::new(v).size(10.0));
            });
        };
        row(ui, "Service", s.service_id.clone());
        if let Some(b) = s.bitrate {
            row(ui, "Bitrate", format!("{b} kbps"));
        }
        if let Some(p) = &s.protection {
            row(ui, "Protection", p.clone());
        }
        if let Some(c) = s.subchannel {
            row(ui, "Sub-channel", c.to_string());
        }
        if let Some(e) = &st.ensemble {
            row(ui, "Ensemble", e.clone());
        }
        row(ui, "Channel", self.state.dab.channel.clone());
        row(ui, "Decoder", format!("{} frames · {} fibs", st.frames, st.fibs));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        dab_block_has_multiplex, dab_channel_choices, dab_focus_service, dab_is_split, dab_mhz,
        dab_service_extra,
    };
    use sdroxide_types::{DabService, DabStatus};

    /// Kevin's report: 8B read `197.6` where the block is 197.648 MHz.
    #[test]
    fn a_block_shows_its_frequency_to_the_khz() {
        let hz = sdroxide_types::DAB_BAND_III.iter().find(|(n, _)| *n == "8B").unwrap().1;
        assert_eq!(dab_mhz(hz), "197.648");
        assert_eq!(dab_mhz(174_928_000.0), "174.928");
    }

    fn v(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn svc(id: &str) -> DabService {
        DabService {
            service_id: id.to_string(),
            label: format!("Station {id}"),
            subchannel: None,
            bitrate: None,
            protection: None,
        }
    }

    fn status(services: Vec<DabService>, playing: Option<&str>) -> DabStatus {
        DabStatus { services, playing: playing.map(|s| s.to_string()), ..Default::default() }
    }

    /// The named-ensemble case, which is the one the sweep used to require.
    fn named_ensemble(label: &str) -> DabStatus {
        DabStatus { ensemble: Some(label.to_string()), ..Default::default() }
    }

    /// The reported symptom: a sweep filled the picker only when it *finished*,
    /// so the operator watched SCANNING… and an unchanged channel list and
    /// concluded the scan was dead. Its findings have to appear as it goes.
    #[test]
    fn a_running_scan_shows_what_it_has_found_so_far() {
        assert_eq!(dab_channel_choices(&[], Some(&v(&["7D", "8B"]))), v(&["7D", "8B"]));
    }

    /// The live findings are the fresh answer to "what is on the air here", so
    /// they lead; the remembered list still follows, and a block named by both
    /// appears once — a duplicate chip would offer the same channel twice.
    #[test]
    fn live_findings_lead_and_nothing_is_offered_twice() {
        assert_eq!(
            dab_channel_choices(&v(&["8B", "12C"]), Some(&v(&["7D", "8B"]))),
            v(&["7D", "8B", "12C"])
        );
    }

    /// No sweep running: the remembered list, and before the first scan a
    /// couple of common blocks so the picker is never an empty row.
    #[test]
    fn the_picker_is_never_empty() {
        assert_eq!(dab_channel_choices(&[], None), v(&["8B", "11C", "12B", "12C"]));
        assert_eq!(dab_channel_choices(&v(&["7D"]), None), v(&["7D"]));
    }

    /// The three-way split needs room for a service label and a channel chip at
    /// once; below that the two lists stack full width, which is what a phone
    /// can actually read. A third of a phone's width each is a column of
    /// clipped text and a row of nothing.
    #[test]
    fn the_pane_splits_only_when_three_columns_fit() {
        assert!(dab_is_split(900.0));
        assert!(dab_is_split(560.0));
        assert!(!dab_is_split(400.0));
        assert!(!dab_is_split(0.0));
    }

    /// The info column describes the station the operator is *hearing*. A click
    /// sets the choice and the audio follows a moment later, so describing the
    /// choice would contradict what is playing — and once one station is playing
    /// and another is chosen, the old choice is the stale one.
    #[test]
    fn the_info_column_describes_what_is_playing() {
        let st = status(vec![svc("0xF201"), svc("0xF202")], Some("0xF202"));
        assert_eq!(dab_focus_service(&st, "0xF201").map(|s| s.service_id.as_str()), Some("0xF202"));
    }

    /// Before the audio has started — the click that has not yet reached the
    /// receiver — the choice is the best answer there is. An empty channel
    /// names nothing rather than falling back to the first service, which would
    /// put a station on screen the operator never picked.
    #[test]
    fn before_anything_plays_the_column_follows_the_choice() {
        let st = status(vec![svc("0xF201"), svc("0xF202")], None);
        assert_eq!(dab_focus_service(&st, "0xF202").map(|s| s.service_id.as_str()), Some("0xF202"));
        assert!(dab_focus_service(&st, "").is_none());
        assert!(dab_focus_service(&status(vec![], None), "0xF201").is_none());
    }

    /// The numbers a listener would otherwise look up, and only the ones the FIC
    /// actually named — a line that reads "sub none" is a worse answer than a
    /// shorter one.
    #[test]
    fn the_info_line_carries_only_what_was_named() {
        let mut s = svc("0xF201");
        assert_eq!(dab_service_extra(&s), "0xF201");
        s.bitrate = Some(128);
        s.protection = Some("EEP 3-A".into());
        s.subchannel = Some(12);
        assert_eq!(dab_service_extra(&s), "0xF201 · 128 kbps · EEP 3-A · sub 12");
    }

    /// The reported bug: a sweep found a station on a block, showed the
    /// operator its services, and still never offered that channel. The test
    /// used to be an ensemble name, which a multiplex is not required to carry.
    #[test]
    fn a_block_carrying_stations_counts_as_found_without_a_ensemble_name() {
        let st = status(vec![svc("0xF201")], None);
        assert!(dab_block_has_multiplex(&st));
        assert!(dab_block_has_multiplex(&named_ensemble("Métropolitain 2")));
        // Nothing decoded at all: silence, or a block with something else on
        // it, must not become a channel chip.
        assert!(!dab_block_has_multiplex(&status(vec![], None)));
    }
}

#[cfg(test)]
mod clear_list_tests {
    use super::dab_clear_offered;

    /// The four columns take half the panel when the full meter fits there;
    /// below that the meter narrows and drops its plate before the group
    /// grows past half.
    #[test]
    fn the_columns_keep_to_half_the_panel_and_the_meter_narrows_first() {
        use super::dab_group_layout;
        use crate::widgets::vu_meter::{WIDTH, WIDTH_COMPACT};
        for w in [1200.0, 1600.0, 1920.0] {
            let (group, vu, compact) = dab_group_layout(w);
            assert_eq!(group, w * 0.5, "{w} pt: the group is half the panel");
            assert_eq!((vu, compact), (WIDTH, false), "{w} pt: the full meter fits");
        }
        for w in [700.0, 900.0, 1100.0] {
            let (group, vu, compact) = dab_group_layout(w);
            assert_eq!((vu, compact), (WIDTH_COMPACT, true), "{w} pt: the narrow meter");
            assert!(group <= w, "{w} pt: never wider than the panel");
        }
        assert_eq!(dab_group_layout(1100.0).0, 552.0, "the narrow meter's saving is kept");
    }

    /// Kevin's "Add clear list" on the fork's #18. The CHANNELS list is
    /// remembered and only ever grows, so it needs a way out — and the chip must
    /// not exist when it has nothing to do.
    #[test]
    fn the_clear_chip_is_offered_only_when_there_is_something_to_clear() {
        assert!(dab_clear_offered(false, 1), "one channel found: clear it");
        assert!(dab_clear_offered(false, 38), "a full sweep's worth");
        // Nothing to clear, so no chip — rather than a button that does nothing.
        assert!(!dab_clear_offered(false, 0), "an empty list needs no CLEAR");
    }

    /// During a sweep the chip must be absent: stopping writes `scan.found`
    /// straight into `state.dab.found`, so a clearing pressed mid-sweep would be
    /// silently undone a moment later.
    #[test]
    fn the_clear_chip_is_absent_while_a_sweep_is_running() {
        assert!(!dab_clear_offered(true, 12), "a clearing mid-sweep would be overwritten");
        assert!(!dab_clear_offered(true, 0));
    }
}
