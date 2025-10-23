# chroma-ls

Tiny LSP server for highlighting color literals in source files.
It implements only the [textDocument/documentColor](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#textDocument_documentColor) method.
<img alt="Preview" src="https://github.com/user-attachments/assets/b53bf537-4169-4e40-be88-6e7f803d4c24" />

## Installation

### Using cargo

```bash
cargo install chroma-ls
```

## Editor Configuration

### Neovim

Create `lsp/chroma_ls.lua`:

```lua
---@type vim.lsp.Config
return {
  cmd = { "chroma-ls" }
}
```

> With no filetypes provided, it will be active in all buffers.

Enable the LSP:

```lua
vim.lsp.enable("chroma_ls")
```


### Helix

In `languages.toml`:

```toml
[language-server.chroma]
command = "chroma-ls"
```

Helix [does not currently](https://github.com/helix-editor/helix/issues/12721) support assigning an LSP globally to all filetypes.
You need to specify the languages explicitly. For example:

```toml
[[language]]
name = "json"
language-servers = [ "chroma" ]
```
