use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::RwLock;

use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::lsp_types::*;
use tower_lsp_server::{Client, LanguageServer, LspService, Server};

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

struct Backend {
    #[allow(dead_code)]
    client: Client,
    documents: RwLock<HashMap<Uri, String>>,
}

impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        // TODO: support INCREMENTAL.
                        change: Some(TextDocumentSyncKind::FULL),
                        ..Default::default()
                    },
                )),
                color_provider: Some(ColorProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "chroma-ls".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;

        self.documents.write().await.insert(uri, text);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            self.documents.write().await.insert(uri, change.text);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.write().await.remove(&uri);
    }

    async fn document_color(&self, params: DocumentColorParams) -> Result<Vec<ColorInformation>> {
        let uri = params.text_document.uri;
        let docs = self.documents.read().await;
        let Some(text) = docs.get(&uri) else {
            return Ok(vec![]);
        };

        let mut colors = Vec::new();
        for (line_idx, line_text) in text.lines().enumerate() {
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
        Ok(colors)
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: RwLock::new(HashMap::new()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
