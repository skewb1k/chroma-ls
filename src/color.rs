use tower_lsp_server::lsp_types::{Color, ColorInformation, Position, Range};

/// Parses all hex color codes in a line and returns them as `ColorInformation`.
pub fn parse_line_colors(line: &str, line_idx: usize) -> Vec<ColorInformation> {
    let mut colors: Vec<ColorInformation> = Vec::new();
    let mut chars = line.encode_utf16().peekable();
    let mut pos: u32 = 0; // position in UTF-16 code units

    while let Some(&c) = chars.peek() {
        pos += 1;
        chars.next();
        if c != '#' as u16 {
            // Skip until first '#'
            continue;
        }

        let mut digits = [0u8; 8];
        let mut length = 0;
        // Replace "slots" in digits with parsed colors.
        for slot in digits.iter_mut() {
            // Try to parse hex digit
            let Some(digit) = chars
                .peek()
                .and_then(|&c| char::from_u32(c as u32))
                .and_then(|ch| ch.to_digit(16))
                .map(|val| val as u8)
            else {
                break;
            };
            *slot = digit;
            length += 1;
            pos += 1;
            chars.next();
        }
        // Fallback to length 6 if 7 digits was parsed.
        if length == 7 {
            length = 6;
            pos -= 1
        }

        if length < 6 {
            continue;
        }

        let red = (digits[0] * 16 + digits[1]) as f32 / 255.0;
        let green = (digits[2] * 16 + digits[3]) as f32 / 255.0;
        let blue = (digits[4] * 16 + digits[5]) as f32 / 255.0;
        let alpha = if length == 8 {
            (digits[6] * 16 + digits[7]) as f32 / 255.0
        } else {
            1.0
        };

        colors.push(ColorInformation {
            range: Range {
                start: Position {
                    line: line_idx as u32,
                    character: pos - (1 + length),
                },
                end: Position {
                    line: line_idx as u32,
                    character: pos,
                },
            },
            color: Color {
                red,
                green,
                blue,
                alpha,
            },
        });
    }
    colors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_line_colors_line_idx() {
        let colors = parse_line_colors("#FF0000", 10);
        assert_eq!(colors.len(), 1);

        let c = &colors[0];
        assert_eq!(c.range.start.line, 10);
        assert_eq!(c.range.end.line, 10);
    }

    #[test]
    fn parse_line_colors_rgb() {
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
    fn parse_line_colors_rgb_lowercase() {
        let colors = parse_line_colors("#ff0000", 0);
        assert_eq!(colors.len(), 1);

        let c = &colors[0];
        assert_eq!(c.color.red, 1.0);
        assert_eq!(c.color.green, 0.0);
        assert_eq!(c.color.blue, 0.0);
        assert_eq!(c.range.start.character, 0);
        assert_eq!(c.range.end.character, 7);
    }

    #[test]
    fn parse_line_colors_rgba() {
        let colors = parse_line_colors("#11223344", 0);
        assert_eq!(colors.len(), 1);

        let c = &colors[0];
        assert_eq!(c.color.red, 0x11 as f32 / 255.0);
        assert_eq!(c.color.green, 0x22 as f32 / 255.0);
        assert_eq!(c.color.blue, 0x33 as f32 / 255.0);
        assert_eq!(c.color.alpha, 0x44 as f32 / 255.0);
    }

    #[test]
    fn parse_line_colors_unicode_before() {
        let colors = parse_line_colors("•#FF0000•", 0);
        assert_eq!(colors.len(), 1);

        let c = &colors[0];
        assert_eq!(c.color.red, 1.0);
        assert_eq!(c.color.green, 0.0);
        assert_eq!(c.color.blue, 0.0);
        assert_eq!(c.range.start.character, 1);
        assert_eq!(c.range.end.character, 8);
    }

    #[test]
    fn parse_line_colors_multiple_colors() {
        let colors = parse_line_colors("#FF0000#00FF00#0000FF", 0);
        assert_eq!(colors.len(), 3);

        assert_eq!(colors[0].range.start.character, 0);
        assert_eq!(colors[1].range.start.character, 7);
        assert_eq!(colors[2].range.start.character, 14);
    }

    #[test]
    fn parse_line_colors_no_colors() {
        let colors = parse_line_colors("#### no colors here #A 161616 #FF FF FF", 0);
        assert!(colors.is_empty());
    }

    #[test]
    fn parse_line_colors_text_with_color() {
        let colors = parse_line_colors("Color: #ABCDEF;", 0);
        assert_eq!(colors.len(), 1);

        let c = &colors[0];
        assert_eq!(c.range.start.character, 7);
        assert_eq!(c.range.end.character, 14);
    }

    #[test]
    fn parse_line_colors_hash_before() {
        let colors = parse_line_colors("#A#ABCDEF", 0);
        assert_eq!(colors.len(), 1);

        let c = &colors[0];
        assert_eq!(c.range.start.character, 2);
        assert_eq!(c.range.end.character, 9);
    }

    #[test]
    fn parse_line_colors_embedded_color() {
        let colors = parse_line_colors("123#ABCDEFasd", 0);
        assert_eq!(colors.len(), 1);

        let c = &colors[0];
        assert_eq!(c.range.start.character, 3);
        assert_eq!(c.range.end.character, 10);
    }
}
