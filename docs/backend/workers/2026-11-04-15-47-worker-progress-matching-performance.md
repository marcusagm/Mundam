# Progresso coalescido e custo do matching

> **Estado consolidado:** ciclo encerrado e validado pelo usuário em Windows/macOS. Este documento preserva o relato de sua etapa; referências a próximas entregas e validações pendentes devem ser lidas nesse contexto histórico. Consulte o [encerramento](2026-11-04-22-18-workers-closure.md) e as [evoluções futuras separadas](2026-11-04-22-18-workers-future-evolutions.md).


Entrega de 4 de novembro de 2026, após [limites dinâmicos e orçamento de operações](2026-11-04-14-09-worker-resource-budget.md).

## Estado de validação

O usuário confirmou funcionamento correto e boa performance das entregas anteriores no Windows e no macOS. Essa validação de uso foi registrada no [plano de integração](2026-11-04-00-36-workers-frontend-integration.md). Ela é distinta das medições sintéticas abaixo e não é apresentada como benchmark de bibliotecas grandes nas duas plataformas.

## Progresso manual

O scan manual deixa de criar um canal `mpsc::unbounded_channel` e de publicar `DuplicateScanProgressed` por asset. O frontend já consumia o snapshot global e descartava esses eventos, mas eles ainda atravessavam o EventBus e a ponte Tauri.

[`FingerprintProcessor`](../../../src-tauri/src/feature/duplicates/fingerprint_processor.rs) passa a informar `(processed, total)` por `tokio::sync::watch`: só o valor mais recente fica retido. O consumidor [`scan_progress`](../../../src-tauri/src/feature/duplicates/scan_progress.rs) atualiza o estado do run correspondente. Uma rajada não cria uma fila proporcional ao número de assets. O fechamento do produtor drena a última atualização antes de entrar em matching. Atualizações antigas não alteram um run novo e progresso não sobrescreve `cancel_requested`.

Finder e statusbar continuam usando o mesmo polling global. Não há novo polling, notificação de progresso por asset, invalidação de grupos por progresso ou toast. `DuplicateScanFinished`, `DuplicateScanStopped` e a sequência persistida de publicação conservam suas funções de conclusão e recuperação. O contrato legado `DuplicateScanProgressed` e seu ramo vazio no frontend foram removidos.

## Otimizações do matching

O algoritmo conserva ordem de anchors, exclusão de conteúdo exato, seleção de grupos, scores, razões, limiares e publicação transacional por fase. Duas mudanças reduzem trabalho repetido:

1. As variantes de rotação/espelhamento de cada dHash são preparadas uma vez por snapshot, em vez de serem recriadas a cada par. A preparação testa cancelamento e usa memória proporcional à quantidade de fingerprints, com no máximo oito variantes por candidato.
2. As subgrades de crop só são avaliadas depois de confirmar o sinal de corroboração exigido pela regra atual: similaridade visual **ou** multiscale de pelo menos `0.72`. Antes, as subgrades eram calculadas mesmo quando esse sinal já impedia aceitar o par. A corroboração exclusivamente por multiscale permanece válida.

A API de avaliação de um par continua disponível e delega à mesma avaliação usada pelo caminho com variantes preparadas. A representação dos fingerprints e as regras salvas não mudam; não é necessário rebuild ou migration.

O processamento registra **uma medição agregada por fase concluída** com `phase`, `revision`, quantidade de fingerprints/grupos, `claim_ms`, `compare_ms`, `publish_ms` e `published`. `compare_ms` mede dentro da tarefa de CPU, sem incluir espera na fila de `spawn_blocking`; `claim_ms` inclui a captura do snapshot no repositório. Os registros são locais ao backend, sem caminhos, hashes ou mensagens por asset. Eles permitem identificar se a próxima limitação real está na consulta, comparação ou publicação.

## Benchmark reproduzível

O runner [`bench-duplicate-matching.mjs`](../../../scripts/bench-duplicate-matching.mjs) compila apenas a matemática de produção com `rustc -O`, usando o driver [`Rust`](../../../scripts/bench-duplicate-matching.rs). Não compila Tauri nem acessa banco ou mídia do usuário. Requer Node, Git e Rust stable; no Windows usa o toolchain MSVC.

```sh
node scripts/bench-duplicate-matching.mjs
```

A referência padrão é o commit `2b2bb29`, anterior a esta entrega. Pode-se informar outro commit compatível como argumento. O runner extrai o avaliador de referência para um diretório temporário, executa três amostras por cenário e compara os checksums dos resultados antes de gravar os TSVs e metadados em `docs/backend/workers/benchmarks`. A preparação das variantes entra no tempo medido. Os temporários são removidos sem alterar o checkout. Novos artefatos recebem o prefixo `yyyy-mm-dd-hh-ii` do início da execução no horário local; os metadados registram `startedAt` e `measuredAt` em UTC. O runner recusa sobrescrever uma rodada da mesma plataforma com o mesmo prefixo de minuto. Os nomes datados dos artefatos históricos abaixo usam a primeira criação local de cada arquivo como referência histórica, conforme a escolha registrada no encerramento; seus metadados originais permanecem preservados.

Os cenários usam seed fixa e 128, 512 e 1.024 hashes. Em 1.024 hashes são avaliados **523.776 pares** por amostra. Há cenários sem relação, com crop habilitado/desabilitado, cenários visualmente próximos e crops corroborados por multiscale. Eles exercitam scores e razões de matches visuais e derivados. O digest inclui índices, tipo, score e razões dos resultados.

A rodada automatizada no Windows está registrada em [antes](benchmarks/2026-11-04-15-40-duplicate-matching-windows-before.tsv), [depois](benchmarks/2026-11-04-15-43-duplicate-matching-windows-after.tsv) e [metadados](benchmarks/2026-11-04-15-55-duplicate-matching-windows-metadata.json). Máquina: Intel Core i3-2100, x64, Rust 1.94.0. Para 1.024 hashes:

| Cenário | Antes | Depois | Relação aproximada |
| --- | --- | --- | --- |
| Não relacionados, crop habilitado | 1.265 ms | 187 ms | 6,8× |
| Não relacionados, somente visual | 338 ms | 184 ms | 1,8× |
| Visualmente próximos, crop habilitado | 910 ms | 791 ms | 1,2× |
| Crops corroborados por multiscale | 1.729 ms | 1.577 ms | 1,1× |

Os doze cenários conservaram seus checksums. Divergências interrompem o runner antes de gravar resultados. Os números medem **avaliação de pares**, incluindo preparação e checksum, e excluem extração, SQLite, agrupamento greedy e interface. Não são tempos ponta a ponta nem metas de performance. A máquina e o compiler da rodada automatizada ficam registrados no JSON ao lado dos TSVs; novos resultados devem ser comparados na mesma máquina sem carga concorrente relevante.

A rodada no macOS está registrada em [antes](benchmarks/2026-11-04-21-12-duplicate-matching-macos-before.tsv), [depois](benchmarks/2026-11-04-21-12-duplicate-matching-macos-after.tsv) e [metadados](benchmarks/2026-11-04-21-12-duplicate-matching-macos-metadata.json). Máquina: MacBook Air 2017 (A1466, `MacBookAir7,2`), Intel Core i5-5350U @ 1,80 GHz, x64, macOS 12.7.6, Rust 1.99.1. Para 1.024 hashes:

| Cenário | Antes | Depois | Relação aproximada |
| --- | --- | --- | --- |
| Não relacionados, crop habilitado | 3.256 ms | 367 ms | 8,9× |
| Não relacionados, somente visual | 990 ms | 331 ms | 3,0× |
| Visualmente próximos, crop habilitado | 2.250 ms | 1.827 ms | 1,2× |
| Crops corroborados por multiscale | 4.516 ms | 4.537 ms | 1,0× (sem ganho) |

Os doze cenários conservaram seus checksums. O cenário de crops corroborados por multiscale não mostra ganho, pois o sinal de corroboração é satisfeito e as subgrades precisam ser avaliadas; a diferença de ~0,5% está dentro do ruído de uma máquina sem ventoinha dedicada a carga sustentada (notebook ultrabook sujeito a throttling térmico). Os valores são medianas de três amostras de uma única execução e não são comparáveis em termos absolutos com o Windows (CPU e compiler distintos).

## Matching incremental e escopo restante

Esta entrega **não mantém um índice incremental persistente de relações**. O matching global continua com pior caso quadrático. Precomputar variantes e evitar crops rejeitados reduz seu custo, sem alterar a semântica de agrupamento.

Um índice de pares ou cache incremental exige uma política específica: alterações/remoções de anchors podem reorganizar grupos dependentes, regras invalidam relações anteriores, e grupos revisados/ignorados/resolvidos precisam preservar decisões. A avaliação de crop também pode aceitar um par por multiscale mesmo com dHashes distantes; um índice baseado apenas em distância de dHash eliminaria matches válidos.

No encerramento, o usuário confirmou bom desempenho em bibliotecas representativas nas duas plataformas e dispensou benchmarks ponta a ponta adicionais. Matching incremental persistente foi adiado explicitamente. Os logs e este runner ficam disponíveis para uma eventual avaliação futura; não há medição adicional exigida para concluir o ciclo. O [encerramento](2026-11-04-22-18-workers-closure.md) registra a decisão e as [evoluções futuras](2026-11-04-22-18-workers-future-evolutions.md) explicam o escopo técnico separado.

## Verificação

Os testes novos cobrem rajada de 100.000 atualizações, drenagem do último valor, fencing de run e preservação do cancelamento; integração de scan completo; variantes preparadas com regras combinadas e evidências opcionais; corroboração de crop somente por multiscale; ordem de anchors, exclusão exata e cancelamento do matching. Os testes existentes do matcher e da publicação persistida continuam aplicáveis.

No Windows, **77 testes de processamento** e **29 testes existentes do matcher** passaram, totalizando **106 testes backend**. Também passaram **31 testes frontend** de atividade, controles de duplicados, transporte e reconciliação de publicações.

TypeScript, ESLint do arquivo frontend alterado com `--max-warnings 0`, Prettier, build de produção com `--configLoader native`, `git diff --check`, links locais e hashes das fontes do benchmark foram verificados. O benchmark comparativo executou os 12 cenários com checksums idênticos. Os testes frontend usam jsdom; esta entrega não foi executada nativamente no macOS nesta sessão.

O relatório histórico `05-job-scheduler-workers.md` permanece inalterado.
