use regex::Regex;
use std::sync::OnceLock;
use tower_lsp_server::lsp_types::*;
use tower_lsp_server::lsp_types::{Color, ColorInformation};

fn color_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"#(?:[0-9A-Fa-f]{2})(?:[0-9A-Fa-f]{2})(?:[0-9A-Fa-f]{2})(?:[0-9A-Fa-f]{2})?")
            .unwrap()
    })
}

/// Converts a hex color string (e.g. `#RRGGBB` or `#RRGGBBAA`) into a `Color`.
///
/// This function assumes the input string matches the `color_regex()` pattern,
/// meaning it always starts with `#` and contains 6 or 8 valid hexadecimal digits.
fn hex_to_color(hex: &str) -> Color {
    fn float_from_hex(hex: &str, i: usize) -> f32 {
        u8::from_str_radix(&hex[i..i + 2], 16).unwrap() as f32 / 255.0
    }

    Color {
        red: float_from_hex(hex, 1),
        green: float_from_hex(hex, 3),
        blue: float_from_hex(hex, 5),
        alpha: if hex.len() == 9 {
            float_from_hex(hex, 7)
        } else {
            1.0
        },
    }
}

pub struct Document {
    lines: Vec<String>,
}

impl Document {
    /// Creates a document from a string, splitting it into lines
    pub fn from_str(content: &str) -> Self {
        Self {
            lines: content.lines().map(|l| l.to_string()).collect(),
        }
    }

    pub fn get_colors(&self) -> Vec<ColorInformation> {
        let mut colors = vec![];
        for (line_idx, line_text) in self.lines.iter().enumerate() {
            for mat in color_regex().find_iter(line_text) {
                let color = hex_to_color(mat.as_str());
                let range = Range {
                    start: Position {
                        line: line_idx as u32,
                        character: mat.start() as u32,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: mat.end() as u32,
                    },
                };
                colors.push(ColorInformation { range, color });
            }
        }
        colors
    }

    pub fn edit(&mut self, new_text: &str) {
        self.lines = new_text.lines().map(|l| l.to_string()).collect();
    }
}

#[cfg(test)]
mod tests {
    use crate::document::Document;

    #[test]
    fn test_get_colors_rgb() {
        let doc = Document::from_str(
            r#"
            #FF0000
            #00FF00
            #0000FF
            "#,
        );
        let colors = doc.get_colors();

        assert_eq!(colors.len(), 3);

        let red = &colors[0].color;
        assert_eq!(red.red, 1.0);
        assert_eq!(red.green, 0.0);
        assert_eq!(red.blue, 0.0);
        assert_eq!(red.alpha, 1.0);

        let green = &colors[1].color;
        assert_eq!(green.red, 0.0);
        assert_eq!(green.green, 1.0);
        assert_eq!(green.blue, 0.0);
        assert_eq!(green.alpha, 1.0);

        let blue = &colors[2].color;
        assert_eq!(blue.red, 0.0);
        assert_eq!(blue.green, 0.0);
        assert_eq!(blue.blue, 1.0);
        assert_eq!(blue.alpha, 1.0);
    }

    #[test]
    fn test_get_colors_rgba() {
        let doc = Document::from_str("#11223344");
        let colors = doc.get_colors();
        assert_eq!(colors.len(), 1);
        let c = &colors[0].color;
        assert_eq!(c.red, 0x11 as f32 / 255.0);
        assert_eq!(c.green, 0x22 as f32 / 255.0);
        assert_eq!(c.blue, 0x33 as f32 / 255.0);
        assert_eq!(c.alpha, 0x44 as f32 / 255.0);
    }

    #[test]
    fn test_replace_text() {
        let mut doc = Document::from_str("#FF0000");
        let colors1 = doc.get_colors();

        assert_eq!(colors1.len(), 1);
        let c1 = &colors1[0].color;
        assert_eq!(c1.red, 1.0);
        assert_eq!(c1.green, 0.0);
        assert_eq!(c1.blue, 0.0);
        assert_eq!(c1.alpha, 1.0);

        doc.edit("#00FF00");
        let colors2 = doc.get_colors();
        assert_eq!(colors2.len(), 1);

        let c2 = &colors2[0].color;
        assert_eq!(c2.red, 0.0);
        assert_eq!(c2.green, 1.0);
        assert_eq!(c2.blue, 0.0);
        assert_eq!(c2.alpha, 1.0);
    }

    #[test]
    fn test_append_text() {
        let mut doc = Document::from_str("#FF0000");
        let colors1 = doc.get_colors();

        assert_eq!(colors1.len(), 1);
        let c1 = &colors1[0].color;
        assert_eq!(c1.red, 1.0);
        assert_eq!(c1.green, 0.0);
        assert_eq!(c1.blue, 0.0);
        assert_eq!(c1.alpha, 1.0);

        doc.edit(
            r#"
            #FF0000
            #00FF00
            "#,
        );
        let colors2 = doc.get_colors();
        assert_eq!(colors2.len(), 2);

        let c2 = &colors2[0].color;
        assert_eq!(c2.red, 1.0);
        assert_eq!(c2.green, 0.0);
        assert_eq!(c2.blue, 0.0);
        assert_eq!(c2.alpha, 1.0);

        let c3 = &colors2[1].color;
        assert_eq!(c3.red, 0.0);
        assert_eq!(c3.green, 1.0);
        assert_eq!(c3.blue, 0.0);
        assert_eq!(c3.alpha, 1.0);
    }
}
