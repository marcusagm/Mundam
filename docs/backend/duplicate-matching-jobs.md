# Comparações de duplicados persistentes

**Implementação:** 4 de novembro de 2026.

Comparações exatas e visuais agora têm intenção, reserva, retry e conclusão persistidos. O worker automático e o scan manual usam o mesmo caminho de cálculo e publicação, substituindo os dois algoritmos que gravavam diretamente pelo adaptador legado.

## Contratos e responsabilidades

| Módulo | Responsabilidade |
| --- | --- |
| [`core/duplicates/matching.rs`](../../src-tauri/src/core/duplicates/matching.rs) | Portas e DTOs de reserva, snapshot, resultados e status |
| [`infra/database/duplicates/matching.rs`](../../src-tauri/src/infra/database/duplicates/matching.rs) | Admissão exclusiva, captura dos inputs, cancelamento e retries |
| [`matching_publication.rs`](../../src-tauri/src/infra/database/handlers/matching/publication.rs) | Reconciliação dos grupos e conclusão protegida na mesma transação |
| [`feature/duplicates/matching.rs`](../../src-tauri/src/feature/duplicates/matching.rs) | Cálculo determinístico usando o matcher existente |
| [`matching_processor.rs`](../../src-tauri/src/feature/duplicates/matching_processor.rs) | Trabalho em `spawn_blocking`, heartbeat e desfecho da reserva |
| Worker e comandos de duplicados | Política automática, admissão de sessão, fases e eventos consolidados |

`DuplicatesRepository` estende `MatchingRepository`, implementado pelo mesmo adaptador SQLite. A composição injeta esse repository também no serviço de snapshots. Nenhum comando IPC novo foi acrescentado; o cancelamento existente passa a aguardar a persistência quando há comparação manual agendada.

## Intenção e versão dos inputs

A migration `20261104010000_duplicate_matching_jobs.sql` cria uma revisão global e dois registros coalescidos em `duplicate_matching_jobs`: `exact_matching` e `visual_matching`. Não existe um job de comparação por asset.

Triggers enfileiram ambas as fases na mesma transação que modifica os fingerprints, as regras ou a disponibilidade/origem de um asset. Alterações de metadados de apresentação e a transição normal para `Thumbnailed` não invalidam a comparação. Um rollback do fingerprint também desfaz o agendamento. A migration inicia uma reconciliação do catálogo existente, sujeita à preferência automática.

Cada reserva captura a revisão, a política ativa completa e os fingerprints elegíveis dentro de `BEGIN IMMEDIATE`. A política ativa usa o mesmo desempate determinístico por ID nas consultas da interface e no executor. Os inputs excluem placeholders, fingerprints anteriores à v5, assets removidos, na lixeira, offline e fingerprints cujo registro de processamento aponta para outra origem.

Mudanças durante o cálculo invalidam a publicação anterior. Os dois registros conservam a revisão nova, sem gerar milhares de jobs ou eventos. A revisão é do catálogo e das solicitações, não um snapshot físico atômico dos arquivos.

## Reserva, execução e recuperação

- Reserva exclusiva por fase, com UUID novo e prazo de 120 segundos.
- Heartbeat a cada cinco segundos. Não revive uma reserva expirada, cancelada ou superada.
- Cálculo de grupos fora do executor async, com cancelamento entre candidatos e pares.
- Liberação cooperativa ao terminar ou abortar; após crash, o prazo permite nova aquisição.
- Até três falhas por revisão, com backoff de cinco e trinta segundos. Uma mudança de inputs ou nova solicitação explícita abre outra revisão.
- O coordenador local continua serializando hashing e matching com o scan manual. As reservas acrescentam proteção entre processos da mesma base por fase.

Exemplo de uso após admissão pelo coordenador e pelo controle de pausa:

```rust,ignore
let published = matching_processor::process(
    repository.clone(),
    coordinator.clone(),
    MatchingKind::Exact,
    automatic_detection_enabled,
    &cancellation_token,
).await?;
```

`Some(count)` representa uma fase publicada. `None` pode indicar ausência de admissão, cancelamento ou revisão superada; não encerra a intenção persistida como sucesso.

O worker conserva a preferência automática e o agrupamento por lotes: comparação exata após cem fingerprints ou ao atingir ociosidade; visual ao atingir ociosidade. O contador local controla apenas a frequência, não a recuperação. Após reiniciar, o backlog persistido permite executar as comparações mesmo sem novos fingerprints ou eventos.

Um scan manual grava sua intenção de comparação depois da fase de fingerprints. Se interrompido a partir desse ponto, as comparações são recuperadas mesmo com automação desligada, respeitando a pausa da sessão e o prazo de reservas anteriores. Esta entrega não recria o fluxo de hashing de um scan manual que caiu antes de registrar sua intenção de comparação; pendências de fingerprints continuam no mecanismo existente e seguem sua política automática ou uma nova ação explícita.

Cancelar remove a intenção manual correspondente e solicita saída cooperativa, inclusive para trabalho recuperado. Enquanto a reserva manual ainda estiver ativa, o snapshot distingue a parada solicitada da saída confirmada. Intenções automáticas independentes são preservadas e seguem a preferência do usuário. Uma comparação cancelada não se torna uma obrigação manual após o próximo reinício.

## Publicação e escolhas do usuário

Cada fase é publicada em uma única transação. A validação de UUID, expiração e revisão ocorre antes e depois das gravações; resultado obsoleto ou erro desfaz a fase inteira. Comparação exata e visual são fases independentes: cancelar a visual não desfaz a exata já concluída.

O cálculo preserva a matemática existente, com ordem estável por asset e cancelamento também no loop interno de pares. Grupos de conteúdo idêntico não são duplicados pela comparação visual. Os flags e limiares efetivamente consumidos pelo matcher conservam suas semânticas anteriores; esta entrega não redefine os algoritmos.

Na reconciliação:

- Grupos abertos com a mesma composição conservam ID, canonical asset e seleção de candidatos.
- Grupos revisados, ignorados e resolvidos são preservados. Uma nova comparação não reabre esses grupos nem recria um grupo sobre os mesmos membros protegidos naquela categoria.
- Grupos abertos que deixaram de existir são removidos, sem deixar grupos abertos com zero candidatos.
- Histórico protegido só é apagado por uma ação explícita de rebuild, conforme sua confirmação existente.
- Desabilitar uma estratégia remove seus grupos abertos na próxima publicação daquela fase; não altera originais.

O registro de conclusão e `publication_sequence` avançam na mesma transação. Eventos consolidados carregam essa sequência. Se um evento se perder ou o processo cair depois do commit, o frontend recupera os resultados comparando a sequência do snapshot.

## Integração com a interface

O snapshot global acrescenta `matching`, separado das contagens de assets: comparações pendentes, aguardando retry, falhas e sequência publicada. O estado manual combina a execução local com a intenção persistida, permitindo consultar e cancelar uma comparação recuperada.

Configurações, finder e statusbar distinguem recuperação, pausa, espera por retry e cancelamento. A barra de progresso não anima durante pausa ou espera por retry. `Scan Now` solicita novas comparações; não reinicia falhas terminais da leitura de arquivos. Rebuild conserva seu alcance maior e sua confirmação.

[`MatchingPublicationTracker`](../../src/components/features/duplicates/state/matchingPublication.ts) invalida grupos quando observa nova publicação ou reinício do serviço. Um evento que já atualizou os grupos pode confirmar a mesma sequência sem outra invalidação. Amostras repetidas de atividade não recarregam biblioteca, árvore ou badges.

## Limites e próximos passos

O snapshot dos inputs e a publicação ainda abrangem a fase inteira. A comparação visual continua global e quadrática; matching incremental, orçamento comum e medições de carga continuam planejados. Não há nova configuração de concorrência dinâmica nesta etapa. A identificação da origem continua baseada no catálogo.

Retry por worker e diagnóstico agrupado foram implementados na etapa posterior de [retries e diagnóstico](worker-retries-diagnostics.md). Repetir falhas de duplicados enfileira fingerprints e comparações que falharam na mesma transação, sem exigir rebuild. Limites dinâmicos e orçamento compartilhado de operações foram entregues em [Orçamento dos workers](worker-resource-budget.md). Medição de carga e otimização orientada por resultados continuam planejadas.

## Validação

Os testes usam migrations, SQLite real e banco em arquivo fechado/reaberto. Cobrem reservas concorrentes, expiração, revisão de regras e fingerprints, rollback de publicação, preservação de escolhas, cancelamento, retries limitados, retomada com automação desligada, pausa e recuperação do snapshot. O frontend testa resultados perdidos/repetidos, reinício do serviço e apresentação da recuperação pausada.

Validação final no Windows:

- `./scripts/test-backend-windows.ps1 processing_`: **57 testes aprovados**.
- Executável de testes com filtro `folder_`: **16 regressões aprovadas**.
- Filtro `feature::duplicates::matcher::tests`: **29 testes aprovados**.
- Vitest: **40 testes frontend aprovados** em seis arquivos, incluindo reconciliação das publicações, controles, statusbar e regressões de thumbnails.
- Typecheck, lint dos arquivos frontend alterados com `--max-warnings 0`, Prettier, build de produção com `--configLoader native`, `git diff --check` e links locais da documentação: sem erros.

Os testes de componentes usam jsdom e IPC simulado. Não houve uso visual da aplicação ou execução no macOS nesta etapa. O documento histórico `05-job-scheduler-workers.md` permanece inalterado.
