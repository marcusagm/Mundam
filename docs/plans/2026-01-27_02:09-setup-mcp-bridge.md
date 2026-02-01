# Plan: Setup Tauri MCP Bridge Plugin

Configuração do plugin Tauri MCP Bridge para permitir que o assistente AI interaja com o backend e o estado da aplicação.

## Steps

1. **Fix MCP Server Installation**:
   - Resolve the npm cache permission issue.
   - Install the MCP server configuration for VS Code/Claude.

2. **Add Rust Dependency**:
   - Add `tauri-plugin-mcp-bridge` to `src-tauri/Cargo.toml`.

3. **Register Plugin in Rust**:
   - Initialize the plugin in `src-tauri/src/lib.rs`.

4. **Install Frontend Bindings**:
   - Install `@hypothesi/tauri-plugin-mcp-bridge` via npm.

5. **Verification**:
   - Compile and ensure the plugin loads correctly.

## Verification Criteria
- [x] `Cargo.toml` contains `tauri-plugin-mcp-bridge`.
- [x] `lib.rs` has `.plugin(tauri_plugin_mcp_bridge::init())`.
- [x] `package.json` contains `@hypothesi/tauri-plugin-mcp-bridge`.
- [x] MCP Server is successfully registered in the AI client.
