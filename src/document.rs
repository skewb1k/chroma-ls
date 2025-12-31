use tower_lsp_server::lsp_types::ColorInformation;
use tower_lsp_server::lsp_types::*;

use crate::color::parse_line_colors;

#[derive(Default)]
pub struct Line {
    text: String,
    colors: Vec<ColorInformation>,
}

impl std::fmt::Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

pub struct Document {
    lines: Vec<Line>,
}

impl std::fmt::Display for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{line}")?;
        }
        Ok(())
    }
}

impl From<&str> for Document {
    /// Converts a `&str` into a `Document` by splitting it into lines.
    fn from(s: &str) -> Self {
        let lines = s
            .lines()
            .enumerate()
            .map(|(idx, line)| Line {
                text: line.to_string(),
                colors: parse_line_colors(line, idx),
            })
            .collect();

        Self { lines }
    }
}

impl Document {
    pub fn get_colors(&self) -> Vec<ColorInformation> {
        // TODO: do smarter than collecting lines.
        self.lines
            .iter()
            .flat_map(|line| line.colors.clone())
            .collect()
    }

    pub fn edit(&mut self, change: &TextDocumentContentChangeEvent) {
        match &change.range {
            // Full content replace
            None => {
                self.lines = change
                    .text
                    .lines()
                    .enumerate()
                    .map(|(i, line)| Line {
                        text: line.to_string(),
                        colors: parse_line_colors(line, i),
                    })
                    .collect();
            }
            // Partial change
            Some(range) => {
                let start_line = range.start.line as usize;
                let end_line = range.end.line as usize;

                // Ensure enough lines exist
                while self.lines.len() <= end_line {
                    self.lines.push(Line::default());
                }

                let start_byte = utf16_to_byte_index(
                    &self.lines[start_line].text,
                    range.start.character as usize,
                );
                let end_byte =
                    utf16_to_byte_index(&self.lines[end_line].text, range.end.character as usize);

                let prefix = &self.lines[start_line].text
                    [..start_byte.min(self.lines[start_line].text.len())];
                let suffix =
                    &self.lines[end_line].text[end_byte.min(self.lines[end_line].text.len())..];

                let mut new_lines: Vec<Line> = change
                    .text
                    .lines()
                    .map(|line| Line {
                        text: line.to_string(),
                        colors: vec![],
                    })
                    .collect();

                // .lines() ignores final line ending.
                // TODO: handle \r.
                if change.text.ends_with('\n') {
                    new_lines.push(Line::default());
                }

                if new_lines.is_empty() {
                    new_lines.push(Line {
                        text: format!("{}{}", prefix, suffix),
                        colors: vec![],
                    });
                } else {
                    new_lines[0].text = format!("{}{}", prefix, new_lines[0].text);
                    let last_idx = new_lines.len() - 1;
                    new_lines[last_idx].text = format!("{}{}", new_lines[last_idx].text, suffix);
                }

                // Reparse colors for each new line
                for (i, line) in new_lines.iter_mut().enumerate() {
                    line.colors = parse_line_colors(&line.text, start_line + i);
                }

                // Save number of lines replaced
                let replaced_line_count = end_line - start_line + 1;

                // Replace lines in the document
                // Adjust line numbers of all colors after the edited range
                let line_delta = new_lines.len() as isize - replaced_line_count as isize;
                if line_delta != 0 {
                    for line in &mut self.lines[start_line + replaced_line_count..] {
                        for color in &mut line.colors {
                            color.range.start.line =
                                (color.range.start.line as isize + line_delta) as u32;
                            color.range.end.line =
                                (color.range.end.line as isize + line_delta) as u32;
                        }
                    }
                }

                self.lines.splice(start_line..=end_line, new_lines);
            }
        }
    }
}

fn utf16_to_byte_index(line: &str, utf16_idx: usize) -> usize {
    let mut count = 0;
    for (byte_idx, _) in line.char_indices() {
        if count == utf16_idx {
            return byte_idx;
        }
        count += line[byte_idx..]
            .chars()
            .next()
            .unwrap()
            .encode_utf16(&mut [0; 2])
            .len();
    }
    line.len()
}

#[cfg(test)]
mod tests {
    use crate::document::Document;
    use tower_lsp_server::lsp_types::*;

    #[test]
    fn unicode_edit_in_string() {
        let mut doc = Document::from("a•a");

        doc.edit(&TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position {
                    line: 0,
                    character: 2,
                },
                end: Position {
                    line: 0,
                    character: 3,
                },
            }),
            range_length: None,
            text: "b".to_string(),
        });

        assert_eq!(doc.to_string(), "a•b");
    }

    fn assert_colors_eq(
        colors: Vec<ColorInformation>,
        expected: &[(f32, f32, f32, f32, u32, u32, u32, u32)],
    ) {
        assert_eq!(colors.len(), expected.len(), "unexpected number of colors");
        for (i, (c, &(r, g, b, a, sl, sc, el, ec))) in
            colors.iter().zip(expected.iter()).enumerate()
        {
            assert_eq!(c.color.red, r, "color[{i}]: red mismatch");
            assert_eq!(c.color.green, g, "color[{i}]: green mismatch");
            assert_eq!(c.color.blue, b, "color[{i}]: blue mismatch");
            assert_eq!(c.color.alpha, a, "color[{i}]: alpha mismatch");

            assert_eq!(c.range.start.line, sl, "color[{i}]: start line mismatch");
            assert_eq!(
                c.range.start.character, sc,
                "color[{i}]: start char mismatch"
            );
            assert_eq!(c.range.end.line, el, "color[{i}]: end line mismatch");
            assert_eq!(c.range.end.character, ec, "color[{i}]: end char mismatch");
        }
    }

    #[test]
    fn replace_text() {
        let mut doc = Document::from("#FF0000");

        assert_colors_eq(doc.get_colors(), &[(1.0, 0.0, 0.0, 1.0, 0, 0, 0, 7)]);

        doc.edit(&TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "#00FF00".to_string(),
        });
        assert_eq!(doc.to_string(), "#00FF00");

        assert_colors_eq(doc.get_colors(), &[(0.0, 1.0, 0.0, 1.0, 0, 0, 0, 7)]);
    }

    #[test]
    fn append_end() {
        let mut doc = Document::from("#FF0000");

        assert_colors_eq(doc.get_colors(), &[(1.0, 0.0, 0.0, 1.0, 0, 0, 0, 7)]);

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
        assert_eq!(doc.to_string(), "#FF0000\n#00FF00");

        assert_colors_eq(
            doc.get_colors(),
            &[
                (1.0, 0.0, 0.0, 1.0, 0, 0, 0, 7),
                (0.0, 1.0, 0.0, 1.0, 1, 0, 1, 7),
            ],
        );
    }

    #[test]
    fn append_middle() {
        let mut doc = Document::from("#FF0000\n#00FF00\n#0000FF");

        assert_colors_eq(
            doc.get_colors(),
            &[
                (1.0, 0.0, 0.0, 1.0, 0, 0, 0, 7),
                (0.0, 1.0, 0.0, 1.0, 1, 0, 1, 7),
                (0.0, 0.0, 1.0, 1.0, 2, 0, 2, 7),
            ],
        );

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
            text: "\n#000000\n".to_string(),
        });
        assert_eq!(doc.to_string(), "#FF0000\n#000000\n#00FF00\n#0000FF");

        assert_colors_eq(
            doc.get_colors(),
            &[
                (1.0, 0.0, 0.0, 1.0, 0, 0, 0, 7),
                (0.0, 0.0, 0.0, 1.0, 1, 0, 1, 7),
                (0.0, 1.0, 0.0, 1.0, 2, 0, 2, 7),
                (0.0, 0.0, 1.0, 1.0, 3, 0, 3, 7),
            ],
        );
    }

    #[test]
    fn delete_color_line() {
        let mut doc = Document::from("#FF0000\n#00FF00\n#0000FF");

        assert_colors_eq(
            doc.get_colors(),
            &[
                (1.0, 0.0, 0.0, 1.0, 0, 0, 0, 7),
                (0.0, 1.0, 0.0, 1.0, 1, 0, 1, 7),
                (0.0, 0.0, 1.0, 1.0, 2, 0, 2, 7),
            ],
        );

        // Delete the middle line
        doc.edit(&TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 2,
                    character: 0,
                },
            }),
            range_length: None,
            text: "".to_string(),
        });
        assert_eq!(doc.to_string(), "#FF0000\n#0000FF");

        assert_colors_eq(
            doc.get_colors(),
            &[
                (1.0, 0.0, 0.0, 1.0, 0, 0, 0, 7),
                (0.0, 0.0, 1.0, 1.0, 1, 0, 1, 7),
            ],
        );
    }

    #[test]
    fn delete_one_char() {
        let mut doc = Document::from("#FF0000");

        assert_colors_eq(doc.get_colors(), &[(1.0, 0.0, 0.0, 1.0, 0, 0, 0, 7)]);

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
        assert_eq!(doc.to_string(), "#FF000");

        assert!(doc.get_colors().is_empty());
    }

    #[test]
    fn replace_partial_line() {
        let mut doc = Document::from("#FF0000");

        assert_colors_eq(doc.get_colors(), &[(1.0, 0.0, 0.0, 1.0, 0, 0, 0, 7)]);

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
        assert_eq!(doc.to_string(), "#FF00FF");

        assert_colors_eq(doc.get_colors(), &[(1.0, 0.0, 1.0, 1.0, 0, 0, 0, 7)]);
    }

    #[test]
    fn clear_document_then_add_from() {
        let mut doc = Document::from("#FF0000\n#00FF00");

        assert_colors_eq(
            doc.get_colors(),
            &[
                (1.0, 0.0, 0.0, 1.0, 0, 0, 0, 7),
                (0.0, 1.0, 0.0, 1.0, 1, 0, 1, 7),
            ],
        );

        // Clear all content (simulate full replace)
        doc.edit(&TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "".to_string(),
        });
        assert_eq!(doc.to_string(), "");

        assert!(doc.get_colors().is_empty());

        // Add a new color
        doc.edit(&TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "#FFFFFF".to_string(),
        });
        assert_eq!(doc.to_string(), "#FFFFFF");

        assert_colors_eq(doc.get_colors(), &[(1.0, 1.0, 1.0, 1.0, 0, 0, 0, 7)]);
    }

    #[test]
    fn multiple_incremental_edits() {
        let mut doc = Document::from("#FF0000");

        assert_colors_eq(doc.get_colors(), &[(1.0, 0.0, 0.0, 1.0, 0, 0, 0, 7)]);

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
        assert_eq!(doc.to_string(), "#FF0000\n#00FF00");

        assert_colors_eq(
            doc.get_colors(),
            &[
                (1.0, 0.0, 0.0, 1.0, 0, 0, 0, 7),
                (0.0, 1.0, 0.0, 1.0, 1, 0, 1, 7),
            ],
        );

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
        assert_eq!(doc.to_string(), "#FF0000\n#00FF00\n#0000FF");

        assert_colors_eq(
            doc.get_colors(),
            &[
                (1.0, 0.0, 0.0, 1.0, 0, 0, 0, 7),
                (0.0, 1.0, 0.0, 1.0, 1, 0, 1, 7),
                (0.0, 0.0, 1.0, 1.0, 2, 0, 2, 7),
            ],
        );
    }
}
