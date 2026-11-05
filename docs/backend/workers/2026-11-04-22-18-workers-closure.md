# Encerramento da padronização dos workers

**Estado:** escopo atual concluído e aceito pelo usuário.  
**Data de consolidação:** 4 de novembro de 2026, no fuso America/Sao_Paulo.  
**Escopo:** thumbnails e metadados, cores, fingerprints, comparações de duplicados, controles e integração com a interface.

## Aceite e decisão de encerramento

O usuário confirmou o funcionamento da última entrega no Windows e no macOS, incluindo suspensão por ferramenta indisponível, recuperação, controles e apresentação na interface. Também informou bom desempenho do processamento completo em bibliotecas representativas. Essa avaliação é de uso real: **não foram executados benchmarks ponta a ponta** e não há números de CPU, memória, I/O ou latência a atribuir a essa confirmação.

Os benchmarks sintéticos de avaliação de pares foram executados anteriormente nas duas plataformas e permanecem arquivados com seus metadados. Eles não medem todo o pipeline. O usuário dispensou benchmarks adicionais como condição para concluir este ciclo.

Matching incremental persistente foi explicitamente adiado. As comparações continuam globais, com as otimizações e as proteções de publicação entregues. Não há implementação incremental anunciada nem pendência desse recurso para este encerramento.

As evoluções técnicas foram separadas em [Evoluções futuras](2026-11-04-22-18-workers-future-evolutions.md), com explicação do problema, motivo, benefício e critérios para uma eventual implementação. Não são compromissos de uma próxima entrega.

## Resultado entregue

| Área | Comportamento consolidado |
| --- | --- |
| Elegibilidade e recuperação | Trabalho persistido por etapa, exclusão de assets inativos e recuperação sem depender de eventos ou do viewport. |
| Concorrência e publicação | Reservas com expiração e heartbeat; tentativas antigas não publicam sobre gerações ou proprietários novos. Resultados e conclusão são aceitos transacionalmente. |
| Thumbnails e previews | Artefatos por tentativa, publicação por ponteiros e reconciliação do cache, preservando arquivos originais. Loader e fallback refletem o resultado real da etapa. |
| Cores e fingerprints | Execução especializada com política comum de tentativas. Scan manual e automático compartilham o cálculo de fingerprints e preservam o resultado exato quando não há sinais visuais. |
| Comparações | Jobs persistidos, regras por revisão, recuperação após crash e sequência de publicação. Grupos e decisões protegidas permanecem preservados. |
| Controle do usuário | Snapshot global, pausa/retomada, retries e diagnóstico agrupado na statusbar; integração com configurações e Duplicate Finder. |
| Recursos | Capacidade compartilhada e limites de operações alteráveis em execução, com drenagem das operações já admitidas. |
| Ferramentas | Suspensão seletiva por FFmpeg, FFprobe ou Assimp indisponível, sem consumir tentativas; probes limitados e recheck explícito. |
| Atualizações da interface | Progresso coalescido e polling compartilhado; telemetria não invalida listas, árvores e badges por asset nem gera rajadas de toasts. |
| Organização | Domínios agrupados no backend e frontend, documentação vinculada e auditoria conservadora de arquivos sem consumidores. |

Os limites continuam explícitos: pausas são da sessão e incluem prioridade do viewport; operações nativas admitidas drenam; a identificação de origem usa o catálogo; o versionamento uniforme por algoritmo e o matching incremental não foram implementados.

## Evidências e limites de validação

- A última entrega registrou **83 testes backend** no Windows: 81 de processamento e dois de transcoding; **121 testes frontend em 23 arquivos**; TypeScript, lint, formatação e build aprovados.
- A auditoria de arquivos registrou **638 arquivos frontend e 355 Rust**, sem candidatos órfãos ou referências não resolvidas no escopo do analisador. Isso não prova uso em runtime de todos os exports.
- A validação funcional e o bom desempenho em Windows/macOS foram confirmados pelo usuário. Essa confirmação complementa os testes; não equivale a executar a suíte automatizada nativamente no macOS nesta sessão.
- As contagens de testes dos documentos de cada etapa são fotografias daquela entrega. Não devem ser somadas nem tratadas como pendências atuais.
- Nenhum benchmark novo foi executado para esta consolidação documental. Os artefatos existentes permanecem evidências históricas.

## Organização e nomes dos arquivos

Os documentos e relatórios deste ciclo estão agrupados em `docs/backend/workers/`, com as medições na subpasta `benchmarks/`. Cada arquivo recebe como prefixo a data e hora da sua primeira criação local, no fuso America/Sao_Paulo, usada por escolha do usuário como referência da etapa histórica. Esse dado foi capturado antes das renomeações e não representa um horário de início de implementação comprovado pelo Git. As datas das entregas e os campos `measuredAt` dos benchmarks permanecem preservados. Os documentos novos de encerramento e evoluções futuras usam suas próprias datas de criação.

A convenção é `yyyy-mm-dd-hh-ii-nome-do-arquivo.md` para documentos. Relatórios estruturados mantêm `.json` e tabelas mantêm `.tsv`, também com o prefixo de sua primeira criação local, preservando seus formatos e conteúdos. O runner de benchmarks passa a gerar arquivos datados na pasta agrupada, usando o início de cada nova execução e o horário local do processo; `startedAt` e `measuredAt` nos novos metadados usam ISO UTC.

## Índice do ciclo

Os documentos detalhados preservam o histórico e os limites de cada entrega. Declarações como “próxima etapa” nesses relatos se referem ao momento da implementação; o estado final é o deste encerramento.

| Documento | Conteúdo |
| --- | --- |
| [Diagnóstico e proposta](2026-10-30-05-17-workers-processing-control.md) | Problemas iniciais, plano e acompanhamento das entregas. |
| [Controles na statusbar](2026-11-03-09-01-worker-statusbar-controls.md) | Snapshot, atividade e política de pausa/retomada. |
| [Fingerprints compartilhados](2026-11-03-20-35-fingerprint-processing.md) | Pipeline comum ao scan manual e automático. |
| [Reservas e publicação](2026-11-03-23-32-processing-leases.md) | Propriedade, expiração, heartbeat, artefatos e commits. |
| [Integração com o frontend](2026-11-04-00-36-workers-frontend-integration.md) | Configurações, finder, statusbar e aceite final. |
| [Comparações persistentes](2026-11-04-03-46-duplicate-matching-jobs.md) | Jobs, revisão de política e recuperação das comparações. |
| [Retries e diagnósticos](2026-11-04-05-51-worker-retries-diagnostics.md) | Ações por etapa e causas seguras. |
| [Orçamento de recursos](2026-11-04-14-09-worker-resource-budget.md) | Limites dinâmicos e capacidade compartilhada. |
| [Progresso e performance](2026-11-04-15-47-worker-progress-matching-performance.md) | Coalescência, otimizações e benchmarks sintéticos. |
| [Organização e auditoria](2026-11-04-19-02-worker-module-organization.md) | Divisão dos módulos e revisão do plano. |
| [Disponibilidade de ferramentas](2026-11-04-21-32-worker-tool-availability.md) | Suspensão, probes, recuperação e interface. |
| [Decodificador GPR](2026-10-29-00-57-gpr-native-decoder.md) | Registro técnico relacionado de extração nativa. |
| [Auditoria de arquivos](2026-11-04-19-13-source-file-audit.json) | Relatório estruturado de alcançabilidade de arquivos. |
| [Benchmarks](benchmarks/) | TSVs e metadados das medições históricas. |
| [Evoluções futuras](2026-11-04-22-18-workers-future-evolutions.md) | Possibilidades separadas do escopo concluído. |

O documento histórico `docs/report/backend-architeture/definition/modules/05-job-scheduler-workers.md` permanece inalterado. Esta consolidação reorganiza documentação e o destino de relatórios; não altera o comportamento da aplicação.
