use std::fmt::Write as _;

use rustybuzz::ttf_parser::{Face, GlyphId, OutlineBuilder};

#[derive(Default)]
struct Path(String);

impl OutlineBuilder for Path {
    fn move_to(&mut self, x: f32, y: f32) {
        write!(self.0, "M{x} {y}").unwrap();
    }
    fn line_to(&mut self, x: f32, y: f32) {
        write!(self.0, "L{x} {y}").unwrap();
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        write!(self.0, "Q{x1} {y1} {x} {y}").unwrap();
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        write!(self.0, "C{x1} {y1} {x2} {y2} {x} {y}").unwrap();
    }
    fn close(&mut self) {
        self.0.push('Z');
    }
}

/// Outline in unscaled font units, y upwards. A space has an empty path.
pub fn svg_path(face: &Face<'_>, id: u16) -> String {
    let mut path = Path::default();
    face.outline_glyph(GlyphId(id), &mut path);
    path.0
}
