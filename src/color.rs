use tower_lsp_server::ls_types::{Color, ColorInformation, Position, Range};

/// Parses all hex color codes in a line and returns them as `ColorInformation`.
pub fn parse_line_colors(line: &str, line_idx: usize) -> Vec<ColorInformation> {
    let mut colors: Vec<ColorInformation> = Vec::new();

    let chars: Vec<u16> = line.encode_utf16().collect();
    let mut i: usize = 0;
    while i < chars.len() {
        let current_unit = chars[i];
        i += 1;

        if current_unit != '#' as u16 {
            // Skip until first '#'
            continue;
        }

        let mut digits = [0u8; 8];
        let mut length: u32 = 0;
        // Replace "slots" in digits with parsed colors
        for slot in digits.iter_mut() {
            if i >= chars.len() {
                break;
            }
            let current_unit = chars[i];
            let current_char = char::from_u32(current_unit as u32).unwrap();

            let Some(digit) = current_char.to_digit(16) else {
                break;
            };
            *slot = digit as u8;

            length += 1;
            i += 1;
        }
        if length < 6 {
            continue;
        }

        // Fallback to length 6 if 7 digits was parsed
        if length == 7 {
            length = 6;
            i -= 1;
        }

        let color = color_from_digits(digits, length);
        colors.push(ColorInformation {
            range: Range {
                start: Position {
                    line: line_idx as u32,
                    character: i as u32 - (1 + length),
                },
                end: Position {
                    line: line_idx as u32,
                    character: i as u32,
                },
            },
            color,
        });
    }
    colors
}

fn color_from_digits(digits: [u8; 8], length: u32) -> Color {
    let red = (digits[0] * 16 + digits[1]) as f32 / 255.0;
    let green = (digits[2] * 16 + digits[3]) as f32 / 255.0;
    let blue = (digits[4] * 16 + digits[5]) as f32 / 255.0;
    let alpha = if length == 8 {
        (digits[6] * 16 + digits[7]) as f32 / 255.0
    } else {
        1.0
    };

    Color {
        red,
        green,
        blue,
        alpha,
    }
}

#[cfg(test)]
mod tests {
    use crate::color::parse_line_colors;

    #[test]
    fn parse_line_colors_line_idx() {
        let colors = parse_line_colors("#FF0000", 10);
        assert_eq!(colors.len(), 1);

        let color_info = &colors[0];
        assert_eq!(color_info.range.start.line, 10);
        assert_eq!(color_info.range.end.line, 10);
    }

    #[test]
    fn parse_line_colors_rgb() {
        let colors = parse_line_colors("#FF0000", 0);
        assert_eq!(colors.len(), 1);

        let color_info = &colors[0];
        assert_eq!(color_info.color.red, 1.0);
        assert_eq!(color_info.color.green, 0.0);
        assert_eq!(color_info.color.blue, 0.0);
        assert_eq!(color_info.range.start.character, 0);
        assert_eq!(color_info.range.end.character, 7);
    }

    #[test]
    fn parse_line_colors_rgb_lowercase() {
        let colors = parse_line_colors("#ff0000", 0);
        assert_eq!(colors.len(), 1);

        let color_info = &colors[0];
        assert_eq!(color_info.color.red, 1.0);
        assert_eq!(color_info.color.green, 0.0);
        assert_eq!(color_info.color.blue, 0.0);
        assert_eq!(color_info.range.start.character, 0);
        assert_eq!(color_info.range.end.character, 7);
    }

    #[test]
    fn parse_line_colors_rgba() {
        let colors = parse_line_colors("#11223344", 0);
        assert_eq!(colors.len(), 1);

        let color_info = &colors[0];
        assert_eq!(color_info.color.red, 0x11 as f32 / 255.0);
        assert_eq!(color_info.color.green, 0x22 as f32 / 255.0);
        assert_eq!(color_info.color.blue, 0x33 as f32 / 255.0);
        assert_eq!(color_info.color.alpha, 0x44 as f32 / 255.0);
    }

    #[test]
    fn parse_line_colors_unicode_before() {
        let colors = parse_line_colors("•#FF0000•", 0);
        assert_eq!(colors.len(), 1);

        let color_info = &colors[0];
        assert_eq!(color_info.color.red, 1.0);
        assert_eq!(color_info.color.green, 0.0);
        assert_eq!(color_info.color.blue, 0.0);
        assert_eq!(color_info.range.start.character, 1);
        assert_eq!(color_info.range.end.character, 8);
    }

    #[test]
    fn parse_line_colors_multiple_colors() {
        let colors = parse_line_colors("#FF0000 #00FF00 #0000FF", 0);
        assert_eq!(colors.len(), 3);

        assert_eq!(colors[0].range.start.character, 0);
        assert_eq!(colors[1].range.start.character, 8);
        assert_eq!(colors[2].range.start.character, 16);
    }

    #[test]
    fn parse_line_colors_no_colors() {
        let colors = parse_line_colors("#### no colors here #A 161616 #FF FF FF", 0);
        assert_eq!(colors, Vec::new());
    }

    #[test]
    fn parse_line_colors_text_with_color() {
        let colors = parse_line_colors("Color: #ABCDEF;", 0);
        assert_eq!(colors.len(), 1);

        let color = &colors[0];
        assert_eq!(color.range.start.character, 7);
        assert_eq!(color.range.end.character, 14);
    }

    #[test]
    fn parse_line_colors_hash_before() {
        let colors = parse_line_colors("#A#ABCDEF", 0);
        assert_eq!(colors.len(), 1);

        let color_info = &colors[0];
        assert_eq!(color_info.range.start.character, 2);
        assert_eq!(color_info.range.end.character, 9);
    }

    #[test]
    fn parse_line_colors_embedded_color() {
        let colors = parse_line_colors("123#ABCDEFasd", 0);
        assert_eq!(colors.len(), 1);

        let color_info = &colors[0];
        assert_eq!(color_info.range.start.character, 3);
        assert_eq!(color_info.range.end.character, 10);
    }
}
