use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::RwLock;

use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::lsp_types::*;
use tower_lsp_server::{Client, LanguageServer, LspService, Server};

fn color_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"#([0-9A-Fa-f]{6,8})").unwrap())
}

// TODO: add tests.
fn hex_to_color(hex: &str) -> Option<Color> {
    let len = hex.len();
    let (r, g, b, a) = match len {
        6 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            255,
        ),
        8 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            u8::from_str_radix(&hex[6..8], 16).ok()?,
        ),
        _ => return None,
    };
    Some(Color {
        red: r as f32 / 255.0,
        green: g as f32 / 255.0,
        blue: b as f32 / 255.0,
        alpha: a as f32 / 255.0,
    })
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
                let start_char = line_text[..mat.start()].chars().count() as u32;
                let end_char = line_text[..mat.end()].chars().count() as u32;

                let hex = &line_text[mat.start() + 1..mat.end()];
                if let Some(color) = hex_to_color(hex) {
                    let range = Range {
                        start: Position {
                            line: line_idx as u32,
                            character: start_char,
                        },
                        end: Position {
                            line: line_idx as u32,
                            character: end_char,
                        },
                    };
                    colors.push(ColorInformation { range, color });
                }
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
