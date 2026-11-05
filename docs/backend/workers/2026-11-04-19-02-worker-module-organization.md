# Organização dos módulos e revisão de pendências

> **Estado consolidado:** ciclo encerrado e validado pelo usuário em Windows/macOS. Este documento preserva o relato de sua etapa; referências a próximas entregas e validações pendentes devem ser lidas nesse contexto histórico. Consulte o [encerramento](2026-11-04-22-18-workers-closure.md) e as [evoluções futuras separadas](2026-11-04-22-18-workers-future-evolutions.md).


Revisão de 4 de novembro de 2026. A visão geral atualizada está em [overview.md](../overview.md). O caminho informado como `overview/.md` corresponde ao arquivo existente `docs/backend/overview.md`.

## Organização aplicada

| Responsabilidade | Localização |
| --- | --- |
| Contratos das etapas e porta de processamento | `src-tauri/src/core/processing/mod.rs` |
| Orçamento, controle de admissão e diagnósticos | `core/processing/{budget,control,diagnostics}.rs` |
| Contratos de comparação e publicação | `core/duplicates/matching.rs` |
| Política pura de caminhos da lixeira | `core/trash/paths.rs`, exposta por `core/trash/mod.rs` |
| Consultas de outcomes e controles dos workers | `feature/processing/` |
| Fachada SQLite da fila e testes reais | `infra/database/processing/` |
| Leituras de pendências, status, contagens e diagnósticos | `infra/database/query_handlers/processing/` |
| Intenção, admissão, tentativas, fencing e retry | `infra/database/handlers/processing/` |
| Predicados comuns de elegibilidade | `infra/database/processing/selection.rs` |
| Fachada de duplicados e testes de matching | `infra/database/duplicates/` |
| Status, inputs transacionais e leitura de política | `infra/database/query_handlers/matching/` e `query_handlers/duplicate_policy.rs` |
| Claims, renovação, cancelamento e publicação de matching | `infra/database/handlers/matching/` |
| Adoção de hierarquia e testes de remoção de pastas | `infra/database/handlers/folders/` |
| Limpeza de thumbnails e previews gerados | `infra/filesystem/asset_cache.rs` |
| Executors e testes específicos | `processing/workers/{thumbnails,colors,duplicates,trash}/` |
| Fixtures comuns dos executors | `processing/workers/test_support.rs` |

As fachadas conservam os contratos de repositório. SQL fica nos módulos de consulta e mutação, com os mesmos limites de transação e de publicação. Leituras necessárias à validação de um claim ou commit permanecem dentro da transação do handler. O retry de fingerprints e comparações continua atômico.

Dois casos legados misturavam consultas e mutações: a criação da regra padrão de duplicados e a adoção de pastas órfãs. A criação padrão agora é um handler explícito chamado pela fachada; a consulta de política é read-only. A adoção usa o handler de hierarquia também para os chamadores do antigo port de consultas. Esse método legado do port continua existindo para preservar seu contrato; uma migração completa desse port para CQRS estrito seria um trabalho separado.

Os tipos públicos dos workers continuam disponíveis pela fachada `processing::workers`. Não foram criados módulos de compatibilidade nos caminhos antigos. O arquivo incluído pelo teste da migration de remoção usa `CARGO_MANIFEST_DIR`, evitando depender da profundidade da pasta.

## Frontend

- `core/store/processing/` agrupa cliente, contratos, read model de thumbnails e testes. O polling continua compartilhado pelo hook existente.
- `components/features/statusbar/processing/` agrupa o painel, linhas, ações de retry, diagnósticos, apresentação, CSS e testes.
- O tracker de publicações de matching e seus testes ficam em `components/features/duplicates/state/`, pois só a feature de duplicados o consome.
- Configurações continuam na feature `settings`, com os componentes e testes de performance junto dos demais painéis. Utilitários de apresentação compartilhados permanecem em `src/utils/processing.ts`.

Imports de código, mocks e testes foram atualizados. A reorganização não altera os nomes dos comandos IPC, snapshots, políticas de pausa ou regras de similaridade.

## Auditoria de arquivos sem uso

```sh
node scripts/audit-source-files.mjs --strict
```

O script segue entradas HTML, imports/exports estáticos e dinâmicos, imports de workers do Vite, URLs de workers, CSS e testes. No Rust, percorre módulos da biblioteca/binário e módulos com `#[path]`, incluindo testes condicionais. O [relatório desta revisão](2026-11-04-19-13-source-file-audit.json) registra, após a entrega de disponibilidade de ferramentas, 638 arquivos frontend e 355 arquivos Rust, sem candidatos órfãos ou referências não resolvidas no escopo do script. O resultado é uma auditoria conservadora de arquivos, sem afirmar que todos os exports de um arquivo são executados em runtime. Declarações ambientes e quatro fachadas públicas são classificadas separadamente.

Foram removidos oito arquivos frontend sem importadores de aplicação, testes ou ferramentas:

- `src/App.css`: estilo remanescente do template inicial.
- `src/components/features/duplicates/mockData.ts`: dados de exemplo sem consumidores.
- `src/components/features/viewport/components/DragOverlay.tsx` e `DragOverlay.css`: overlay antigo sem chamadas no fluxo de DnD atual.
- `src/core/store/selection/schemas.ts`: schema não utilizado com IDs numéricos, distinto do contrato atual.
- `src/core/system/featureFlags.ts`: flags sem consumidores.
- `src/core/utils/actions.ts`: alternativa de validação de ações sem importadores.
- `src/types/events.ts`: cópia incompleta e não utilizada de eventos de domínio.

No Rust, `processing/media/extractors/test_dds.rs` foi removido por não pertencer à árvore de módulos. Ele continha diagnósticos manuais por `println!` e caminhos locais de amostras, sem assertions; não representava cobertura executada pelo Cargo.

As fachadas `statusbar/index.ts` e `core/input/{primitives,providers,store}/index.ts` foram preservadas como entradas públicas. Os providers `PointerProvider` e `GestureProvider` continuam expostos por sua fachada, mas não são ativados pelo `InputProvider` atual. Essa condição é reportada como `publicOnly`, sem classificá-los como código ativo nem habilitar listeners durante a limpeza.

## Situação no encerramento

O usuário confirmou a validação funcional das entregas em Windows/macOS e bom desempenho do processamento completo em bibliotecas representativas. Não houve benchmark ponta a ponta; sua execução foi dispensada como condição de aceite.

| Parte do plano | Situação final |
| --- | --- |
| Correções, elegibilidade, tentativas e coordenação | Entregues. |
| Reservas persistentes e publicação transacional | Entregues para extrações e comparações. |
| Snapshot, pausa/retomada, retries, diagnóstico e limites dinâmicos | Entregues e integrados à interface. |
| Progresso e otimização do matching | Coalescência, variantes preparadas, crop sob corroboração, logs por fase e benchmarks sintéticos Windows/macOS entregues. |
| Suspensão por ferramenta e probes | Entregues e validados pelo usuário. |
| Validação funcional e de desempenho em uso | Aceita pelo usuário nas duas plataformas. Sem benchmark ponta a ponta adicional. |
| Matching incremental persistente | Adiado explicitamente; fora do escopo concluído. |

Identidade de execução do indexer, verificação física da origem, versionamento por algoritmo e opções adicionais estão nas [evoluções futuras](2026-11-04-22-18-workers-future-evolutions.md), com seus motivos, benefícios e limites. Não são pendências para encerrar este ciclo. O [registro de encerramento](2026-11-04-22-18-workers-closure.md) distingue aceite do usuário, evidência automatizada e medições históricas.

## Validação da reorganização

No Windows, a suíte completa da biblioteca passou: **219 testes**, incluindo processamento, lixeira, hierarquia, permissões IPC e extratores com as amostras disponíveis. O teste novo verifica que a consulta de política funciona com SQLite em modo read-only e que a criação padrão permanece explícita e idempotente no adaptador.

A suíte completa frontend passou: **116 testes em 23 arquivos**, incluindo os testes e mocks movidos. TypeScript, ESLint dos arquivos alterados com `--max-warnings 0`, Prettier e build de produção com `--configLoader native` passaram. A auditoria estrita passou sem candidatos órfãos e os links locais foram verificados.

Essa validação foi executada no Windows. A confirmação anterior de uso no macOS permanece registrada separadamente; esta reorganização ainda não foi executada nativamente no macOS nesta sessão. Não há migration de banco ou alteração dos formatos de cache. O documento histórico `05-job-scheduler-workers.md` permanece inalterado.
