use std::sync::OnceLock;

use regex::Regex;
use tower_lsp_server::lsp_types::{Color, ColorInformation, Position, Range};

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

fn byte_to_utf16_col(line: &str, byte_idx: usize) -> u32 {
    line[..byte_idx].encode_utf16().count() as u32
}

pub fn parse_line_colors(line: &str, line_idx: usize) -> Vec<ColorInformation> {
    color_regex()
        .find_iter(line)
        .map(|mat| {
            let start_utf16 = byte_to_utf16_col(line, mat.start());
            let end_utf16 = byte_to_utf16_col(line, mat.end());
            ColorInformation {
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: start_utf16,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: end_utf16,
                    },
                },
                color: hex_to_color(mat.as_str()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb() {
        let colors = parse_line_colors("#FF0000", 0);
        assert_eq!(colors.len(), 1);

        let c = &colors[0];
        assert_eq!(c.color.red, 1.0);
        assert_eq!(c.color.green, 0.0);
        assert_eq!(c.color.blue, 0.0);
        assert_eq!(c.range.start.character, 0);
        assert_eq!(c.range.end.character, 7);
    }

    #[test]
    fn rgba() {
        let colors = parse_line_colors("#11223344", 0);
        assert_eq!(colors.len(), 1);

        let c = &colors[0];
        assert_eq!(c.color.red, 0x11 as f32 / 255.0);
        assert_eq!(c.color.green, 0x22 as f32 / 255.0);
        assert_eq!(c.color.blue, 0x33 as f32 / 255.0);
        assert_eq!(c.color.alpha, 0x44 as f32 / 255.0);
    }

    #[test]
    fn unicode_before() {
        let colors = parse_line_colors("•#FF0000", 0);
        assert_eq!(colors.len(), 1);

        let c = &colors[0];
        assert_eq!(c.color.red, 1.0);
        assert_eq!(c.color.green, 0.0);
        assert_eq!(c.color.blue, 0.0);
        assert_eq!(c.range.start.character, 1);
        assert_eq!(c.range.end.character, 8);
    }

    #[test]
    fn multiple_unicode_before() {
        let colors = parse_line_colors("• ☀️#FF0000", 0);
        assert_eq!(colors.len(), 1);

        let c = &colors[0];
        assert_eq!(c.color.red, 1.0);
        assert_eq!(c.color.green, 0.0);
        assert_eq!(c.color.blue, 0.0);
        assert_eq!(c.range.start.character, 4);
        assert_eq!(c.range.end.character, 11);
    }

    #[test]
    fn unicode_after() {
        let colors = parse_line_colors("#FF0000•", 0);
        assert_eq!(colors.len(), 1);

        let c = &colors[0];
        assert_eq!(c.color.red, 1.0);
        assert_eq!(c.color.green, 0.0);
        assert_eq!(c.color.blue, 0.0);
        assert_eq!(c.range.start.character, 0);
        assert_eq!(c.range.end.character, 7);
    }

    #[test]
    fn multiple_colors() {
        let colors = parse_line_colors("#FF0000 #00FF00 #0000FF", 0);
        assert_eq!(colors.len(), 3);

        assert_eq!(colors[0].range.start.character, 0);
        assert_eq!(colors[1].range.start.character, 8);
        assert_eq!(colors[2].range.start.character, 16);
    }

    #[test]
    fn no_colors() {
        let colors = parse_line_colors("no colors here", 2);
        assert!(colors.is_empty());
    }

    #[test]
    fn embedded_color() {
        let colors = parse_line_colors("Color: #ABCDEF;", 3);
        assert_eq!(colors.len(), 1);

        let c = &colors[0];
        assert_eq!(c.range.start.character, 7);
        assert_eq!(c.range.end.character, 14);
    }
}
