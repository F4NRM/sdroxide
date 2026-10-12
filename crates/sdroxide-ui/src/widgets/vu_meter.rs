//! A stereo VU meter: two columns of segments, left and right, that move with
//! the programme. Drawn for the DAB panel, where the audio is decoded on the
//! station and only its level reaches the screen.
//!
//! Two halves, kept apart so the motion can be tested without drawing:
//! [`VuBallistics`] turns the level the engine reports into what the needle —
//! here the lit column — should show, and [`stereo`] paints it in the theme's
//! own colours.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use crate::theme;

/// The scale, top to bottom, in VU (dB relative to the alignment level).
/// One segment each; the two at the top are the red overload zone.
pub const SCALE_VU: [i32; 16] = [8, 6, 4, 2, 0, -2, -4, -6, -8, -12, -16, -20, -28, -40, -48, -60];

/// The red zone starts here: a segment at or above it is drawn in the alert
/// colour.
pub const RED_FROM_VU: i32 = 6;

/// 0 VU sits at −18 dBFS RMS — the EBU alignment level. Broadcast audio lives
/// a few dB either side of it, so the column moves through the middle of the
/// scale on music and only reaches the red on genuinely loud passages.
pub const ALIGN_DBFS: f32 = -18.0;

/// Below the bottom of the scale: the column is dark.
const FLOOR_VU: f32 = -70.0;

/// A VU meter's integration time: the reading takes about this long to reach
/// 99 % of a step, rising and falling alike.
const VU_TIME_S: f32 = 0.3;

/// How long the peak marker holds before it starts to fall, and how fast it
/// falls then.
const PEAK_HOLD_S: f32 = 1.5;
const PEAK_FALL_VU_PER_S: f32 = 20.0;

/// The meter's motion, left and right: the integrated level and the held peak,
/// both in VU.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VuBallistics {
    pub level: [f32; 2],
    pub peak: [f32; 2],
    peak_age: [f32; 2],
}

impl Default for VuBallistics {
    fn default() -> Self {
        VuBallistics { level: [FLOOR_VU; 2], peak: [FLOOR_VU; 2], peak_age: [0.0; 2] }
    }
}

impl VuBallistics {
    /// The dBFS RMS the engine reports, as VU.
    pub fn vu_of_dbfs(dbfs: f32) -> f32 {
        if dbfs.is_finite() { (dbfs - ALIGN_DBFS).max(FLOOR_VU) } else { FLOOR_VU }
    }

    /// Advance by `dt` seconds towards `input` (dBFS RMS, left and right).
    /// `None` means nothing is playing: the column falls away to dark.
    pub fn step(&mut self, input: Option<[f32; 2]>, dt: f32) {
        let dt = dt.clamp(0.0, 0.5);
        // A first-order integrator whose 99 % time is `VU_TIME_S`: ln(100) ≈ 4.6.
        let k = 1.0 - (-dt * 4.6 / VU_TIME_S).exp();
        for ch in 0..2 {
            let target = input.map_or(FLOOR_VU, |l| Self::vu_of_dbfs(l[ch]));
            self.level[ch] += (target - self.level[ch]) * k;
            if self.level[ch] >= self.peak[ch] {
                self.peak[ch] = self.level[ch];
                self.peak_age[ch] = 0.0;
            } else {
                self.peak_age[ch] += dt;
                if self.peak_age[ch] > PEAK_HOLD_S {
                    self.peak[ch] = (self.peak[ch] - PEAK_FALL_VU_PER_S * dt).max(self.level[ch]);
                }
            }
        }
    }

    /// Whether anything is still moving, so the caller knows to keep asking
    /// for frames.
    pub fn is_live(&self) -> bool {
        self.level.iter().chain(self.peak.iter()).any(|v| *v > FLOOR_VU + 0.5)
    }
}

/// How many segments, counted from the bottom, a level lights.
pub fn lit_segments(vu: f32) -> usize {
    SCALE_VU.iter().filter(|s| vu >= **s as f32).count()
}

/// The segment the peak marker sits on, counted from the bottom (1-based), or
/// 0 when the peak is below the scale.
fn peak_segment(vu: f32) -> usize {
    lit_segments(vu)
}

/// The column width the meter asks for: two bars, the scale between and beside
/// them, and a frame.
pub const WIDTH: f32 = 112.0;

/// The narrow form, for a panel too tight for [`WIDTH`]: the scale on the left
/// only, and no plate.
pub const WIDTH_COMPACT: f32 = 64.0;

/// The engraving on the plate under the bars.
pub const PLATE: &str = "Oxide Brown";

/// Paint a stereo meter into a box `size` big, in the theme's colours.
/// `compact` is the narrow form ([`WIDTH_COMPACT`]): one scale, no plate.
pub fn stereo(ui: &mut egui::Ui, m: &VuBallistics, size: Vec2, compact: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let p = ui.painter_at(rect);

    // The case: the theme's deep ground, framed like every other box.
    let case = rect.shrink(1.0);
    p.rect_filled(case, 6.0, theme::BG_DEEP());
    p.rect_stroke(case, 6.0, Stroke::new(1.0, theme::LINE_LIT()), egui::StrokeKind::Inside);

    let title_h = 18.0;
    let plate_h = if compact { 0.0 } else { 20.0 };
    let pad = 6.0;
    p.text(
        Pos2::new(case.center().x, case.top() + pad + title_h / 2.0 - 2.0),
        Align2::CENTER_CENTER,
        "VU",
        FontId::proportional(13.0),
        theme::TEXT_STRONG(),
    );

    // The bars and their scale.
    let body = Rect::from_min_max(
        Pos2::new(case.left() + pad, case.top() + pad + title_h),
        Pos2::new(
            case.right() - pad,
            case.bottom() - pad - plate_h - if compact { 12.0 } else { 18.0 },
        ),
    );
    let n = SCALE_VU.len();
    let pitch = body.height() / n as f32;
    let seg_h = (pitch * 0.72).max(2.0);
    let label_w = 22.0;
    let gap = 4.0;
    let labels = if compact { 1.0 } else { 2.0 };
    let bar_w = ((body.width() - labels * label_w - gap) / 2.0).max(6.0);
    let left_x = body.left() + label_w;
    let right_x = left_x + bar_w + gap;

    let lit = theme::CYAN();
    let red = theme::ALERT();
    let unlit = |c: Color32| c.gamma_multiply(0.14);
    let label_ink = theme::gray(150);

    let lit_n = [lit_segments(m.level[0]), lit_segments(m.level[1])];
    let peak_n = [peak_segment(m.peak[0]), peak_segment(m.peak[1])];

    for (i, vu) in SCALE_VU.iter().enumerate() {
        // `i` counts from the top; a segment is lit when the level reaches it,
        // which counted from the bottom is `n - i`.
        let from_bottom = n - i;
        let y = body.top() + pitch * i as f32 + (pitch - seg_h) / 2.0;
        let base = if *vu >= RED_FROM_VU { red } else { lit };
        for (ch, x) in [(0usize, left_x), (1usize, right_x)] {
            let seg = Rect::from_min_size(Pos2::new(x, y), Vec2::new(bar_w, seg_h));
            let on = from_bottom <= lit_n[ch];
            let is_peak = from_bottom == peak_n[ch] && peak_n[ch] > 0;
            if on || is_peak {
                // A soft glow behind the lit segment, then the segment itself.
                p.rect_filled(seg.expand(2.0), 3.0, base.gamma_multiply(0.25));
                p.rect_filled(seg, 2.0, base);
                p.rect_filled(
                    Rect::from_min_size(seg.min, Vec2::new(seg.width(), seg.height() * 0.35)),
                    2.0,
                    Color32::from_white_alpha(40),
                );
            } else {
                p.rect_filled(seg, 2.0, unlit(base));
            }
        }
        let text = if *vu > 0 { format!("+{vu}") } else { vu.to_string() };
        let cy = y + seg_h / 2.0;
        let font = FontId::monospace(9.0);
        p.text(Pos2::new(left_x - 3.0, cy), Align2::RIGHT_CENTER, &text, font.clone(), label_ink);
        if !compact {
            p.text(
                Pos2::new(right_x + bar_w + 3.0, cy),
                Align2::LEFT_CENTER,
                &text,
                font,
                label_ink,
            );
        }
    }
    // L and R under their columns.
    for (x, ch) in [(left_x, "L"), (right_x, "R")] {
        p.text(
            Pos2::new(x + bar_w / 2.0, body.bottom() + 3.0),
            Align2::CENTER_TOP,
            ch,
            FontId::proportional(9.0),
            label_ink,
        );
    }

    if compact {
        return resp;
    }
    // The plate: a brushed-grey strip with the name engraved on it.
    let plate = Rect::from_min_max(
        Pos2::new(case.left() + pad + 8.0, case.bottom() - pad - plate_h),
        Pos2::new(case.right() - pad - 8.0, case.bottom() - pad),
    );
    p.rect_filled(plate, 3.0, theme::gray(175));
    p.rect_stroke(plate, 3.0, Stroke::new(1.0, theme::gray(110)), egui::StrokeKind::Inside);
    for (x, y) in [(plate.left() + 4.0, plate.center().y), (plate.right() - 4.0, plate.center().y)]
    {
        p.circle_filled(Pos2::new(x, y), 1.6, theme::gray(110));
    }
    let engrave = FontId::proportional(10.5);
    p.text(
        plate.center() + Vec2::new(0.0, 0.6),
        Align2::CENTER_CENTER,
        PLATE,
        engrave.clone(),
        Color32::from_white_alpha(90),
    );
    p.text(plate.center(), Align2::CENTER_CENTER, PLATE, engrave, theme::gray(55));

    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_alignment_level_reads_zero_vu() {
        assert_eq!(VuBallistics::vu_of_dbfs(ALIGN_DBFS), 0.0);
        assert_eq!(VuBallistics::vu_of_dbfs(f32::NEG_INFINITY), FLOOR_VU);
    }

    #[test]
    fn a_level_lights_the_segments_up_to_it_and_no_further() {
        assert_eq!(lit_segments(FLOOR_VU), 0, "silence is dark");
        assert_eq!(lit_segments(-60.0), 1, "the bottom segment");
        assert_eq!(lit_segments(0.0), 12, "0 VU: everything from -60 to 0");
        assert_eq!(lit_segments(8.0), SCALE_VU.len(), "the top segment is +8");
        assert_eq!(lit_segments(5.9), 11 + 3, "+6 is red and not reached at +5.9");
    }

    /// The meter rises over about 300 ms, as a VU meter does, rather than
    /// jumping — and settles on the level it is fed.
    #[test]
    fn the_column_rises_like_a_vu_meter_and_settles() {
        let mut m = VuBallistics::default();
        let tone = Some([ALIGN_DBFS; 2]);
        m.step(tone, 1.0 / 60.0);
        assert!(m.level[0] < -50.0, "one frame does not reach the level: {}", m.level[0]);
        for _ in 0..30 {
            m.step(tone, 1.0 / 60.0);
        }
        assert!(m.level[0] > -1.0, "after ~0.5 s the level is reached: {}", m.level[0]);
    }

    /// The peak marker stays where the loudest moment was for a while, then
    /// falls; and silence takes the whole meter dark.
    #[test]
    fn the_peak_holds_then_falls_and_silence_goes_dark() {
        let mut m = VuBallistics::default();
        for _ in 0..60 {
            m.step(Some([-10.0, -30.0]), 1.0 / 60.0);
        }
        let held = m.peak[0];
        for _ in 0..30 {
            m.step(Some([-40.0, -40.0]), 1.0 / 60.0);
        }
        assert_eq!(m.peak[0], held, "half a second in, the peak still holds");
        assert!(m.level[0] < held, "the level has fallen under it");
        for _ in 0..600 {
            m.step(None, 1.0 / 60.0);
        }
        assert!(!m.is_live(), "ten seconds of nothing is a dark meter: {m:?}");
        assert_eq!(lit_segments(m.level[0]), 0);
    }

    #[test]
    fn left_and_right_move_on_their_own() {
        let mut m = VuBallistics::default();
        for _ in 0..60 {
            m.step(Some([-10.0, -40.0]), 1.0 / 60.0);
        }
        assert!(lit_segments(m.level[0]) > lit_segments(m.level[1]));
    }
}
