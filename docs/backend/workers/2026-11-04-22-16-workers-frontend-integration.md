# Integração dos workers com a interface

> **Estado consolidado:** ciclo encerrado e validado pelo usuário em Windows/macOS. Este documento preserva o relato de sua etapa; referências a próximas entregas e validações pendentes devem ser lidas nesse contexto histórico. Consulte o [encerramento](2026-11-04-22-16-workers-closure.md) e as [evoluções futuras separadas](2026-11-04-22-16-workers-future-evolutions.md).


**Revisão:** 4 de novembro de 2026.

Esta etapa revisa configurações, duplicados e statusbar após a unificação dos fingerprints e a publicação transacional com reservas persistentes. Os contratos IPC dos workers continuam compatíveis; as reservas são um mecanismo interno de execução.

## Caminhos revisados

| Área | Integração e comportamento |
| --- | --- |
| Configurações gerais | Limites compartilhado e de thumbnails aplicam sem reinício; concorrência do indexer permanece separada e exige reinício. O painel informa a capacidade efetiva. O cache de transcodificação conserva seu fluxo próprio. |
| Configurações de duplicados | Automação persistida é diferente de pausa da sessão. A preferência descreve análise em segundo plano pelas regras ativas, sem prometer agrupamento instantâneo. |
| Scan e manutenção | Scan, cancelamento e rebuild usam o mesmo controlador compartilhado da tela de duplicados. Rebuild exige confirmação sobre fingerprints, grupos e histórico de resoluções; preserva arquivos, thumbnails e cores. |
| Duplicate Finder | Estado e fases vêm do snapshot compartilhado. Progresso de hashing não permanece em 100% enquanto compara arquivos; cancelamento permanece como parada solicitada até a saída. |
| Modal de regras | Salvar e analisar usa a mesma admissão. A ação fica bloqueada durante pausa ou outra execução; falha ao salvar mantém o modal aberto com feedback. |
| Statusbar | Apresenta fases manuais e contagens reais, explica falhas/ausência de extratores e desabilita controles de executores indisponíveis. Botões de ícones têm nomes acessíveis. O atalho de configurações acompanha a plataforma. |
| Viewport | Os contratos de resultado de thumbnails permanecem os mesmos; os testes existentes de loading, fallback e recuperação foram reexecutados. |

## Fonte de estado e ações

[`useDuplicateScan`](../../../src/components/features/duplicates/hooks/useDuplicateScan.ts) consome o mesmo `ProcessingClient` utilizado pela statusbar. Configurações, regras e finder não criam novos loops de polling. A admissão local é compartilhada entre componentes; o backend conserva a validação definitiva contra concorrência e pausa.

Exemplo de consumo:

```tsx
const scan = useDuplicateScan();
<Button disabled={!scan.canStart()} onClick={scan.startScan}>
    Scan Now
</Button>;
```

O scan manual continua disponível quando a automação está desligada. Snapshot desconhecido, erro de transporte, executor indisponível, pausa ou execução em andamento bloqueiam uma nova solicitação. Nas configurações é possível retomar duplicados sem fechar o modal.

As fases `preparing`, `hashing`, `exact_matching`, `visual_matching` e `cancel_requested` têm rótulos próprios. Somente hashing com total positivo recebe percentual; as demais fases são indeterminadas. Não são apresentados contadores de grupos criados sem suporte no snapshot.

O progresso manual vem somente do snapshot global, alimentado por um canal de último valor; os eventos legados de progresso por asset foram retirados. Eventos de resultado continuam invalidando os grupos. A sequência persistida de publicação permite recuperar resultados mesmo quando a notificação de término é perdida. O polling de progresso não recarrega listas, árvores ou badges a cada amostragem.

Uma confirmação de cancelamento não equivale à saída do executor. A interface mantém “Stopping” até o término e não informa rebuild concluído para uma solicitação cancelada. Falhas de ações explícitas recebem feedback consolidado; falhas de polling permanecem no painel.

## Estado final da integração

A integração com configurações, Duplicate Finder, viewport e statusbar está concluída. O usuário confirmou a validação funcional da última entrega no Windows e no macOS, além de bom desempenho em bibliotecas representativas. Não foram feitos benchmarks ponta a ponta e o usuário dispensou sua realização para encerrar o ciclo.

| Área | Situação |
| --- | --- |
| Comparações persistentes e recuperação | Entregues. |
| Retry por etapa e diagnóstico agrupado | Entregues. |
| Limites dinâmicos e orçamento compartilhado | Entregues. |
| Progresso coalescido e otimização medida | Entregues, com benchmarks sintéticos Windows/macOS preservados. |
| Suspensão por ferramenta e recheck na statusbar | Entregues e validados. |
| Matching incremental persistente | Adiado por decisão do usuário; comparações permanecem globais. |

O [encerramento](2026-11-04-22-16-workers-closure.md) consolida o aceite e os limites. As [evoluções técnicas](2026-11-04-22-16-workers-future-evolutions.md) ficam em documento próprio e não constituem uma próxima entrega obrigatória.

## Validação desta revisão

Vitest aprovou **34 testes** nos arquivos `duplicateProcessing.spec.tsx`, `ProcessingActivityPanel.spec.tsx`, `processingClient.spec.ts`, `thumbnailProcessing.spec.ts` e `Thumbnail.spec.tsx`. Eles cobrem admissão compartilhada, retomada, automação desligada, fases, cancelamento local/remoto, recuperação por snapshot, confirmação do rebuild, acessibilidade, foco durante polling e regressões de thumbnails.

`npm run typecheck`, ESLint dos arquivos alterados com `--max-warnings 0`, Prettier e `git diff --check` passaram. O build de produção também passou com `npm run build -- --configLoader native`. O carregador padrão do esbuild não conseguiu ler diretórios ancestrais no ambiente restrito; o carregador nativo utilizou a mesma configuração sem alterá-la.

Os testes de componentes desta revisão original usaram jsdom e IPC simulado. Posteriormente, o usuário confirmou funcionamento correto e boa performance das entregas no Windows e no macOS, em 4 de novembro de 2026. A validação de carga controlada permanece separada, conforme a tabela atualizada. O relatório histórico `05-job-scheduler-workers.md` permanece inalterado.
