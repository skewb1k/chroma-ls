use regex::Regex;
use std::sync::OnceLock;
use tower_lsp_server::lsp_types::*;
use tower_lsp_server::lsp_types::{Color, ColorInformation};

pub struct Document {
    pub content: String,
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

impl Document {
    pub fn get_colors(&self) -> Vec<ColorInformation> {
        let mut colors = vec![];
        for (line_idx, line_text) in self.content.lines().enumerate() {
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
}

fn color_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"#(?:[0-9A-Fa-f]{2})(?:[0-9A-Fa-f]{2})(?:[0-9A-Fa-f]{2})(?:[0-9A-Fa-f]{2})?")
            .unwrap()
    })
}
