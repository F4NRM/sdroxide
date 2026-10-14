//! The colour under the spectrum trace: the area between the line and the
//! bottom of the spectrum, filled so the band reads as a shape and a signal
//! stands out by its mass as well as by its outline.
//!
//! Kept apart from the panadapter's draw so the colour rule is one function the
//! panadapter and `examples/spectrum_fill_probe.rs` both call — the probe draws
//! the fill through this code, not through a copy of it.

use eframe::egui::epaint::{Mesh, Vertex, WHITE_UV};
use eframe::egui::{Color32, Pos2};

/// How the fill is shaded down a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillStyle {
    /// One translucent colour from the line to the bottom.
    Solid,
    /// Full at the bottom, fading out as it rises to the line.
    Gradient,
}

/// Opacity of a solid fill: enough to give the band its mass, little enough
/// that the grid still shows through it.
const SOLID_ALPHA: u8 = 90;
/// Opacity of a gradient at the bottom of the spectrum; it falls to nothing at
/// the line.
const GRADIENT_ALPHA: u8 = 170;

/// The colours at the line and at the bottom of a column, premultiplied.
pub fn fill_colors(rgb: [u8; 3], style: FillStyle) -> (Color32, Color32) {
    let [r, g, b] = rgb;
    match style {
        FillStyle::Solid => {
            let c = Color32::from_rgba_unmultiplied(r, g, b, SOLID_ALPHA);
            (c, c)
        }
        FillStyle::Gradient => {
            (Color32::TRANSPARENT, Color32::from_rgba_unmultiplied(r, g, b, GRADIENT_ALPHA))
        }
    }
}

/// The fill under a trace, as a mesh of one quad per pair of trace points:
/// each column runs from the line down to `bottom`.
pub fn fill_mesh(points: &[Pos2], bottom: f32, rgb: [u8; 3], style: FillStyle) -> Mesh {
    let (at_line, at_bottom) = fill_colors(rgb, style);
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
        let mesh = fill_mesh(&pts, 50.0, [120, 220, 255], FillStyle::Solid);
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
    fn the_gradient_fades_out_at_the_line() {
        let (line, bottom) = fill_colors([0, 255, 0], FillStyle::Gradient);
        assert_eq!(line.a(), 0, "the gradient is not clear at the line");
        assert!(bottom.a() > 0, "the gradient has no colour at the bottom");
        let (line, bottom) = fill_colors([0, 255, 0], FillStyle::Solid);
        assert_eq!(line, bottom, "a solid fill changed down the column");
    }

    #[test]
    fn a_trace_too_short_to_span_anything_draws_nothing() {
        assert!(fill_mesh(&[Pos2::new(0.0, 1.0)], 5.0, [1, 2, 3], FillStyle::Solid).is_empty());
    }
}
