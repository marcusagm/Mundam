# Disponibilidade de ferramentas dos workers

> **Estado consolidado:** ciclo encerrado e validado pelo usuário em Windows/macOS. Este documento preserva o relato de sua etapa; referências a próximas entregas e validações pendentes devem ser lidas nesse contexto histórico. Consulte o [encerramento](2026-11-04-22-18-workers-closure.md) e as [evoluções futuras separadas](2026-11-04-22-18-workers-future-evolutions.md).


Esta entrega distingue falha do arquivo de indisponibilidade temporária de **FFmpeg, FFprobe e Assimp**. O [benchmark de matching](2026-11-04-15-47-worker-progress-matching-performance.md), agora com resultados Windows e macOS fornecidos pelo usuário, preserva as medições históricas; matching incremental persistente foi adiado pelo usuário no encerramento. A matemática e os critérios de agrupamento não mudam nesta entrega.

## Comportamento

Quando uma etapa retorna uma causa tipada de ferramenta indisponível, sua tarefa permanece em `retry_wait`, com diagnóstico específico. A leitura apresenta `blocked` enquanto a ferramenta não está disponível. Esse resultado **não consome tentativas de extração**. Arquivos atendidos por extratores nativos e outras etapas continuam elegíveis; o worker inteiro não é pausado.

Exemplo: uma thumbnail HEIC que requer FFmpeg pode aguardar a ferramenta enquanto thumbnails PNG nativas continuam sendo geradas. Um arquivo corrompido que falha com a ferramenta disponível conserva a política de até três tentativas e termina como falha. Não se deduz indisponibilidade a partir de mensagens de erro livres.

Uma verificação executa somente o comando de versão de cada ferramenta, sem receber arquivos da biblioteca: `ffmpeg -version`, `ffprobe -version` e `assimp version`. Cada subprocesso tem timeout de dois segundos, entrada e saídas descartadas e janela oculta no Windows. Processos que excedem o prazo são encerrados e coletados. O lote sequencial é executado fora do executor assíncrono e não reserva capacidade de mídia.

O loop de manutenção verifica no início e a cada 60 segundos. A statusbar permite antecipar a verificação em **System Activity → Check extraction tools**. Solicitações concorrentes compartilham o lote; um intervalo mínimo de um segundo evita repetições imediatas. Uma ferramenta recuperada libera as tarefas dependentes sem esperar o backoff de extração. Pausas, automação de duplicados, gerações e resultados válidos permanecem preservados.

## Contratos e organização

- [`core/processing/tools.rs`](../../../src-tauri/src/core/processing/tools.rs): inventário finito, fatos seguros e port do probe.
- [`feature/processing/tools.rs`](../../../src-tauri/src/feature/processing/tools.rs): coordenação e coalescência dos lotes.
- [`processing/transcoding/binaries.rs`](../../../src-tauri/src/processing/transcoding/binaries.rs): descoberta e probe técnico. A descoberta consulta recursos/PATH sem subprocessos, com cache positivo e negativo de 60 segundos, limitado a 32 contextos. O probe invalida a descoberta anterior. Falhas tipadas de lançamento invalidam o cache.
- [`processing/workers/tools/mod.rs`](../../../src-tauri/src/processing/workers/tools/mod.rs): manutenção registrada no lifecycle; não é uma quarta linha de worker controlável.
- [`processing_tool_health`](../../../src-tauri/migrations/20261104030000_processing_tool_health.sql): três fatos persistidos, com horário e revisão. Leituras ficam em `query_handlers/processing/tools.rs` e mutações em `handlers/processing/tools.rs`.
- [`check_processing_tools`](../../../src-tauri/src/delivery/tauri/processing.rs): comando sem argumentos que retorna o snapshot global ordenado. Permitido às janelas da biblioteca e negado ao splashscreen.

Uma falha de extração só altera a disponibilidade dentro da transação que aceita seu ticket vigente. Probes capturam revisões antes de iniciar e publicam por comparação de revisão: uma resposta antiga não sobrescreve uma falha mais recente. Tickets expirados ou substituídos não alteram a saúde da ferramenta.

O snapshot global acrescenta `tools` e a contagem `blocked` por worker. Diagnósticos agrupados também distinguem `blocked`. A interface mostra ferramentas opcionais indisponíveis sem declarar todo o processamento problemático; o indicador recebe atenção quando existe trabalho bloqueado. Falhas de comunicação aparecem inline, sem toast ou invalidação de listas, árvores e badges.

O viewport recebe `blocked` no read model compartilhado de thumbnails e mostra um ícone explicado, mantendo o polling em lote. O ícone fica estável enquanto aguarda; após recuperação, o estado volta a `pending` e depois ao resultado publicado. Thumbnails legitimamente pendentes conservam seu loader. A [implementação do hook](../../../src/components/features/viewport/assets/useThumbnail.ts) contém a apresentação, mantendo `Thumbnail.tsx` como componente visual.

## Limites

O inventário verifica execução de versão, não suporte a cada codec ou integridade de arquivos. Provedores desconhecidos continuam classificados como sem suporte, com recheck explícito. Outros executáveis não entram automaticamente nessa política. Um asset ainda pode fazer sua primeira tentativa para identificar a dependência; não há bloqueio antecipado por extensão.

Fallbacks nativos continuam sob responsabilidade dos providers. A política existente de fingerprints que aceita hash exato quando não é possível calcular sinais visuais permanece preservada. Falhas opcionais de preview que o provider absorve não se tornam bloqueios de toda a etapa. A conversão Assimp resolve sua própria dependência, sem exigir FFmpeg e FFprobe.

A disponibilidade observada pode mudar entre probe e execução. Ela é um fato recuperável, não uma garantia de sucesso. Probes não reiniciam falhas terminais antigas nem trabalhos sem suporte: essas situações conservam as ações de retry/recheck já existentes.

## Validação

Testes com SQLite real cobrem suspensão seletiva, tentativas preservadas, recuperação antes do backoff, retenção de resultados, diagnósticos, revisão de probes e rejeição de tickets antigos/expirados. Probes injetáveis verificam coalescência sem depender das ferramentas instaladas. Testes IPC exercitam as capacidades reais das janelas.

No frontend, os testes cobrem o ícone bloqueado estável, recuperação por polling, loader pendente, controles, atualização de inventário e erros inline. No Windows passaram **83 testes backend** (81 de processamento e dois de transcoding) e **121 testes frontend em 23 arquivos**, além de TypeScript, ESLint com zero warnings nos arquivos alterados, Prettier, build de produção e auditoria estrita de arquivos. A auditoria cobre 638 arquivos frontend e 355 Rust, sem candidatos órfãos ou referências não resolvidas. Links locais e `git diff --check` também foram verificados.

Comandos usados:

```powershell
./scripts/test-backend-windows.ps1 processing_
./src-tauri/target/debug/deps/mundam_lib-af984ef4adbe7005.exe processing::transcoding::tests --test-threads=2
npx vitest run --configLoader native --pool threads --maxWorkers 1
npm run typecheck
npm run build -- --configLoader native
node scripts/audit-source-files.mjs --strict
```

O nome do executável de testes depende do build; o script Windows localiza o binário e incorpora o manifesto necessário antes de executá-lo. Posteriormente, o usuário confirmou a validação funcional desta entrega no Windows/macOS. Também confirmou bom desempenho em bibliotecas representativas, sem executar benchmarks ponta a ponta e dispensando-os para o aceite. O documento histórico `05-job-scheduler-workers.md` permanece inalterado.
