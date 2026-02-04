# Implementação: Centralização de Configurações e UI de Preferências

## Objetivo
Centralizar configurações "hardcoded" espalhadas pelo código (ex: `BATCH_SIZE`, limites de threads) em módulos dedicados e expor opções relevantes para o usuário no painel de configurações (`GeneralPanel.tsx`).

## Hardcoded Configs Identificados
1.  **Frontend (`src/core/store/libraryStore.ts`)**:
    *   `BATCH_SIZE = 100`: Tamanho do lote de carregamento de imagens.
2.  **Backend (`src-tauri/src/thumbnail_worker.rs`)**:
    *   `num_threads(2)`: Número de threads para geração de thumbnails.
    *   `get_images_needing_thumbnails(6)`: Tamanho do lote de processamento do worker.
    *   `sleep(Duration::from_secs(2))`: Tempo de espera quando ocioso.
3.  **Backend (`src-tauri/src/search_logic.rs`)**:
    *   Limites de recursividade (ex: `depth < 50`).

## Arquitetura Proposta

### 1. Backend: `Startup Config` não persistente
Para configurações de baixo nível que afetam a inicialização (como threads), usaremos variáveis de ambiente ou um arquivo `config.rs` com valores padrão, mas permitindo override via banco de dados (`app_settings`) na inicialização.

*   **Novo Módulo**: `src-tauri/src/config.rs`
    *   Struct `AppConfig` com campos como `thumbnail_threads`, `indexer_batch_size`.
    *   Carregamento: Padrão -> Override do DB.

### 2. Frontend: `Constants` e `SettingsStore`
*   **Arquivo de Constantes**: `src/config/constants.ts` para valores imutáveis de desenvolvimento (ex: timeouts de API, endpoints).
*   **Persistent Settings**: Usar a tabela `app_settings` via `db_settings.rs` para configurações do usuário.
*   **Store**: Atualizar `systemStore.ts` ou criar `settingsStore.ts` para cachear essas preferências no frontend.

## Passo a Passo de Implementação

### Passo 1: Centralização Frontend
1.  Criar `src/config/constants.ts`.
2.  Mover `BATCH_SIZE` de `libraryStore.ts` para o novo arquivo.
3.  Refatorar `libraryStore.ts` para importar de `constants.ts`.

### Passo 2: Backend Config & Threads Dinâmicas
1.  Criar `src-tauri/src/config.rs` com defaults.
2.  Ler configurações do DB na inicialização do `main.rs` ou `lib.rs` e armazenar no `State`.
3.  Atualizar `thumbnail_worker.rs` para ler `num_threads` do `State` ou de uma configuração passada no construtor.
    *   Isso permitirá que o usuário altere a performance no futuro sem recompilar.

### Passo 3: UI de Configurações (`GeneralPanel.tsx`)
Adicionar controles para:
1.  **Performance de Indexação**: Slider ou Select para "Uso de CPU" (Baixo, Médio, Alto) -> controla threads do Worker.
    *   *Nota*: Alterar threads do worker pode exigir reinicialização do app ou do worker. Para simplificar nesta iteração, salvaremos no DB e pediremos reinício.
2.  **Visualização**: Checkbox para "Mostrar animações de carregamento" ou similar (se houver).

## Plano de Verificação
1.  **Build**: Verificar se frontend e backend compilam.
2.  **Runtime**: Alterar `BATCH_SIZE` no código e verificar se a rolagem infinita adapta a frequência de requests.
3.  **UI Settings**: Alterar configuração simulada e verificar persistência no SQLite (`select * from app_settings`).
