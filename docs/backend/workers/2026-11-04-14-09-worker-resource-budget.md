# Orçamento compartilhado dos workers

> **Estado consolidado:** ciclo encerrado e validado pelo usuário em Windows/macOS. Este documento preserva o relato de sua etapa; referências a próximas entregas e validações pendentes devem ser lidas nesse contexto histórico. Consulte o [encerramento](2026-11-04-22-18-workers-closure.md) e as [evoluções futuras separadas](2026-11-04-22-18-workers-future-evolutions.md).


Entrega de 4 de novembro de 2026, após [retries e diagnósticos](2026-11-04-05-51-worker-retries-diagnostics.md).

## Comportamento

As configurações de Performance passam a oferecer dois limites que valem sem reiniciar:

- **Shared processing limit**: quantidade máxima de operações de mídia admitidas ao mesmo tempo, compartilhada entre thumbnails, cores, fingerprints e comparações exatas/visuais. Aceita `0` (automático) e `1..8`.
- **Thumbnail limit**: quantidade máxima de operações de thumbnail, sempre sujeita à capacidade compartilhada. A chave persistida continua sendo `thumbnail_threads`; `0` permite usar o limite compartilhado. Configurações existentes são preservadas, com admissão limitada de forma segura.

O modo automático usa `available_parallelism - 1`, limitado a `1..8`. A capacidade reservada ao sistema é uma escolha conservadora de admissão, não uma medição de CPU livre. Cores e duplicados continuam com execução serial por grupo. Uma comparação manual e o processamento automático passam pelo mesmo orçamento.

Reduzir o limite **não cancela** operações já admitidas. Por exemplo, reduzir de quatro para um com três operações em andamento pode mostrar `3 / 1` no painel até as operações terminarem. Enquanto a ocupação não ficar abaixo do novo limite, nenhuma operação adicional entra. Aumentar o limite acorda os solicitantes em espera. Pausas e a política de duplicados automáticos continuam independentes desses limites.

A concorrência do **indexer** permanece separada e ainda exige reinício. Reprodução/transcodificação sob demanda e limpeza de lixeira/cache não entram neste orçamento.

## Admissão e recuperação

[`ProcessingBudget`](../../../src-tauri/src/core/processing/budget.rs) mantém uma fila em memória e devolve permissões RAII. A fila é FIFO entre solicitações elegíveis: um worker que já atingiu seu limite individual não bloqueia a capacidade livre dos demais. O executor de thumbnails continua selecionando primeiro a janela mais recente do viewport quando recebe capacidade; pedidos de outros workers não ficam atrás de uma fila ilimitada de thumbnails.

As permissões são obtidas **antes** de `ProcessingRepository.begin` e de `claim_matching`. Esperar capacidade não consome tentativas nem mantém reservas de banco. As reservas e heartbeats existentes só começam depois da admissão. Cada etapa do scan manual libera sua capacidade antes da próxima operação, sem reservar o orçamento por todo o scan. O coordenador de duplicados permanece responsável por impedir scans concorrentes.

Cancelar ou descartar uma espera remove sua solicitação da fila. Uma permissão em andamento é liberada ao finalizar ou desenrolar a execução. O encerramento do worker de thumbnails continua drenando suas tarefas admitidas. Os limites não são persistidos como tarefas e não alteram generations, resultados publicados ou arquivos originais.

## Integração

- [`ProcessingControl`](../../../src-tauri/src/core/processing/control.rs) possui a instância compartilhada do orçamento.
- O bootstrap cria o controle junto das configurações, carrega os limites persistidos antes de iniciar os workers e injeta a mesma instância no coordenador de duplicados.
- [`SettingsService`](../../../src-tauri/src/feature/settings/service.rs) serializa mutações entre seus clones. Uma alteração só configura o orçamento depois de o repositório confirmar a gravação. Não é criado um segundo polling de configurações.
- O snapshot global inclui `capacity: { limit, active, waiting, thumbnail_limit }`. `active` mede permissões de recursos e é independente de atividades longas de scan manual. `waiting` mede solicitações de admissão dos executores, não o tamanho das filas persistidas de assets.
- [`PerformanceSettings`](../../../src/components/features/settings/PerformanceSettings.tsx) distingue limites aplicados imediatamente da configuração do indexer. Os valores de mídia no store só mudam após confirmação do backend; falhas aparecem inline. Seletores têm nomes acessíveis no botão interativo.
- A statusbar apresenta capacidade efetiva e espera usando o snapshot compartilhado já existente. Esse campo é opcional no frontend para compatibilidade com um backend anterior ainda em execução.

Não há migration de banco. Arquivos antigos de configurações recebem `processing_concurrency = 0` pela desserialização. O controle de pausa continua sendo apenas da sessão.

## Limites desta entrega

O orçamento limita **operações**, não threads nativas, memória ou percentuais de CPU/I/O. FFmpeg, codecs e bibliotecas de imagem podem criar várias threads dentro de uma operação. Todas as operações têm peso unitário; não há ajuste adaptativo por telemetria, classes por custo ou quotas distintas para disco e CPU. O matching global conserva seu algoritmo atual, inclusive o custo quadrático da comparação visual em grandes bibliotecas.

A etapa posterior [Progresso e custo do matching](2026-11-04-15-47-worker-progress-matching-performance.md) remove o canal ilimitado de progresso e registra uma primeira otimização com benchmark sintético. Medições ponta a ponta de cargas representativas continuam necessárias antes de introduzir pesos, índices persistentes ou políticas específicas por tipo de operação.

## Validação

Os testes cobrem crescimento/redução em runtime, drenagem, FIFO entre solicitantes elegíveis, limites individuais, cancelamento/descarte de espera e uma carga mista de 120 operações. A integração de configurações cobre compatibilidade com JSON anterior, gravações concorrentes entre clones e falha ao persistir sem mudar o orçamento. No frontend, os testes cobrem confirmação de gravação, falha sem alteração otimista, validação de limites, seletores acessíveis e apresentação reativa de capacidade.

No Windows, **71 testes de processamento** passaram, incluindo os nove testes novos do orçamento e da configuração. **52 testes frontend** passaram em oito arquivos, cobrindo o novo painel, store de configurações, statusbar, controles de duplicados e regressões de thumbnails.

`npm run typecheck`, ESLint dos arquivos alterados com `--max-warnings 0`, Prettier, `npm run build -- --configLoader native`, `git diff --check` e verificação dos links locais passaram. Os testes de componentes usam jsdom e IPC simulado.

Posteriormente, o usuário confirmou funcionamento correto e boa performance desta entrega no Windows e no macOS (4 de novembro de 2026). Isso registra validação em uso; benchmarks controlados de carga nas duas plataformas permanecem separados.
