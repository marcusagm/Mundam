# Integração dos workers com a interface

**Revisão:** 4 de novembro de 2026.

Esta etapa revisa configurações, duplicados e statusbar após a unificação dos fingerprints e a publicação transacional com reservas persistentes. Os contratos IPC dos workers continuam compatíveis; as reservas são um mecanismo interno de execução.

## Caminhos revisados

| Área | Integração e comportamento |
| --- | --- |
| Configurações gerais | Concorrência de thumbnails continua exigindo reinício. O texto explica esse limite e indica os controles de pausa na atividade do sistema. O cache de transcodificação conserva seu fluxo próprio. |
| Configurações de duplicados | Automação persistida é diferente de pausa da sessão. A preferência descreve análise em segundo plano pelas regras ativas, sem prometer agrupamento instantâneo. |
| Scan e manutenção | Scan, cancelamento e rebuild usam o mesmo controlador compartilhado da tela de duplicados. Rebuild exige confirmação sobre fingerprints, grupos e histórico de resoluções; preserva arquivos, thumbnails e cores. |
| Duplicate Finder | Estado e fases vêm do snapshot compartilhado. Progresso de hashing não permanece em 100% enquanto compara arquivos; cancelamento permanece como parada solicitada até a saída. |
| Modal de regras | Salvar e analisar usa a mesma admissão. A ação fica bloqueada durante pausa ou outra execução; falha ao salvar mantém o modal aberto com feedback. |
| Statusbar | Apresenta fases manuais e contagens reais, explica falhas/ausência de extratores e desabilita controles de executores indisponíveis. Botões de ícones têm nomes acessíveis. O atalho de configurações acompanha a plataforma. |
| Viewport | Os contratos de resultado de thumbnails permanecem os mesmos; os testes existentes de loading, fallback e recuperação foram reexecutados. |

## Fonte de estado e ações

[`useDuplicateScan`](../../src/components/features/duplicates/hooks/useDuplicateScan.ts) consome o mesmo `ProcessingClient` utilizado pela statusbar. Configurações, regras e finder não criam novos loops de polling. A admissão local é compartilhada entre componentes; o backend conserva a validação definitiva contra concorrência e pausa.

Exemplo de consumo:

```tsx
const scan = useDuplicateScan();
<Button disabled={!scan.canStart()} onClick={scan.startScan}>
    Scan Now
</Button>;
```

O scan manual continua disponível quando a automação está desligada. Snapshot desconhecido, erro de transporte, executor indisponível, pausa ou execução em andamento bloqueiam uma nova solicitação. Nas configurações é possível retomar duplicados sem fechar o modal.

As fases `preparing`, `hashing`, `exact_matching`, `visual_matching` e `cancel_requested` têm rótulos próprios. Somente hashing com total positivo recebe percentual; as demais fases são indeterminadas. Não são apresentados contadores de grupos criados sem suporte no snapshot.

Eventos de progresso antigos não alteram mais o estado de execução do finder. Eventos de resultado continuam invalidando os grupos. Se a notificação de término for perdida, a transição de execução para ociosidade no snapshot recupera a consulta dos resultados. O polling de progresso não recarrega listas, árvores ou badges a cada amostragem.

Uma confirmação de cancelamento não equivale à saída do executor. A interface mantém “Stopping” até o término e não informa rebuild concluído para uma solicitação cancelada. Falhas de ações explícitas recebem feedback consolidado; falhas de polling permanecem no painel.

## Pendências do plano

As seções iniciais de [Workers: diagnóstico e proposta](workers-processing-control.md) preservam o planejamento original. As entregas posteriores não encerram todos os itens:

| Prioridade sugerida | Próxima entrega | Limite atual |
| --- | --- | --- |
| 1 | Jobs persistidos de comparação de duplicados, com política de regras e recuperação após crash | Fingerprints têm reserva persistente; matching e sua intenção agendada continuam com coordenação local e estado em memória. |
| 2 | Repetir falhas por estágio e diagnóstico agrupado de capacidades/extratores | Há resultados e limite de tentativas, mas não comando global de retry por worker ou diagnóstico detalhado no painel. Rebuild repete fingerprints; regeneração/reextração de assets conservam suas ações existentes. |
| 3 | Concorrência efetiva em tempo de execução e orçamento comum de CPU/I/O | Os limites configurados ainda exigem reinício; workers mantêm concorrências próprias. |
| 4 | Coalescência de progresso manual e matching incremental, orientados por benchmarks | Snapshot global é compartilhado, mas o backend ainda conserva o canal legado de progresso manual e comparações globais. |
| Validação | Uso visual e testes de carga nas duas plataformas | Testes automatizados desta etapa não substituem uso real de modais, foco, statusbar e carga de biblioteca em Windows/macOS. |

Persistência da pausa, separação entre pausa de fundo e prioridade do viewport, identidade por execução do indexer e snapshot físico atômico da origem também permanecem evoluções separadas. A interface atual explica a política suportada: pausa inclui thumbnails visíveis, drena operações admitidas e reinicia com a aplicação.

## Validação desta revisão

Vitest aprovou **34 testes** nos arquivos `duplicateProcessing.spec.tsx`, `ProcessingActivityPanel.spec.tsx`, `processingClient.spec.ts`, `thumbnailProcessing.spec.ts` e `Thumbnail.spec.tsx`. Eles cobrem admissão compartilhada, retomada, automação desligada, fases, cancelamento local/remoto, recuperação por snapshot, confirmação do rebuild, acessibilidade, foco durante polling e regressões de thumbnails.

`npm run typecheck`, ESLint dos arquivos alterados com `--max-warnings 0`, Prettier e `git diff --check` passaram. O build de produção também passou com `npm run build -- --configLoader native`. O carregador padrão do esbuild não conseguiu ler diretórios ancestrais no ambiente restrito; o carregador nativo utilizou a mesma configuração sem alterá-la.

Os testes de componentes usam jsdom e IPC simulado; não houve execução visual da aplicação ou validação no macOS nesta etapa. O relatório histórico `05-job-scheduler-workers.md` permanece inalterado.
