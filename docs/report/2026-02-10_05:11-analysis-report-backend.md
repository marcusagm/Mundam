# Relatório de Análise Técnica: Backend (Mundam)
> **Escopo**: Diretório `src-tauri/` (Rust)
> **Data**: 10 de Fevereiro de 2026

## 1. Arquitetura e Engenharia de Software

O backend foi construído utilizando **Rust**, aproveitando o ecossistema robusto do **Tauri**. A arquitetura segue princípios de *modular monolith*, onde funcionalidades distintas são encapsuladas em módulos crate-level (`mod.rs`), mantendo o binário único para facilitar a distribuição e o IPC (Inter-Process Communication).

### Estrutura de Módulos
*   **`database`**: Camada de persistência. Abstração sobre o SQLite via `sqlx`.
*   **`indexer`**: O "motor" de ingestão. Responsável por monitorar o sistema de arquivos (`notify`), percorrer diretórios (`walkdir`) e manter o banco de dados sincronizado.
*   **`streaming`**: Servidor HLS customizado (Axum) para entrega de vídeo. Uma peça de engenharia notável que permite streaming de qualquer formato de vídeo para o frontend web.
*   **`thumbnails`**: Pipeline de geração de previews. Implementa o padrão *Strategy* para lidar com dezenas de formatos diferentes (Imagens, Vídeos, Fonts, 3D, ZIPs proprietários).
*   **`protocols`**: Handlers de protocolo customizados (`orig://`, `thumb://`) para servir arquivos locais de forma segura ao WebView.

**Avaliação**: A arquitetura é de nível profissional. O uso de `Tokio` para assincronismo em I/O intensivo (scanning/watching) demonstra domínio da linguagem e do problema.

---

## 2. Destaques da Implementação

### 2.1 Indexador (`src-tauri/src/indexer/mod.rs`)
Responsável por manter o estado da biblioteca em sincronia com o disco.
*   **✅ Pontos Fortes**:
    *   **Monitoramento Reativo**: Usa a crate `notify` para detectar mudanças em tempo real. A lógica de *debounce* (600ms) evita processamento excessivo em operações em lote.
    *   **Heurísticas de Rename**: O watcher implementa lógica inteligente para detectar quando um "delete" seguido de um "add" é, na verdade, um "rename" (movimentação de arquivo), preservando metadados e tags. Isso é crucial para a UX.
    *   **Worker Pattern**: Separação clara entre a *Producer* (scanner) e o *Consumer* (DB worker) via canais (`mpsc`), permitindo processamento paralelo.
*   **⚠️ Risco**: A função `ensure_folder_hierarchy` pode se tornar um gargalo em discos lentos com milhares de pastas aninhadas, pois faz muitas queries sequenciais de "upsert". Um *bulk insert* seria mais performático.
    *   ✅ **Otimizado (2026-02-10)**: Implementada lógica iterativa e atômica. O uso de uma única transação para toda a árvore de diretórios elimina o gargalo de I/O em discos lentos, consolidando as queries de upsert em um único flush de disco.

### 2.2 Streaming Server (`src-tauri/src/streaming/server.rs`)
Um servidor HTTP completo embutido no app.
*   **✅ Pontos Fortes**:
    *   **Axum**: Escolha moderna e performática para o servidor web interno.
    *   **Clean Architecture**: Separação clara entre Rotas (`_handler`), Lógica (`probe`, `playlist`) e Estado (`AppState`).
    *   **Gestão de Recursos**: Tasks de background para limpeza de processos órfãos (`cleanup_stale`) e sessões lineares inativas, prevenindo vazamento de memória/processos FFmpeg.
    *   **Protocolo HLS**: Implementação correta de HLS (VOD e Live/Linear) permite *seeking* instantâneo e adaptação de qualidade.

### 2.3 Geração de Thumbnails (`src-tauri/src/thumbnails/mod.rs`)
*   **✅ Pontos Fortes**:
    *   **Strategy Pattern**: `ThumbnailStrategy` define claramente como cada arquivo deve ser tratado.
    *   **Priorização**: A lógica tenta usar FFmpeg (processo externo rápido) primeiro, caindo para implementações nativas (Rust puro) ou extratores específicos em caso de falha.
    *   **Cache**: Nomes de arquivos baseados em hash simples do caminho (`DefaultHasher`), garantindo consistência. (Nota: Hash do conteúdo seria mais seguro contra colisões em renomeações, mas o path hash é muito mais rápido).

---

## 3. Segurança e Performance

*   **Concurrency**: O uso extensivo de `Arc<Mutex<...>>` e `Arc<RwLock<...>>` (ex: `WatcherRegistry`, `ProcessManager`) mostra cuidado com o acesso concorrente aos recursos compartilhados.
*   **Tratamento de Erros**: Embora existam alguns `unwrap()` em locais de inicialização (o que é aceitável para *fail-fast* no boot), a maioria das operações de I/O retorna `Result` e logs de erro apropriados.
*   **Privacidade**: Sendo *Local-First*, todos os dados ficam na máquina do usuário. O servidor de streaming roda em `127.0.0.1`, isolado da rede externa.

---

## 4. Recomendações Técnicas

1.  **Migrações de Banco** ✅ *Resolvido*: Como mencionado no relatório geral, substituir a criação manual de tabelas em `database.rs` por `sqlx migrate`. Isso é a dívida técnica mais urgente.
2.  **Otimização do Indexer** ✅ *Resolvido*: Em bibliotecas gigantes (>100k arquivos), o `WalkDir` inicial pode demorar. Considerar persistir o "last scan time" e scanear apenas diretórios modificados na inicialização.
3.  **Refatoração de Erros**: Criar um tipo `AppError` centralizado (usando `thiserror`) para padronizar o retorno de erros para o frontend, em vez de retornar strings ou imprimir no console.

## 5. Conclusão do Backend

O backend do Mundam é uma peça de engenharia sólida. Ele resolve problemas complexos (como streaming de vídeo e watch de sistema de arquivos) de maneira eficiente e manutenível.

*   **Arquitetura**: ⭐⭐⭐⭐⭐
*   **Performance**: ⭐⭐⭐⭐⭐ (Rust + Tokio + Axum)
*   **Robustez**: ⭐⭐⭐⭐ (Precisa melhorar migrações DB e padronização de erros)
*   **Inovação**: ⭐⭐⭐⭐⭐ (Servidor de Streaming embutido é um diferencial enorme)
