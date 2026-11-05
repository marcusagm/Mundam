# Evoluções técnicas futuras do processamento

**Estado:** propostas fora do escopo encerrado; sem implementação neste ciclo.  
**Contexto:** o usuário confirmou funcionamento e bom desempenho em Windows/macOS, dispensou benchmarks ponta a ponta para o encerramento e adiou matching incremental. O [encerramento](2026-11-04-22-16-workers-closure.md) registra o aceite.

## 1. Identidade por execução do indexer

**Do que se trata:** atribuir uma identidade a cada execução de indexação e associá-la ao progresso, término, erro e cancelamento. O frontend aceitaria atualizações somente da execução vigente, como já ocorre com o scan manual de duplicados. Uma identidade da instância também pode distinguir reinícios do processo.

**Motivo:** uma execução pode terminar ou entregar progresso depois de outra começar. Sem identificação, uma mensagem atrasada pode alterar o progresso exibido ou encerrar visualmente a execução seguinte. Isso é uma proteção de concorrência; não significa que esse defeito tenha sido observado na validação atual.

**Benefício:** progresso coerente ao iniciar scans sucessivos, cancelar ou reiniciar, sem confundir atividade antiga com trabalho atual.

**Possível implementação:** definir os contratos no domínio, instalar a identidade na admissão da execução, propagar por todos os resultados e aplicar a mesma verificação no estado compartilhado do frontend. O snapshot deve recuperar a execução vigente quando uma notificação for perdida. A identidade de execução não substitui a geração ou o lease de um asset.

**Critério de aceite futuro:** progresso e término da execução A não alteram a execução B; cancelamento de A não cancela B; reinício da aplicação não aceita mensagens da instância anterior. Comparar formatos de eventos legados e adaptar todos os consumidores antes da troca.

## 2. Verificação da origem física durante o processamento

**Do que se trata:** fortalecer a identificação do arquivo que realmente foi lido. Atualmente, a validação usa caminho, tamanho e data de modificação do catálogo, além de geração e lease. Ela protege mudanças já conhecidas pelo catálogo, mas não representa um snapshot físico atômico durante a extração.

**Motivo:** um programa externo pode substituir ou modificar o arquivo enquanto o provider o lê, antes de o watcher atualizar o catálogo. Uma validação apenas do catálogo pode não reconhecer essa alteração naquele instante.

**Benefício:** reduzir a possibilidade de publicar thumbnail, cores, metadados ou fingerprint de uma versão física que já foi substituída.

**Possível implementação:** capturar a identidade e os metadados físicos antes e depois da leitura, usando informações compatíveis com cada plataforma. Sempre que possível, ler e inspecionar o mesmo handle. Subprocessos que reabrem um caminho precisam de uma estratégia própria. Comparar as observações físicas junto da validade do ticket antes de publicar.

Validações antes/depois detectam classes de mudanças, mas não garantem um snapshot atômico. Uma cópia imutável, snapshot de filesystem ou mecanismo de bloqueio pode dar garantias mais fortes, com custos e limitações distintos. A escolha exige avaliar disco, I/O, arquivos grandes, unidades de rede e compatibilidade dos providers; não se deve anunciar atomicidade apenas por repetir `stat`.

**Critério de aceite futuro:** testar alteração, substituição e renomeação durante a leitura, inclusive sem atualização imediata do catálogo. Resultados superados são rejeitados e o trabalho permanece recuperável. Originais não são modificados; artefatos rejeitados são reconciliados. Documentar as garantias efetivas de cada estratégia nas duas plataformas.

## 3. Versão por algoritmo e etapa

**Do que se trata:** registrar a versão do algoritmo/configuração relevante ao resultado de thumbnails, cores e fingerprints, com uma política uniforme de elegibilidade. Fingerprints já têm sua versão própria; falta uniformizar esse mecanismo nas demais etapas.

**Motivo:** mudar um algoritmo ou corrigir um provider pode deixar resultados antigos aparentemente válidos. Invalidar toda a biblioteca para qualquer alteração desperdiçaria trabalho e poderia apagar resultados ainda úteis.

**Benefício:** reprocessamento seletivo, migrações previsíveis e identificação clara de quais resultados foram gerados com uma implementação anterior.

**Possível implementação:** definir versões de saída por etapa e, quando necessário, por provider; persistir a versão publicada e a esperada pela tentativa. Uma mudança relevante torna somente os resultados afetados elegíveis. A publicação verifica versão, origem, geração e propriedade na mesma transação. O matching deve respeitar a compatibilidade dos fingerprints e a revisão de regras, sem combinar formatos incompatíveis silenciosamente.

**Critério de aceite futuro:** atualizar cores não regenera thumbnails nem hashes exatos sem necessidade; saídas válidas podem permanecer visíveis enquanto uma substituição é preparada; uma tentativa de versão antiga não sobrescreve a nova; migrations e recuperação após crash preservam a intenção de reprocessar.

## 4. Matching incremental persistente

**Decisão atual:** adiado explicitamente pelo usuário. O desempenho observado é adequado e não há implementação prevista neste ciclo.

**Do que se trata:** reaproveitar comparações anteriores e recalcular apenas a parte afetada por criação, alteração ou remoção de assets, mudança de fingerprints ou revisão das regras. O matching atual continua global, com pior caso quadrático.

**Motivo para reconsiderar:** crescimento de bibliotecas ou evidências de que o tempo das comparações globais passou a prejudicar o uso. O benchmark sintético existente não é, isoladamente, uma justificativa para introduzir esse índice.

**Benefício potencial:** reduzir trabalho repetido e tempo até a publicação de grupos em bibliotecas grandes, ao custo de mais estado persistido e regras de invalidação.

**Possível implementação:** armazenar relações e evidências associadas às versões dos fingerprints e das regras; calcular o conjunto de relações e grupos afetados; invalidar dependências de anchors removidos ou alterados; publicar por revisão vigente. A política precisa preservar decisões de grupos ignorados, resolvidos e revisados. Recalcular apenas os pares do arquivo alterado pode ser insuficiente porque a mudança de um anchor reorganiza grupos dependentes.

Um índice somente por distância de dHash pode descartar crops aceitos pela corroboração multiscale atual. Qualquer redução de candidatos deve demonstrar que preserva os matches válidos e as regras existentes.

**Critério de aceite futuro:** resultados equivalentes ao caminho global para as mesmas entradas e regras, incluindo ordem de anchors, remoções, alterações e decisões do usuário; recuperação após crash e rejeição de publicação antiga; ganho medido em carga que justifique a memória, o banco e a manutenção adicionais.

## 5. Preferências de pausa e prioridade do viewport

**Do que se trata:** duas decisões de produto independentes: persistir a intenção de pausa entre sessões e permitir distinguir trabalho de fundo de thumbnails solicitadas pelo viewport. Hoje a pausa é da sessão e inclui os assets visíveis.

**Motivo:** alguns usuários podem desejar que uma pausa sobreviva ao reinício ou que a navegação permaneça atendida enquanto o processamento de fundo aguarda. Isso mudaria a política atual e precisa ser comunicado claramente.

**Benefício potencial:** controle mais ajustado ao fluxo de trabalho e ao consumo de recursos desejado pelo usuário.

**Possível implementação:** preferências explícitas com defaults e migração documentados; distinção de filas na admissão, preservando o orçamento compartilhado e a justiça entre solicitações. Uma verificação de ferramenta ou alteração de configuração não deve retomar uma pausa implicitamente. Pausar continua diferente de interromper uma chamada nativa em andamento.

**Critério de aceite futuro:** a interface identifica exatamente o que foi pausado; o reinício aplica a preferência escolhida; prioridade não ultrapassa o orçamento nem causa starvation; ações explícitas não criam estados contraditórios entre statusbar e configurações.

## 6. Telemetria adicional e progresso por bytes

**Do que se trata:** ampliar medições agregadas de espera, extração, banco, comparação e publicação e, onde houver leitura mensurável, apresentar bytes processados. Cada métrica deve representar uma etapa concreta; quantidade de operações não equivale a percentual de CPU.

**Motivo:** facilitar o diagnóstico se uma biblioteca ou formato específico apresentar lentidão no futuro. Contar arquivos pode ser pouco informativo quando os tamanhos variam muito.

**Benefício potencial:** distinguir custo de mídia, espera por capacidade, consulta e publicação; dar feedback mais útil para leituras longas.

**Possível implementação:** aproveitar contadores das leituras existentes e amostras agregadas por fase, sem uma segunda leitura só para medir. Coalescer atualizações no snapshot compartilhado, manter fases indeterminadas quando não houver total confiável e evitar eventos, toasts ou invalidações por byte/asset. Caminhos e conteúdo privado não devem fazer parte dos relatórios públicos.

**Critério de aceite futuro:** overhead limitado e medido; métricas com unidade e escopo definidos; progresso monotônico dentro da execução e protegido contra respostas antigas; nenhum aumento relevante de IPC ou reconstrução de listas.

## Como retomar uma evolução

Ao retomar uma evolução por necessidade ou decisão de produto, definir o problema observado, o benefício esperado, o contrato que muda, os custos de migração, os cenários Windows/macOS e os critérios de aceite antes da implementação. Benchmarks de carga real podem ser usados quando ajudarem essa decisão; não são uma dívida impeditiva do ciclo já encerrado.
