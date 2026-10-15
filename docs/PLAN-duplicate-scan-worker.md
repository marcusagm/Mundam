# Plano de Implementação: Background Scanning e Resiliência de Estado no Módulo de Duplicados

## Contexto e Objetivos

O usuário identificou duas falhas críticas na experiência do módulo de duplicados:

1. **P0 — Resiliência de Estado e Concorrência na Varredura de Duplicados**:
   - Ao trocar de tela pela TitleBar durante uma varredura, a interface perde o estado (`isScanning` é reiniciado como `false`).
   - Ao clicar novamente em "Iniciar Scan", a UI fica travada em "Preparing scan..." porque uma varredura anterior ainda está rodando no SQLite e o motor de matching visual ($O(N^2)$) não checa cancelamento, causando contenção de locks no SQLite e bloqueando a resposta do IPC.
   - **Objetivo**: Implementar persistência de status do scan no backend (`get_duplicate_scan_status`), proteção contra execução concorrente duplicada, verificação de cancelamento cooperativo no loop de matching visual, e reidratação imediata de progresso no frontend no `onMount`.

2. **P0 — Background Worker Autônomo para Detecção em Ingestão Limpa**:
   - Ao iniciar a biblioteca do zero, o `BatchCreate` de arquivos não emite `DomainEvent::AssetCreated` individual para evitar overflow do canal de eventos.
   - O `DuplicateWorker` apenas escutava `AssetCreated`, ficando totalmente inativo durante a ingestão inicial.
   - Enquanto o `ThumbnailWorker` possui um loop de background autônomo com semáforo, o `DuplicateWorker` não gerava nenhuma fingerprint nem agrupava cópias exatas, mesmo com `auto_detect_duplicates = true` (padrão).
   - **Objetivo**: Transformar o `DuplicateWorker` em um background worker ativo (semelhante ao `ThumbnailWorker`), com polling inteligente de ativos sem fingerprint, processamento em lotes paralelos e agrupamento automático de duplicados exatos em background.

---

## Modificações Propostas

### 1. Backend: Controle de Estado e Consulta de Status de Varredura

#### [MODIFY] [`src-tauri/src/feature/duplicates/commands.rs`](file:///Users/marcusmaia/Documents/Desenvolvimento/Mundam/src-tauri/src/feature/duplicates/commands.rs)

1. Adicionar struct `DuplicateScanStatus`:
   ```rust
   #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
   pub struct DuplicateScanStatus {
       pub is_running: bool,
       pub phase: String,
       pub processed: usize,
       pub total: usize,
   }
   ```
2. Adicionar campo `scan_status: Arc<std::sync::RwLock<DuplicateScanStatus>>` no `DuplicateCommandService`.
3. Em `start_duplicate_scan()`:
   - Se `scan_status.read().is_running == true`, não iniciar nova varredura em paralelo: registrar log e retornar `Ok(())` imediatamente.
   - Atualizar a fase em cada etapa: `"hashing"`, `"exact_matching"`, `"visual_matching"`.
   - Garantir que `is_running` seja revertido para `false` ao término ou em caso de erro através de um guard RAII ou bloco deferido.
4. Adicionar método público:
   ```rust
   pub fn get_scan_status(&self) -> DuplicateScanStatus {
       self.scan_status.read().unwrap_or_else(|poison| poison.into_inner()).clone()
   }
   ```

#### [MODIFY] [`src-tauri/src/infra/database/handlers/duplicates_handler.rs`](file:///Users/marcusmaia/Documents/Desenvolvimento/Mundam/src-tauri/src/infra/database/handlers/duplicates_handler.rs)

1. No loop principal de [`run_visual_match_scan`](file:///Users/marcusmaia/Documents/Desenvolvimento/Mundam/src-tauri/src/infra/database/handlers/duplicates_handler.rs#L689), adicionar verificação cooperativa de cancelamento:
   ```rust
   for primary_index in 0..total_assets_count {
       if let Some(token) = &cancellation_token {
           if token.is_cancelled() {
               tracing::info!("run_visual_match_scan cancelled");
               break;
           }
       }
       // ...
   ```

#### [MODIFY] [`src-tauri/src/delivery/tauri/commands/duplicates.rs`](file:///Users/marcusmaia/Documents/Desenvolvimento/Mundam/src-tauri/src/delivery/tauri/commands/duplicates.rs) & [`src-tauri/src/lib.rs`](file:///Users/marcusmaia/Documents/Desenvolvimento/Mundam/src-tauri/src/lib.rs)

1. Expor o comando Tauri IPC:
   ```rust
   #[tauri::command]
   pub async fn get_duplicate_scan_status(
       command_service: State<'_, DuplicateCommandService>,
   ) -> AppResult<DuplicateScanStatus> {
       Ok(command_service.get_scan_status())
   }
   ```
2. Registrar o comando no Tauri Builder em `lib.rs`.

---

### 2. Backend: Background Worker Ativo no `DuplicateWorker`

#### [MODIFY] [`src-tauri/src/core/repository/duplicates.rs`](file:///Users/marcusmaia/Documents/Desenvolvimento/Mundam/src-tauri/src/core/repository/duplicates.rs) & [`src-tauri/src/infra/database/duplicates.rs`](file:///Users/marcusmaia/Documents/Desenvolvimento/Mundam/src-tauri/src/infra/database/duplicates.rs)

1. Adicionar método de porta no `DuplicatesRepository`:
   ```rust
   async fn get_assets_needing_fingerprints(
       &self,
       limit: u32,
   ) -> AppResult<Vec<(String, String, String)>>;
   ```
2. Implementar no SQLite com query de alto desempenho:
   ```sql
   SELECT a.id, a.path, a.format_type
   FROM assets a
   LEFT JOIN duplicate_fingerprints df ON a.id = df.asset_id
   WHERE df.asset_id IS NULL AND a.state != 'trashed'
   LIMIT ?
   ```

#### [MODIFY] [`src-tauri/src/feature/duplicates/events.rs`](file:///Users/marcusmaia/Documents/Desenvolvimento/Mundam/src-tauri/src/feature/duplicates/events.rs)

1. Estender o `DuplicateWorker::start`:
   - Além de receber eventos do `event_bus` para inserções unitárias (`DomainEvent::AssetCreated`), iniciar uma rotina concorrente de background polling (`background_fingerprint_loop`).
   - A rotina verifica periodicamente se `auto_detect_duplicates` está ativo nas configurações.
   - Se ativo, busca lotes de até 10 ativos sem fingerprint via `get_assets_needing_fingerprints`.
   - Gera fingerprints em `spawn_blocking` consumindo `resolve_image_via_format_provider` e salva no banco.
   - Após cada lote processado com sucesso, executa `run_exact_match_scan(None)`, agrupando imediatamente cópias idênticas e emitindo `DomainEvent::DuplicateGroupCreated`.
   - Se não houver trabalho pendente ou se `auto_detect_duplicates` estiver desativado, dorme cooperativamente via `tokio::select!` com verificação de `token.cancelled()`.

---

### 3. Frontend: Reidratação de Estado e Prevenção de Travamentos

#### [MODIFY] [`src/lib/duplicates.ts`](file:///Users/marcusmaia/Documents/Desenvolvimento/Mundam/src/lib/duplicates.ts)

1. Definir tipo:
   ```ts
   export interface DuplicateScanStatus {
       is_running: boolean;
       phase: string;
       processed: usize;
       total: usize;
   }
   ```
2. Adicionar método à API:
   ```ts
   getDuplicateScanStatus: async (): Promise<DuplicateScanStatus> => {
       return invoke<DuplicateScanStatus>('get_duplicate_scan_status');
   }
   ```

#### [MODIFY] [`src/components/features/duplicates/hooks/useDuplicateGroups.ts`](file:///Users/marcusmaia/Documents/Desenvolvimento/Mundam/src/components/features/duplicates/hooks/useDuplicateGroups.ts)

1. No `onMount` (ou inicialização do hook):
   - Chamar `duplicatesApi.getDuplicateScanStatus()`.
   - Se `status.is_running`:
     - Sincronizar `setIsScanning(true)`.
     - Sincronizar `setScanProgress({ processed: status.processed, matched: 0, groupsCreated: 0, total: status.total })`.
2. Desta forma, ao navegar da tela de duplicatas para a galeria e retornar, a UI imediatamente consulta o backend e exibe a barra de progresso em andamento sem regressar ao estado inicial e sem dar a falsa impressão de que a varredura foi abortada.

---

## Verificação e Testes

1. **Compilação Backend**: `cargo check` em `src-tauri`.
2. **Testes Unitários Backend**: `cargo test --lib feature::duplicates`.
3. **Checagem de Tipos Frontend**: `npm run typecheck`.
4. **Testes Unitários Frontend**: `npm test`.
5. **Teste de Ciclo de Vida**:
   - Iniciar scan -> navegar para Home/Galeria -> retornar para Duplicatas -> verificar se a UI mantém o status de progresso ativo.
   - Verificar se clicar em "Iniciar Scan" com um scan em andamento não trava nem enfileira operações duplicadas.
   - Verificar se ativos importados em lote (simulando inicialização limpa) são processados automaticamente em background.
