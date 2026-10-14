//! The colour under the spectrum trace: the area between the line and the
//! bottom of the spectrum, full at the bottom and fading out as it rises to
//! the line, so the band reads as a shape while the line itself stays crisp.
//!
//! Kept apart from the panadapter's draw so the colour rule is one function the
//! panadapter and `examples/spectrum_fill_probe.rs` both call — the probe draws
//! the fill through this code, not through a copy of it.

use eframe::egui::epaint::{Mesh, Vertex, WHITE_UV};
use eframe::egui::{Color32, Pos2};

/// Opacity at the bottom of the spectrum; it falls to nothing at the line.
const BOTTOM_ALPHA: u8 = 170;

/// The colours at the line and at the bottom of a column, premultiplied.
pub fn fill_colors(rgb: [u8; 3]) -> (Color32, Color32) {
    let [r, g, b] = rgb;
    (Color32::TRANSPARENT, Color32::from_rgba_unmultiplied(r, g, b, BOTTOM_ALPHA))
}

/// The fill under a trace, as a mesh of one quad per pair of trace points:
/// each column runs from the line down to `bottom`.
pub fn fill_mesh(points: &[Pos2], bottom: f32, rgb: [u8; 3]) -> Mesh {
    let (at_line, at_bottom) = fill_colors(rgb);
    let mut mesh = Mesh::default();
    if points.len() < 2 {
        return mesh;
    }
    mesh.vertices.reserve(points.len() * 2);
    mesh.indices.reserve((points.len() - 1) * 6);
    for p in points {
        let top = p.y.min(bottom);
        mesh.vertices.push(Vertex { pos: Pos2::new(p.x, top), uv: WHITE_UV, color: at_line });
        mesh.vertices.push(Vertex { pos: Pos2::new(p.x, bottom), uv: WHITE_UV, color: at_bottom });
    }
    for i in 0..points.len() as u32 - 1 {
        let (t0, b0, t1, b1) = (2 * i, 2 * i + 1, 2 * i + 2, 2 * i + 3);
        mesh.indices.extend_from_slice(&[t0, b0, t1, t1, b0, b1]);
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fill_runs_from_the_line_to_the_bottom() {
        let pts = [Pos2::new(0.0, 10.0), Pos2::new(1.0, 40.0), Pos2::new(2.0, 25.0)];
        let mesh = fill_mesh(&pts, 50.0, [120, 220, 255]);
        assert_eq!(mesh.vertices.len(), 6);
        assert_eq!(mesh.indices.len(), 12);
        for (i, p) in pts.iter().enumerate() {
            assert_eq!(mesh.vertices[2 * i].pos, *p, "the fill left the line");
            assert_eq!(
                mesh.vertices[2 * i + 1].pos.y,
                50.0,
                "the fill stopped short of the bottom"
            );
        }
    }

    #[test]
    fn the_fill_fades_out_at_the_line() {
        let mesh = fill_mesh(&[Pos2::new(0.0, 10.0), Pos2::new(1.0, 20.0)], 50.0, [0, 255, 0]);
        assert_eq!(mesh.vertices[0].color.a(), 0, "the fill is not clear at the line");
        assert!(mesh.vertices[1].color.a() > 0, "the fill has no colour at the bottom");
        assert!(mesh.vertices[1].color.g() > 0, "the fill lost its colour");
    }

    #[test]
    fn a_trace_too_short_to_span_anything_draws_nothing() {
        assert!(fill_mesh(&[Pos2::new(0.0, 1.0)], 5.0, [1, 2, 3]).is_empty());
    }
}
