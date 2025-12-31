use std::collections::HashMap;

use tokio::sync::RwLock;
use tower_lsp_server::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp_server::lsp_types::*;
use tower_lsp_server::{Client, LanguageServer, LspService, Server};

use chroma_ls::document::Document;

struct Backend {
    documents: RwLock<HashMap<Uri, Document>>,
}

impl Backend {
    fn new(_client: Client) -> Self {
        Self {
            documents: RwLock::new(HashMap::new()),
        }
    }
}

impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        ..Default::default()
                    },
                )),
                color_provider: Some(ColorProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "chroma-ls".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;
        let mut documents = self.documents.write().await;

        documents.insert(uri, Document::from(content.as_str()));
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let mut documents = self.documents.write().await;

        // TODO: warn about error.
        let document = documents
            .get_mut(&uri)
            .expect("document must exist on didChange");
        for change in params.content_changes {
            document.edit(&change);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        let mut documents = self.documents.write().await;

        documents.remove(&uri);
    }

    async fn document_color(&self, params: DocumentColorParams) -> Result<Vec<ColorInformation>> {
        let uri = params.text_document.uri;
        let documents = self.documents.read().await;

        let document = documents.get(&uri).ok_or_else(|| Error {
            code: ErrorCode::InternalError,
            message: format!("Document not found for {} URI", uri.as_str()).into(),
            data: None,
        })?;
        let colors = document.get_colors();
        Ok(colors)
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
