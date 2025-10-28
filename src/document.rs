use regex::Regex;
use std::fmt;
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

#[derive(Default)]
pub struct Document {
    lines: Vec<String>,
}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.lines.join("\n"))
    }
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

    pub fn edit(&mut self, change: &TextDocumentContentChangeEvent) {
        match &change.range {
            // Full content replace
            None => {
                self.lines = change.text.lines().map(|l| l.to_string()).collect();
            }

            // Partial change
            // TODO: implement incremental color update for only the affected range.
            Some(range) => {
                let start_line = range.start.line as usize;
                let start_char = range.start.character as usize;
                let end_line = range.end.line as usize;
                let end_char = range.end.character as usize;

                // Ensure enough lines exist
                while self.lines.len() <= end_line {
                    self.lines.push(String::new());
                }

                let prefix =
                    &self.lines[start_line][..start_char.min(self.lines[start_line].len())];
                let suffix = &self.lines[end_line][end_char.min(self.lines[end_line].len())..];

                let mut new_lines: Vec<String> =
                    change.text.lines().map(|l| l.to_string()).collect();

                // .lines() ignores final line ending.
                // TODO: handle \r.
                if change.text.ends_with('\n') {
                    new_lines.push(String::new());
                }

                if new_lines.is_empty() {
                    // Empty insertion: merge prefix + suffix
                    new_lines.push(format!("{}{}", prefix, suffix));
                } else {
                    // Merge prefix with first line
                    new_lines[0] = format!("{}{}", prefix, new_lines[0]);
                    // Merge suffix with last line
                    let last_idx = new_lines.len() - 1;
                    new_lines[last_idx] = format!("{}{}", new_lines[last_idx], suffix);
                }

                self.lines.splice(start_line..=end_line, new_lines);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::document::Document;
    use tower_lsp_server::lsp_types::{Position, Range, TextDocumentContentChangeEvent};

    macro_rules! assert_colors {
        ($colors:expr, $(($r:expr, $g:expr, $b:expr, $a:expr)),+ $(,)?) => {{
            let colors = &$colors;
            let mut i = 0;
            $(
                {
                    let c = &colors[i];
                    assert_eq!(c.color.red, $r, "color[{}]: red mismatch", i);
                    assert_eq!(c.color.green, $g, "color[{}]: green mismatch", i);
                    assert_eq!(c.color.blue, $b, "color[{}]: blue mismatch", i);
                    assert_eq!(c.color.alpha, $a, "color[{}]: alpha mismatch", i);
                    i += 1;
                }
            )+
            assert_eq!(colors.len(), i, "unexpected number of colors");
        }};
    }

    #[test]
    fn test_get_colors_rgb() {
        let doc = Document::from_str(
            r#"
            #FF0000
            #00FF00
            #0000FF
            "#,
        );

        assert_colors!(
            doc.get_colors(),
            (1.0, 0.0, 0.0, 1.0),
            (0.0, 1.0, 0.0, 1.0),
            (0.0, 0.0, 1.0, 1.0),
        );
    }

    #[test]
    fn test_get_colors_rgba() {
        let doc = Document::from_str("#11223344");

        assert_colors!(
            doc.get_colors(),
            (
                0x11 as f32 / 255.0,
                0x22 as f32 / 255.0,
                0x33 as f32 / 255.0,
                0x44 as f32 / 255.0
            ),
        );
    }

    #[test]
    fn test_replace_text() {
        let mut doc = Document::from_str("#FF0000");

        assert_colors!(doc.get_colors(), (1.0, 0.0, 0.0, 1.0));

        doc.edit(&TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "#00FF00".to_string(),
        });

        assert_colors!(doc.get_colors(), (0.0, 1.0, 0.0, 1.0));
    }

    #[test]
    fn test_append_end() {
        let mut doc = Document::from_str("#FF0000");

        assert_colors!(doc.get_colors(), (1.0, 0.0, 0.0, 1.0));

        doc.edit(&TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position {
                    line: 0,
                    character: 7,
                },
                end: Position {
                    line: 1,
                    character: 0,
                },
            }),
            range_length: None,
            text: "\n#00FF00".to_string(),
        });

        assert_colors!(doc.get_colors(), (1.0, 0.0, 0.0, 1.0), (0.0, 1.0, 0.0, 1.0));
    }

    #[test]
    fn test_append_middle() {
        let mut doc = Document::from_str("#FF0000\n#0000FF");

        assert_colors!(doc.get_colors(), (1.0, 0.0, 0.0, 1.0), (0.0, 0.0, 1.0, 1.0));

        doc.edit(&TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position {
                    line: 0,
                    character: 7,
                },
                end: Position {
                    line: 1,
                    character: 0,
                },
            }),
            range_length: None,
            text: "\n#00FF00\n".to_string(),
        });

        assert_colors!(
            doc.get_colors(),
            (1.0, 0.0, 0.0, 1.0),
            (0.0, 1.0, 0.0, 1.0),
            (0.0, 0.0, 1.0, 1.0)
        );

        assert_eq!(doc.to_string(), "#FF0000\n#00FF00\n#0000FF");
    }

    #[test]
    fn test_delete_color_line() {
        let mut doc = Document::from_str(
            r#"
            #FF0000
            #00FF00
            #0000FF
            "#,
        );

        assert_colors!(
            doc.get_colors(),
            (1.0, 0.0, 0.0, 1.0),
            (0.0, 1.0, 0.0, 1.0),
            (0.0, 0.0, 1.0, 1.0),
        );

        // Delete the middle line
        doc.edit(&TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position {
                    line: 2,
                    character: 0,
                },
                end: Position {
                    line: 3,
                    character: 0,
                },
            }),
            range_length: None,
            text: "".to_string(),
        });

        assert_colors!(doc.get_colors(), (1.0, 0.0, 0.0, 1.0), (0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn test_delete_one_char() {
        let mut doc = Document::from_str("#FF0000");

        assert_colors!(doc.get_colors(), (1.0, 0.0, 0.0, 1.0));

        // Delete the last char
        doc.edit(&TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position {
                    line: 0,
                    character: 6,
                },
                end: Position {
                    line: 0,
                    character: 7,
                },
            }),
            range_length: None,
            text: "".to_string(),
        });

        assert!(doc.get_colors().is_empty());
    }

    #[test]
    fn test_replace_partial_line() {
        let mut doc = Document::from_str("#FF0000");

        assert_colors!(doc.get_colors(), (1.0, 0.0, 0.0, 1.0));

        // Replace last 4 characters "0000" → "00FF"
        doc.edit(&TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position {
                    line: 0,
                    character: 3,
                },
                end: Position {
                    line: 0,
                    character: 7,
                },
            }),
            range_length: None,
            text: "00FF".to_string(),
        });

        assert_colors!(doc.get_colors(), (1.0, 0.0, 1.0, 1.0)); // now #FF00FF
    }

    #[test]
    fn test_clear_document_then_add_new() {
        let mut doc = Document::from_str("#FF0000\n#00FF00");

        assert_colors!(doc.get_colors(), (1.0, 0.0, 0.0, 1.0), (0.0, 1.0, 0.0, 1.0));

        // Clear all content (simulate full replace)
        doc.edit(&TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "".to_string(),
        });

        assert!(doc.get_colors().is_empty());

        // Add a new color
        doc.edit(&TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "#FFFFFF".to_string(),
        });

        assert_colors!(doc.get_colors(), (1.0, 1.0, 1.0, 1.0));
    }

    #[test]
    fn test_multiple_incremental_edits() {
        let mut doc = Document::from_str("#FF0000");

        assert_colors!(doc.get_colors(), (1.0, 0.0, 0.0, 1.0));

        // Append new color
        doc.edit(&TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position {
                    line: 0,
                    character: 7,
                },
                end: Position {
                    line: 1,
                    character: 0,
                },
            }),
            range_length: None,
            text: "\n#00FF00".to_string(),
        });

        assert_colors!(doc.get_colors(), (1.0, 0.0, 0.0, 1.0), (0.0, 1.0, 0.0, 1.0));

        // Append another color
        doc.edit(&TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position {
                    line: 1,
                    character: 7,
                },
                end: Position {
                    line: 2,
                    character: 0,
                },
            }),
            range_length: None,
            text: "\n#0000FF".to_string(),
        });

        assert_colors!(
            doc.get_colors(),
            (1.0, 0.0, 0.0, 1.0),
            (0.0, 1.0, 0.0, 1.0),
            (0.0, 0.0, 1.0, 1.0)
        );
    }
}
