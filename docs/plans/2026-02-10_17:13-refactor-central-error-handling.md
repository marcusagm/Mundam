# Refatoração: Centralização do Gerenciamento de Erros (Backend)

## Contexto
O backend em Rust utilizava `Result<T, String>` na maioria dos comandos Tauri, convertendo erros de forma ad-hoc com `.map_err(|e| e.to_string())`. Esta refatoração implementa um sistema de erros centralizado e tipado para melhor manutenibilidade e comunicação com o frontend.

## Objetivos
- [x] Implementar enum `AppError` usando `thiserror`.
- [x] Padronizar todos os comandos Tauri para usar `AppResult<T>`.
- [x] Melhorar a tipagem de erros internos (SQLx, IO, Tauri).
- [x] Garantir serialização correta para o IPC do Tauri.

## Passo a Passo

### 1. Preparação do Ambiente
- Adicionada a dependência `thiserror` ao `Cargo.toml`.
- Criado o arquivo `src-tauri/src/error.rs`.

### 2. Definição do Tipo de Erro
- Implementado o enum `AppError` com as seguintes variantes:
    - `Db`: Encapsula `sqlx::Error`.
    - `Migration`: Encapsula `sqlx::migrate::MigrateError`.
    - `Tauri`: Encapsula `tauri::Error`.
    - `Io`: Encapsula `std::io::Error`.
    - `Transcoding`: Erros específicos de processamento de mídia.
    - `NotFound`: Para recursos ausentes.
    - `Internal`: Erros de estado da aplicação.
    - `Generic`: Erros gerais com mensagem baseada em string.

### 3. Integração do Módulo
- Registrado o módulo `error` em `lib.rs` como `pub mod error`.
- Criado o type alias `pub type AppResult<T> = Result<T, AppError>`.

### 4. Refatoração dos Comandos
Os seguintes arquivos de comando foram refatorados para substituir `Result<T, String>` por `AppResult<T>`:
- `lib.rs` (comando `start_indexing`)
- `tag_commands.rs`
- `location_commands.rs`
- `metadata_commands.rs`
- `smart_folder_commands.rs`
- `thumbnail_commands.rs`
- `settings_commands.rs`
- `audio_commands.rs`
- `transcoding/commands.rs`

### 5. Otimização do Fluxo Interno
- Métodos da struct `Db` em `db/mod.rs` (`new`, `run_maintenance`) foram atualizados para retornar `AppResult`.
- Funções em `ffmpeg.rs` foram refatoradas para utilizar os novos tipos de erro estruturados e o operador `?`.

### 6. Verificação e Ajustes
- Executado `cargo check` para garantir a integridade dos tipos.
- Corrigida a tipagem em `generate_with_ffmpeg` e `get_audio_waveform`.
- Adicionada conversão para `MigrateError` e `tauri::Error` no enum central.

## Resultados
- **Código mais limpo**: Redução significativa de `.map_err(|e| e.to_string())`.
- **Melhor IPC**: O frontend agora recebe erros estruturados em vez de strings brutas.
- **Conformidade**: O código agora segue as diretrizes estabelecidas em `docs/guidelines/backend-rust.md`.

---
**Data**: 10 de Fevereiro de 2026
**Status**: Concluído
