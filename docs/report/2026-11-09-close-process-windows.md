# Levantamento e plano: janela de finalização do Mundam

**Data:** 9 de novembro de 2026  
**Estado:** três novos encerramentos Windows em desenvolvimento limpos; matriz restante e aceite geral pendentes
**Escopo:** aplicações desktop Tauri em macOS, Windows e Linux  
**Destino:** `docs/report/`, conforme solicitado; este documento combina levantamento e planejamento técnico.

## 1. Resumo executivo

Ao fechar o Mundam, o backend solicita o encerramento de serviços em segundo plano antes de terminar o processo. Hoje a janela que recebeu o pedido continua aberta durante essa espera, sem informar o que está acontecendo. Uma operação demorada pode parecer um travamento e incentivar o fechamento forçado.

Recomenda-se criar uma **janela de finalização independente, leve e acessível**, visualmente relacionada à splash de inicialização, mas alimentada por um **estado de encerramento próprio, mantido pelo backend**. O encerramento deve começar imediatamente e continuar mesmo se a janela, o WebView ou o IPC falharem. Para que mesmo um encerramento rápido seja perceptível, propõe-se uma apresentação mínima de **600 ms após a confirmação de apresentação**, em paralelo à limpeza, limitada aos pedidos normais em que a janela esteja realmente disponível. Esse valor é uma proposta para validação de UX, não uma medição existente.

A janela é somente uma apresentação do ciclo de vida: não deve iniciar, controlar nem ser responsável pela conclusão da limpeza. Um único coordenador deve tratar pedidos repetidos, a concorrência com o boot e as diferentes entradas de saída. O relatório também identifica lacunas de rastreamento que precisam ser resolvidas ou explicitamente delimitadas para que a interface não prometa uma limpeza que o backend não consegue comprovar.

**Decisões centrais recomendadas:**

1. Criar uma entrada HTML/Vite e um WebView dedicados, com recursos locais mínimos, sem carregar a aplicação principal.
2. Interceptar os eventos de fechamento de janela e de saída da aplicação efetivamente suportados pelo Tauri, convergindo-os em um coordenador idempotente.
3. Reter um snapshot revisionado e publicar eventos; o cliente consulta o estado para recuperar eventos perdidos.
4. Apresentar progresso **indeterminado e factual**, sem percentual, duração estimada ou afirmação de dados salvos sem comprovação.
5. Iniciar a limpeza sem aguardar o frontend; manter o feedback pelo tempo mínimo somente depois de uma apresentação confirmada e nunca esperar indefinidamente pelo WebView.
6. Bloquear novas operações e drenar as já admitidas conforme sua semântica, preservando as garantias existentes de transação, journal e recuperação.
7. Ter fallback visível quando possível e saída autônoma quando não for; distinguir atraso, perda de contato, falha e término degradado.

## 2. Objetivo, limites e critérios de sucesso

### Objetivo

Informar imediatamente que o Mundam está finalizando, reduzir a percepção de travamento e manter a integridade de dados e recursos. A interface deve ser coerente com a splash sem sacrificar o encerramento confiável nem a acessibilidade.

### Dentro do escopo proposto

- Pedidos normais de saída da janela principal, da splash durante o boot e de mecanismos de saída da aplicação que sejam efetivamente interceptáveis.
- Uma única apresentação de finalização por execução, mesmo sob pedidos concorrentes.
- Coordenação com inicialização parcial, serviços registrados, operações em andamento, processos filhos e encerramento do servidor.
- Feedback de andamento, demora, perda de contato e falhas tratáveis; fallback quando a janela não puder ser apresentada.
- Testes automatizados, testes desktop em plataformas suportadas e documentação.

### Fora do escopo, salvo aprovação específica

- Garantir feedback ou limpeza perante encerramento forçado pelo sistema, falha de energia, crash, `SIGKILL` ou equivalente.
- Reescrever todos os workers, implementar histórico de shutdown, relatório de telemetria remota, novas métricas persistentes ou refazer a splash.
- Criar uma ação genérica de “forçar fechamento” ou cancelar o encerramento. Em especial, abortar uma tarefa não significa que uma operação nativa ou de disco tenha sido interrompida com segurança.
- Prometer que todos os jobs pendentes foram concluídos: trabalho recuperável pode ficar pendente para a próxima execução.

**Sucesso:** em saída normal suportada, o usuário vê um feedback perceptível; o backend só encerra após as garantias definidas para os recursos admitidos ou um desfecho degradado explicitamente registrado; não há duplicação de coordenação, operações novas após o fechamento da admissão, perda de integridade conhecida ou dependência indefinida da interface. As limitações que não possam ser garantidas devem ser registradas antes do aceite.

## 3. Guias e fontes consideradas

- [Padrões de documentação](../guidelines/documentation.md), [backend Rust](../guidelines/backend-rust.md), [frontend Solid](../guidelines/frontend-solid.md) e [arquitetura core](../guidelines/core-architecture.md).
- [Visão geral do backend](../backend/overview.md), [operações de biblioteca e recuperação](../backend/library-operations.md) e [reservas persistentes de processamento](../backend/workers/2026-11-03-23-32-processing-leases.md).
- [Inicialização e splash](../plans/2026-10-29-application-startup.md): fonte para padrões existentes, não uma garantia de que o desligamento já possua os mesmos mecanismos.
- [Entrada Tauri e eventos](../../src-tauri/src/lib.rs), [registro de ciclo de vida](../../src-tauri/src/lifecycle.rs), [bootstrap](../../src-tauri/src/bootstrap/startup.rs), [estado de startup](../../src-tauri/src/feature/startup.rs) e [adaptador de startup](../../src-tauri/src/delivery/tauri/startup.rs).
- [Cliente da splash](../../src/core/startup/client.ts), [apresentação](../../src/components/features/startup/SplashScreen.tsx), [configuração Tauri](../../src-tauri/tauri.conf.json), [capability da splash](../../src-tauri/capabilities/splashscreen.json) e [configuração Vite](../../vite.config.ts).

Este levantamento é estático: não foram feitas medições de desempenho, testes de interação desktop ou validação visual para esta proposta. O comportamento dos eventos nativos em cada plataforma permanece sujeito a verificação.

## 4. Estado atual e achados

### 4.1 Sequência observada

1. O setup registra `LifecycleRegistry` e inicia o boot assíncrono em [bootstrap/startup.rs](../../src-tauri/src/bootstrap/startup.rs).
2. A splash é aberta pela configuração nativa; a principal começa oculta. Depois do handshake de readiness, a principal é exibida e a splash é fechada.
3. Em [lib.rs](../../src-tauri/src/lib.rs), `CloseRequested` da principal ou da splash antes de readiness é interceptado com `prevent_close()`. A splash pronta e janelas auxiliares são tratadas separadamente por `should_shutdown_on_close`.
4. Uma tarefa assíncrona chama `shutdown_all()` se o registro estiver disponível e então `handle.exit(0)`. O pedido não fecha imediatamente a janela interceptada; não existe apresentação ou estado de finalização específico nesse fluxo.
5. `shutdown_all()` cancela o token raiz, retira os handles registrados e os aguarda **sequencialmente**, com timeout de cinco segundos **por handle**; em timeout chama `abort()`. Erros são registrados em log e a função retorna `()`.

O cliente da splash já usa evento, consulta de snapshot, revisão monotônica e limpeza dos listeners. Sua UI usa mensagem em inglês, progresso indeterminado, `role="status"`, adaptação a tema do sistema e `prefers-reduced-motion`. É uma boa referência, mas não basta renomear as mensagens: o ciclo de finalização tem regras próprias.

### 4.2 Pontos relevantes para a solução

| Achado no código | Consequência para o plano |
| --- | --- |
| `CloseRequested` não tem guarda de “já encerrando”; cada pedido pode iniciar outra tarefa. | A segunda execução pode encontrar o mapa já esvaziado e solicitar saída antes da primeira terminar. É necessário um dono único e um mecanismo atômico de adesão ao mesmo encerramento. |
| Boot é disparado separadamente; `register()` aceita novos handles depois de `shutdown_all()` retirar o mapa. | Fechar durante o boot pode concorrer com inicialização e registro tardio. Token cancelado por si só não comprova que não houve novo recurso ou tarefa fora da drenagem. |
| Cada handle recebe seu próprio timeout em sequência e a iteração é de `HashMap`. | Não há limite global de cinco segundos, ordem determinística nem progresso por etapas inferível da ordem atual. O pior caso pode crescer com a quantidade de tarefas. |
| Timeout faz `abort()` sem aguardar novamente o handle; panic e timeout viram logs, não resultado estruturado. | Não se pode afirmar que todas as operações internas ou nativas terminaram. Uma saída degradada precisa ser identificada, registrada e testada. |
| Em [sensor.rs](../../src-tauri/src/processing/watcher/sensor.rs), `watch` ignora o token e cria loops com `tokio::spawn`; em [bootstrap/library.rs](../../src-tauri/src/bootstrap/library.rs), o handle registrado por watcher é uma tarefa vazia. | O handle registrado não prova encerramento do watcher nativo nem de seus loops. O plano precisa de desligamento real e verificável, não apenas mudança de rótulo. |
| O servidor em [server.rs](../../src-tauri/src/delivery/streaming/server.rs) registra o handle de `axum::serve`, mas cria loops de limpeza independentes, sem seus handles na drenagem. | Cancelamento de filhos não equivale a aguardar seu término. Requisições ativas, loops e gerenciadores precisam ter propriedade e conclusão explícitas. |
| O [HlsManager](../../src-tauri/src/feature/transcoding/hls_manager.rs) limpa sessões com `kill_on_drop`; [LinearManager](../../src-tauri/src/delivery/streaming/linear_manager.rs) mata/aguarda filhos ao limpar sessões expiradas, mas o loop do servidor apenas interrompe na saída; [ProcessManager](../../src-tauri/src/delivery/streaming/process_manager.rs) rastreia PIDs e [segment.rs](../../src-tauri/src/delivery/streaming/segment.rs) aguarda subprocessos em requisições. | Não afirmar que todos os processos foram encerrados/recolhidos apenas porque o servidor retornou. Auditar cada dono e seus caminhos de cancelamento, inclusive requisições em voo e subprocessos fora desses gerenciadores. |
| [DbManager](../../src-tauri/src/infra/database/manager.rs) usa pool SQLx e WAL; o fluxo de saída inspecionado não invoca fechamento explícito desse pool. | Identificar proprietários e conexões ativas antes de decidir se há fechamento explícito seguro. Não adicionar checkpoint, flush ou mensagem “salvando” sem necessidade e sem comprovação. |
| [Reservas de processamento](../backend/workers/2026-11-03-23-32-processing-leases.md) e [journal de operações](../backend/library-operations.md) oferecem mecanismos de recuperação com limites próprios. | Preservar publicação transacional, propriedade das leases, arquivos originais e recuperação no boot; não prometer conclusão de jobs ou recuperação universal. |

O documento histórico de [controle dos workers](../backend/workers/2026-10-30-05-17-workers-processing-control.md) registra problemas anteriores de encerramento; seu diagnóstico é histórico e não deve ser assumido como atual sem conferir os arquivos presentes.

### 4.3 Entradas e diferenças entre plataformas

O botão customizado de Windows/Linux chama `getCurrentWindow().close()` em [WindowControls.tsx](../../src/components/layout/TitleBar/WindowControls.tsx); no macOS há controles nativos. O código inspecionado trata `WindowEvent::CloseRequested`, não `RunEvent::ExitRequested`. Portanto, **não está demonstrado** que todas as formas de “Quit” do menu, Dock, atalhos, encerramento de sessão do SO e outras origens passem pelo mesmo fluxo.

Na versão Tauri instalada, `AppHandle::exit` também dispara `ExitRequested`, e o evento distingue saída solicitada pelo usuário (`code: None`) de saída programática (`Some`). A API documenta que `prevent_exit` é ignorado para restart. A implementação deve verificar o comportamento real e a precedência dos eventos em cada plataforma, inclusive ao sair após o próprio coordenador, sem criar um ciclo infinito. A distinção por código é informação auxiliar, **não** prova suficiente de autorização para sair: usar o estado interno do coordenador como autoridade. Restart, término forçado e solicitações de shutdown do sistema podem ter restrições distintas e precisam de testes; não prometer interceptação universal.

## 5. Alternativas de apresentação

| Alternativa | Vantagens | Limitações | Avaliação |
| --- | --- | --- | --- |
| Overlay na janela principal | Imediato, reutiliza o frontend já carregado. | Não cobre boot/falha da principal; pode herdar bloqueio, navegação ou estado quebrado. | Útil apenas como fallback. |
| Transformar a splash existente | Já possui identidade e cliente de estado. | Foi fechada após startup; reaproveitar label e fluxo mistura semânticas e pode reabrir/alterar o comportamento do boot. | Reutilizar padrões ou componentes realmente comuns, não o estado de startup. |
| **Janela dedicada e independente** | Cobre saída normal e boot parcial; permissões restritas; superfície visual consistente. | Criação/renderização podem falhar ou demorar; há custo de WebView e detalhes nativos por plataforma. | **Recomendada**, com fallback e timeout. |
| Diálogo nativo | Menor dependência do frontend em alguns ambientes. | Aparência e recursos acessíveis variam; APIs e comportamento bloqueante exigem estudo. | Fallback potencial, somente se disponível e testado. |

Janela dedicada **não** é garantia de visibilidade: compositor, minimização, virtual desktops, tela desconectada, acessibilidade do WebView e saída do SO podem impedir a apresentação. Não usar `alwaysOnTop` global por padrão nem roubar foco de outros aplicativos. Testar agrupamento na taskbar/Dock, foco e posição antes de escolher opções nativas definitivas.

## 6. Experiência proposta

### 6.1 Composição e mensagens

Tela enxuta, sem barras de ferramentas, em proporção próxima à splash atual (480 × 360 como ponto de partida, não como requisito rígido). Identidade Mundam, título explícito, status e indicador indeterminado. Centralizar na tela pertinente quando possível, respeitar escala e área útil, e permitir que o conteúdo se adapte a texto ampliado.

O produto usa inglês na splash existente; portanto, exemplos abaixo seguem esse padrão até existir infraestrutura de localização aplicável. Não duplicar strings em vários pontos; permitir futura tradução.

| Situação comprovada | Exemplo de texto | Regra |
| --- | --- | --- |
| Pedido aceito | **Closing Mundam** / “Finishing up. Please wait.” | Exibir imediatamente; não sugerir que o trabalho está concluído. |
| Admissão fechada e drenagem iniciada | “Stopping background activity.” | Só anunciar após essa transição ter ocorrido. |
| Espera prolongada, após 8 s | “Taking longer than usual · 8s” | Tempo decorrido real e monotônico, atualizado visualmente sem anúncio por leitor de tela a cada segundo. |
| Sem contato IPC por 5 s | “Waiting for the application to respond.” | Diferenciar falta de resposta de uma falha comprovada; pausar animação de progresso, continuar tentando consultar. |
| Falha conhecida ainda tratável | “Mundam could not finish closing normally.” | Mostrar código seguro e orientação adequada ao desfecho real; detalhes somente nos logs. |
| Resultado degradado com saída automática | Registrar diagnóstico e usar mensagem compatível apenas se houver tempo real de exibição. | Não apresentar “concluído com sucesso” após timeout/panic nem bloquear indefinidamente para exibir uma mensagem. |

Os limiares de 5 e 8 segundos seguem a splash como **ponto de partida**, não constituem prova de travamento. Usar relógio monotônico e constantes nomeadas. As fases de trabalho só devem aparecer se corresponderem a marcos observáveis; não inferir progresso por nome de tarefa, quantidade de handles ou passagem de tempo. Sem porcentagem, ETA ou frases como “All changes saved” sem garantia real.

### 6.2 Interação e acessibilidade

- A saída já foi solicitada: não oferecer cancelamento/reinício quando não houver reversão segura. Durante o fechamento, nova tentativa de fechar a janela de progresso é interceptada e se junta ao mesmo fluxo; não cria outra instância nem força a saída.
- `main` com título semântico, indicador com `role="progressbar"` indeterminado e nome acessível, e uma única região `role="status"`/`aria-live="polite"` para mudanças relevantes. Nenhum `aria-valuenow` fabricado.
- Sem depender apenas de animação ou cor; respeitar `prefers-reduced-motion`. Contraste de texto e componentes não textuais no mínimo WCAG AA aplicável, inclusive estados de foco.
- Manter controles de fallback acessíveis via teclado quando existirem, com foco visível e sem armadilha de foco. Não criar botão decorativo sem ação. Verificar leitura inicial e atualizações com leitores de tela reais, pois um status criado antes de ganhar foco pode não ser anunciado.
- Usar tokens existentes e CSS colocalizado, com variantes claro/escuro. A splash usa preferência do SO; se o tema persistido estiver disponível sem dependência de boot, avaliar seu uso, mas na ausência dele seguir o SO sem bloquear a apresentação. Testar alto contraste, zoom, escalonamento e textos longos.

### 6.3 Tempo mínimo sem prejudicar limpeza

Recomendação inicial: **600 ms de visibilidade confirmada**, não 600 ms antes de começar a limpeza. O backend usa relógio monotônico. A UI sinaliza uma única vez que montou o conteúdo e teve oportunidade de renderizar; esse handshake é uma aproximação, não prova absoluta de pixels visíveis ou anúncio pelo leitor de tela. Testar apresentação real com o compositor.

O encerramento pode terminar antes do handshake; nesse caso, aguardar somente o menor tempo restante de uma janela de apresentação **limitada**, proposta de **1.500 ms desde o pedido**, e só cumprir o mínimo se houver confirmação válida e janela mostrada. Exemplo: limpeza em 100 ms, confirmação em 200 ms: sair em torno de 800 ms; sem confirmação até 1.500 ms: seguir sem espera adicional. O handshake jamais retarda o início do cancelamento. Em saída do SO sujeita a prazo externo, falha de criação ou indisponibilidade, priorizar a saída segura e não impor esse mínimo. Os valores devem ser validados e aprovados antes da implementação.

## 7. Arquitetura e protocolo recomendados

### 7.1 Componentes e responsabilidades

```text
CloseRequested / ExitRequested elegível
          │
          ▼
Adaptador nativo → coordenador único de encerramento
          │             ├─ fecha admissão e coordena boot
          │             ├─ solicita cancelamento / drena recursos possuídos
          │             ├─ retém estado + publica snapshots
          │             └─ decide saída final independente da UI
          ▼
Janela de finalização ← evento + consulta do snapshot
          └─ confirmação limitada de apresentação; nunca controla a limpeza
```

Separar regras e estado de ciclo de vida da integração Tauri, conforme as camadas da [visão geral](../backend/overview.md). O shutdown é estado da aplicação, não `DomainEvent` da biblioteca. O adaptador transforma eventos nativos em intenções, valida janela chamadora e expõe IPC mínimo; o coordenador delega a serviços/portas responsáveis pelos recursos, sem SQL ou operações de filesystem espalhados em `lib.rs`.

Organização candidata, a confirmar na implementação:

| Superfície | Responsabilidade |
| --- | --- |
| `src-tauri/src/feature/shutdown/` | Estado, transições, resultado e coordenação testável, independente de Tauri. |
| `src-tauri/src/bootstrap/` e `delivery/tauri/` | Composição, eventos de janela/aplicação, publicação e comandos finos. |
| [lifecycle.rs](../../src-tauri/src/lifecycle.rs) e donos dos recursos | Admissão/registro, cancelamento e drenagem reais, com resultado explícito. |
| `src/core/shutdown/` | Contrato tipado, observação e reconciliação de IPC com limpeza de listeners. |
| `src/components/features/shutdown/` | Componente Solid e estilos próprios; abstrair elementos compartilhados com startup apenas se houver responsabilidade comum real. |
| `shutdown.html`, `src/shutdown.tsx`, [vite.config.ts](../../vite.config.ts) | Entrada independente, empacotada no build; apenas dependências essenciais. |
| `src-tauri/capabilities/` e `permissions/` | Capability exclusiva para a label escolhida, consulta de estado, escuta/remoção de listener e, se necessário, confirmação de apresentação. |

Nomes são propostas, não arquivos já existentes. Seguir os guias de nomes completos, responsabilidade única, documentação TSDoc/Rustdoc, erros explícitos e limites de tamanho; analisar o uso da abstração frontend de Tauri prevista em [core-architecture.md](../guidelines/core-architecture.md). A splash atual usa `invoke`/`listen` diretamente no cliente dedicado: qualquer exceção necessária para bootstrap independente deve ser justificada, não propagada a componentes.

### 7.2 Máquina de estados e contrato

Estado sugerido: `idle → requested → draining → finished`, com resultado terminal `clean` ou `degraded`. A apresentação da janela é estado **ortogonal** (`pending`, `presented`, `unavailable`), não uma fase que bloqueia a drenagem. Para boot parcial, ausência de um serviço nunca inicializado é um caso esperado; falha de um serviço já registrado não é sucesso.

Exemplo ilustrativo de snapshot:

```json
{
  "revision": 3,
  "phase": "draining",
  "result": null,
  "presentation": "presented",
  "errorCode": null
}
```

- `revision` cresce apenas quando o estado muda; não regride nem aceita retorno de estado terminal para ativo. Chamadas repetidas compartilham a mesma execução.
- `result` só aparece em `finished`; `degraded` cobre timeout/panic/limpeza incompleta conhecida. O código público é estável e seguro; logs não devem conter conteúdo sensível desnecessário.
- O tempo é medido localmente para UX; o backend mantém apenas os instantes monotônicos necessários às regras de saída. Não transmitir nomes de caminhos, PIDs ou detalhes de biblioteca à janela.
- `shutdown:changed` e `get_shutdown_state` são nomes candidatos. Assinar antes de consultar, rejeitar revisões antigas, consultar periodicamente para recuperar eventos perdidos e liberar timers/listeners inclusive em registro tardio. Resposta de mesma revisão confirma contato, como no startup.
- A confirmação de apresentação deve ser validada por label e estado, ser idempotente, não aceitar confirmação tardia depois do limite, nem permitir que a janela dispare `exit`. Autorizar o comando de leitura apenas às superfícies necessárias. Testar negação para `main`, `design-system`, `splashscreen` e outras labels segundo o contrato escolhido.

### 7.3 Sequência e garantias de concorrência

1. Registrar coordenador antes de disparar o boot. Ao primeiro pedido elegível, impedir fechamento/saída somente quando a API o permitir e adquirir atomicamente a execução de shutdown. Pedidos seguintes aderem à mesma execução; nunca iniciam outra limpeza ou chamam `exit` antecipadamente.
2. Fechar atomicamente a admissão de trabalho e de novos registros, coordenada com a etapa de inicialização e operações que os criam. O boot deve observar o cancelamento entre etapas, não anunciar `ready` nem mostrar a principal após shutdown, e sua tarefa supervisionada deve ser acompanhada. Uma função ainda em execução não deve conseguir registrar serviços depois de a drenagem ter capturado seus handles.
3. Resolver a corrida **criação versus registro**: preferir reservar admissão antes de criar/iniciar o recurso, com ownership que garante registro ou limpeza em caso de falha. Uma simples verificação seguida de `spawn` e `register` não é atômica; rejeitar o registro depois do spawn sem aguardar o handle também não basta.
4. Iniciar a apresentação e a limpeza em paralelo. Não esconder a janela principal/splash antes de haver confirmação suficiente de que existe um feedback substituto; se a criação/montagem falhar, manter ou adaptar a janela de origem quando possível, sem permitir novas operações. Garantir que o fallback também não impeça a saída.
5. Interromper fontes de novas requisições e trabalhos, sinalizar cancelamento e aguardar os recursos sob seus donos. Definir dependências explícitas: por exemplo, impedir novas requisições de streaming antes de limpar seus processos e só fechar conexões quando operações que as utilizam tiverem terminado. Executar em paralelo apenas recursos independentes; não impor uma ordem fictícia baseada no `HashMap`.
6. Retornar resultado estruturado por categoria, sem segredos. A política de timeout deve distinguir tarefas abortáveis, operações nativas não interrompíveis, subprocessos e mutações críticas. `abort()` é um pedido ao runtime e não prova término de `spawn_blocking` ou código nativo; se usado, acompanhar o desfecho do handle e registrar o limite. Não destruir dados ou desligar prematuramente uma operação transacional apenas para satisfazer um relógio.
7. Quando a política de segurança e o limite de apresentação aplicável forem satisfeitos, marcar terminal e autorizar **uma única** saída programática. O tratamento de `ExitRequested` reconhece essa autorização interna e não se auto-intercepta. Falhas ao solicitar saída são registradas e possuem fallback nativo verificável.

### 7.4 Inventário de recursos e limites de desligamento

Antes de escolher prazos definitivos, levantar e testar a propriedade de:

- inicialização, chamadas IPC e mutações de biblioteca já aceitas, inclusive journal, operações de arquivo e transações;
- watcher nativo, seus loops e fila de eventos; indexador, boot scan e operações filhas;
- workers de thumbnail, cores, duplicados, lixo, verificação de ferramentas, leases e tarefas filhas;
- servidor Axum, conexões e requisições ativas; gerenciadores HLS, linear e segmentos, inclusive encerramento e `wait`/reap de subprocessos sob propriedade do Mundam;
- tarefas de extração nativa/`spawn_blocking`, subprocessos de ferramentas e limpeza temporária destacada;
- bridge de eventos, serviços de configuração e todos os pools/conexões relevantes, inclusive eventuais plugins.

Para cada item, documentar dono, como parar novas entradas, como sinalizar, como comprovar término, dependências, tempo máximo ou impossibilidade de interrupção segura e estratégia de recuperação. Uma chamada isolada a `kill_on_drop`, um token cancelado ou um handle fictício não são comprovação de `wait` ou de liberação completa.

**Política proposta:** orçamento global para tarefas cooperativas, usando deadline compartilhado em vez de cinco segundos acumulados por tarefa; ao expirar, registrar o desfecho e aplicar apenas ações de escalonamento que sejam seguras para aquele recurso. O prazo exato e as operações que podem ser abortadas dependem do inventário e de medições. Para operação crítica sem cancelamento seguro, preservar integridade e continuar informando demora em vez de anunciar sucesso ou simplesmente matar por tempo. Se for impossível garantir tanto saída limitada quanto integridade, registrar a decisão de produto e a estratégia de recuperação antes de implementar. Não encerrar processos por nome nem por PID sem ownership verificável.

### 7.5 Falhas e fallback

| Falha | Conduta proposta |
| --- | --- |
| Janela não criada, não mostrada ou WebView não confirma | Log estruturado; manter feedback na origem quando possível; continuar limpeza e saída após limite de apresentação, sem espera infinita. |
| Evento perdido | Recuperar por consulta do snapshot; logar falhas de emissão. |
| IPC sem resposta | Mensagem de espera, animação pausada, reconexão limitada por intervalos; não converter ausência de resposta em sucesso ou erro fatal. |
| Panic, erro ou timeout de recurso | Registrar categoria e duração; resultado degradado. Aplicar somente política segura e verificável de cada recurso. |
| Fechamento repetido ou tentativa de fechar progresso | Manter um coordenador; impedir saída prematura quando possível; não abrir múltiplas janelas. |
| Boot parcial ou falho | Drenar apenas recursos realmente adquiridos, com supervisão do boot; não assumir banco, configurações ou principal presentes. |
| Processo morto externamente | Sem garantia de UI ou cleanup. Confiar somente nos mecanismos de recuperação efetivamente existentes e testados. |

Não exibir na janela dados internos de bibliotecas, caminhos, nomes de arquivos, stack traces ou argumentos de subprocessos. Usar `tracing` com início, origem normalizada, transições, duração por categoria, falhas e resultado; evitar telemetria remota e detalhes pessoais. O estado mantido em memória não substitui journal durável nem relatório de crash.

## 8. Segurança e compatibilidade

- A nova capability deve ser restrita à nova label; não herdar a capability ampla de [default.json](../../src-tauri/capabilities/default.json), acesso a banco, filesystem, biblioteca, opener ou comandos de mutação.
- Verificar tanto ACL quanto validação do chamador no backend para comandos sensíveis; eventos devem ser direcionados ou seus payloads limitados. Testar chamadas não autorizadas.
- Evitar recursos remotos; garantir que a nova entrada faça parte de `vite build` e do bundle Tauri em dev e produção. Manter a splash e o design system funcionando independentemente.
- Não alterar sem necessidade a política de fechamento de janelas auxiliares nem o comportamento de startup concluído. Fechamento da principal minimizada/oculta e múltiplas telas devem ser verificados.
- Não supor que toda solicitação de saída do SO possa ser adiada; documentar comportamento específico e exceções medidas por plataforma.

## 9. Plano de execução após aprovação

| Etapa | Trabalho | Saída verificável |
| --- | --- | --- |
| 1. Inventário e contratos | Mapear entradas de saída e ownership dos recursos; verificar eventos Tauri nas três plataformas e medir tempos atuais. Definir política por recurso e deadlines. | Matriz de recursos e fluxos reais, com lacunas, decisões e baseline registrados neste relatório. |
| 2. Coordenação | Criar estado monotônico e execução única; proteger registro/admissão e corrida com boot; testar origem múltipla e saída autorizada. | Testes determinísticos de concorrência e inicialização parcial aprovados. |
| 3. Drenagem real | Ajustar apenas os donos necessários, incluindo watchers, processos/servidor e tarefas filhas; preservar journals, leases e garantias das mutações. | Evidência por recurso de término ou desfecho degradado, com testes de falhas e recuperação. |
| 4. Janela e protocolo | Implementar entrada independente, capability mínima, estado/IPC, handshake limitado e fallback. | Build inclui recursos; janela funciona sem depender do ciclo da principal. |
| 5. UX e acessibilidade | Aplicar tokens, mensagens factuais, tema e movimento reduzido; avaliar foco e fallback. | Verificações visuais, teclado e leitor de tela registradas. |
| 6. Integração e aceite | Executar matriz de testes, medir latências, atualizar documentação e preencher checklist/área de implementação. | Evidências e limitações registradas; pronto para revisão e aceite. |

As etapas 2 e 3 podem ser refinadas juntas, mas a interface não deve declarar encerramento seguro antes de existir uma definição verificável do que foi drenado.

## 10. Estratégia de validação e critérios de aceite

### 10.1 Testes automatizados

**Rust — unidade e integração**

- Transições monotônicas, códigos seguros, terminal imutável, resultado limpo/degradado e invariantes do snapshot.
- Pedidos concorrentes de janela e aplicação: uma única execução de limpeza e uma única saída final; pedido adicional nunca termina processo enquanto o primeiro ainda drena.
- Barreira de admissão: registrar antes, durante e depois do shutdown; testar explicitamente a corrida entre reserva, criação e registro; recurso rejeitado é encerrado e aguardado.
- Boot fechado em cada marco, inclusive banco ainda indisponível, falha no boot, startup e shutdown disputando `ready`, registro tardio e apresentação tardia da principal.
- Dependências e conclusão: simular tarefas cooperativas, atrasadas, panic e timeout; verificar orçamento compartilhado, política específica, resultado degradado e ausência de afirmação de sucesso após abort.
- Watcher: token e loops reais, liberação da observação, não publicação após encerramento comprovado. Streaming: requisição ativa, fechamento de admissão, processos reais de teste sob controle próprio, confirmação de término/reap e ausência de novos filhos; não matar processos alheios.
- Operações de biblioteca em pontos críticos, rollback/commit e recuperação com banco em arquivo e reinício; leases, tarefas filhas e operações não canceláveis não podem publicar resultados obsoletos. Reusar testes existentes onde aplicável.
- Eventos e IPC: acesso permitido/negado por label, leitura tardia, confirmação idempotente, falha de publicação e prevenção de recursão no `ExitRequested` final.

**Frontend — Vitest/Solid**

- Listener antes da consulta; respostas fora de ordem; mesmo revision confirma contato; evento perdido recuperado por polling; limpeza de timer e listener tardio.
- Estados de demora e indisponibilidade com timers/relógio simulados; recuperação do contato; nenhum percentual fabricado ou anúncio repetido a cada segundo.
- Montagem, acessibilidade semântica, tema, movimento reduzido, texto ampliado e confirmação de apresentação uma única vez; erro de montagem/IPC e fallback.
- Relógio controlado para limpeza rápida/lenta, handshake antes/depois do término, ausência ou confirmação tardia, prazo de 1.500 ms e mínimo de 600 ms. Testar que iniciar limpeza não aguarda o frontend.

**Comandos iniciais sugeridos**, ajustar filtros aos nomes reais quando houver implementação:

```sh
npm run typecheck
npm run lint
npm run build
npm run test -- src/core/startup/client.test.ts
cargo test --manifest-path src-tauri/Cargo.toml feature::startup::tests
```

Acrescentar os testes focados novos de shutdown, lifecycle, watchers, streaming e recuperação, além de regressões das áreas alteradas; executar o conjunto mais amplo quando mudanças de coordenação afetarem múltiplos subsistemas. Nenhum desses comandos foi executado para este relatório, pois não houve alteração de código.

### 10.2 Matriz desktop obrigatória

Executar em **build empacotado** em macOS, Windows e Linux, além do modo de desenvolvimento. Registrar versão do SO, meio de execução, resultado, duração e evidências, sem dados pessoais. Se algum ambiente não estiver disponível, marcar explicitamente como pendente, não como aprovado.

| Cenário | Verificações |
| --- | --- |
| Biblioteca vazia e existente; saída sem trabalho | Feedback visível, duração mínima proposta, saída real e sem janela residual. |
| Indexação, miniaturas, cores e duplicados ativos | UI responsiva, drenagem/recuperação coerente; reabrir e validar integridade dos dados e originais. |
| Vídeo/HLS/transcodificação e requisições ativas | Encerramento comprovado dos subprocessos próprios e do servidor; reabrir e verificar disponibilidade. |
| Operação de arquivo ou transação em andamento | Não perder originais; validar commit/rollback ou recuperação existente após reabrir. |
| Fechar durante cada fase da splash e após falha do boot | Sem reaparecimento da principal, sem novos serviços após drenagem, sem travar aguardando dependências ausentes. |
| Clique repetido, atalho, menu/Dock e saída programática elegível | Uma execução; confirmar quais caminhos realmente geram os eventos esperados em cada plataforma. |
| Janela principal minimizada, outra tela, mudança de escala/alto contraste | Feedback localizável sem foco indevido, layout íntegro e encerramento funcional. |
| Falso atraso, timeout, WebView ou evento indisponível | Mensagens corretas, fallback, saída conforme política e logs adequados. |
| Janelas auxiliares e splash após readiness | Fechamento independente preservado; não disparar encerramento global inadvertido. |
| Leitor de tela, teclado, movimento reduzido e temas | Nome/título/status compreensíveis, foco útil, contraste e ausência de movimento obrigatório. |
| Encerramento de sessão do SO e fechamento forçado | Registrar o que é interceptável e prazos observados; não alegar garantias impossíveis. |

### 10.3 Medições de aceite

- Instrumentar: `pedido → primeira apresentação confirmada`, `pedido → cancelamento`, `pedido → término da drenagem`, `pedido → saída`; distinguir confirmação do frontend de visibilidade observada.
- Confirmar por teste automatizado que, em saída normal com apresentação confirmada, a janela permanece pelo menos **600 ms** a partir da confirmação; que a ausência de confirmação não adiciona mais de **1.500 ms desde o pedido** por motivo de apresentação; que a limpeza começa sem esperar handshake. Considerar precisão dos timers e registrar a tolerância adotada no teste.
- Medir distribuição, não apenas um caso: ao menos 10 execuções quentes e 10 frias por plataforma, em repouso e com carga representativa; registrar mediana, máximo, metodologia e hardware. Não estabelecer meta de latência de criação sem baseline.
- Comparar consumo e tempo com a versão anterior; o único atraso deliberado de saída normal deve ser o mínimo de apresentação definido, sobreposto à limpeza. Nenhum timeout de interface deve prolongar a saída indefinidamente.
- Registrar como pendente qualquer resultado ainda não observado; não interpretar teste unitário de mocks como evidência de funcionamento nativo, acessibilidade real ou processo filho encerrado.

## 11. Riscos, escolhas para aprovação e perguntas abertas

| Tema | Recomendação inicial | Risco/decisão |
| --- | --- | --- |
| Tempo mínimo | 600 ms, medidos após confirmação, em paralelo à limpeza. | Acrescenta latência deliberada em saídas rápidas; validar percepção e aceitar valor. |
| Limite de apresentação | 1.500 ms desde pedido; sem mínimo se não houver confirmação válida. | Sob carga, janela pode não ser vista; registrar fallback e medir. |
| Saída degradada | Prosseguir somente após aplicar política segura por recurso; log explícito. | Uma operação não interrompível pode impedir limite global rígido; acordar política após inventário. |
| Controles | Sem cancelar nem forçar durante encerramento normal. | Se existir espera indefinida real, avaliar resposta acionável e recuperação segura em proposta separada; não oferecer botão sem semântica confiável. |
| Tema | Preferência do SO como fallback, semelhante à splash. | Aparência persistida pode diferir; avaliar disponibilidade segura e consistência. |
| Escopo de saída | Todos os caminhos normais que o SO/Tauri de fato permitem interceptar. | Reinício, logout e terminação externa têm semânticas próprias; documentar cobertura por plataforma. |
| Localização | Inglês conforme splash atual; mensagens centralizadas. | Traduzir quando houver padrão de localização, sem introduzir sistema novo apenas para esta tela. |

**Recomendação para aprovação:** validar a janela independente, os tempos propostos, a ausência inicial de ação forçada e a prioridade à integridade. A escolha de prazos de drenagem e de comportamento em operações não interrompíveis depende obrigatoriamente do inventário e das evidências da etapa 1.

## 12. Lista de entregas e acompanhamento

Preencher durante a implementação. Um item só deve ser marcado quando houver evidência registrada na seção seguinte.

- [x] Confirmar decisões de UX, tempos e cobertura de saída com o responsável pelo produto.
- [ ] Inventariar entradas reais de saída por plataforma e classificar as não interceptáveis.
- [ ] Inventariar recursos, proprietários, dependências, operações críticas e política de timeout/recuperação.
- [ ] Registrar baseline de tempo e consumo nas plataformas disponíveis.
- [x] Implementar coordenador único, estado revisionado e saída final sem recursão.
- [ ] Garantir fechamento atômico da admissão e supervisão do boot, incluindo criação/registro concorrentes.
- [ ] Garantir drenagem verificável dos recursos no escopo, watchers e tarefas filhas.
- [ ] Auditar e tratar servidor, requisições e processos filhos sob propriedade do Mundam.
- [ ] Preservar transações, journal, leases, originais e recuperação após reinício.
- [x] Implementar janela independente, entrada Vite e empacotamento.
- [x] Criar capability mínima e validar autorização de IPC por janela.
- [x] Implementar protocolo evento + consulta, reconciliação e descarte dos listeners.
- [ ] Implementar apresentação mínima, limite de handshake, fallback e mensagens factuais.
- [ ] Validar acessibilidade, teclado, leitores de tela, contraste, temas e movimento reduzido.
- [ ] Cobrir falhas, timeouts, fechamento repetido, saída do SO e inicialização parcial.
- [x] Executar testes automatizados, typecheck, lint e build pertinentes.
- [ ] Executar e registrar matriz desktop e medições; explicitar plataformas não validadas.
- [ ] Atualizar documentação de startup, lifecycle e arquitetura afetada.
- [ ] Registrar decisões finais, desvios, riscos residuais, resultados e aprovação.

## 13. Detalhes da implementação e evidências

**Atualização de 9 de novembro de 2026:** primeira integração em andamento; os
itens ainda não marcados continuam pendentes de implementação ou validação.
Não considerar este registro como aceite final.

### Decisões aprovadas

| Data | Decisão | Responsável / referência |
| --- | --- | --- |
| 2026-11-09 | Plano aprovado; iniciar com janela independente usando Solid/Vite, simplificando para HTML/JS próprio caso necessário. | Solicitação do usuário nesta sessão. |
| 2026-11-09 | Primeira etapa mantém os tempos propostos de 600 ms e 1.500 ms e usa o estado de finalização separado do startup. | Implementação em andamento; validação nativa pendente. |

### Inventário e política de recursos

| Recurso / dono | Admissão e cancelamento | Evidência de término | Timeout / recuperação | Estado |
| --- | --- | --- | --- | --- |
| Inicialização | Token raiz cancelado e supervisor aguardado antes de drenar o registro; verificação entre etapas. | Handle do supervisor. | Sem timeout adicional para boot em andamento. | Implementado; teste de corrida e desktop pendentes. |
| Tarefas registradas | Token raiz cancelado; `shutdown_all` fecha o registro e aguarda handles concorrentemente. | Resultado do handle; panic/timeout marcam resultado degradado. | Prazo compartilhado de 5 s para espera cooperativa; abort não prova término de trabalho nativo nem impõe limite à espera posterior pelo handle. | Parcial; admissão anterior ao registro, política definitiva e cobertura pendentes. |
| Watchers, operações IPC, servidor, subprocessos e tarefas não registradas | Ainda utilizam mecanismos existentes. | Não estabelecida de forma abrangente nesta etapa. | Limitações e recovery existentes precisam ser auditados. | Pendente; não inferir limpeza integral. |

### Alterações realizadas

| Data | Arquivos / área | Alteração e justificativa | Referência |
| --- | --- | --- | --- |
| 2026-11-09 | `src-tauri/src/feature/shutdown.rs`, `bootstrap/shutdown.rs`, `delivery/tauri/shutdown.rs`, `lib.rs` | Estado revisionado, pedido único, eventos/consulta, confirmação e janela dedicada; saída interceptada quando o Tauri permite. | Fluxos básicos testados pelo usuário no macOS em modo de desenvolvimento; demais cenários pendentes. |
| 2026-11-09 | `bootstrap/startup.rs`, `delivery/tauri/startup.rs`, `lifecycle.rs` | Aguardar supervisor do boot; impedir apresentação da principal e avanços de boot após pedido; registrar tarefas com falha/timeout como degradadas. | Ainda não fecha atomicamente toda admissão de trabalho. |
| 2026-11-09 | `shutdown.html`, `src/core/shutdown/`, `src/components/features/shutdown/`, capability/permissão próprias, `vite.config.ts` | Apresentação independente e indeterminada, com tema e redução de movimento compartilhados com a splash. | Janela observada no macOS em modo de desenvolvimento; acessibilidade real e outros ambientes pendentes. |
| 2026-11-09 | `bootstrap/macos_shutdown.rs`, `bootstrap/shutdown.rs`, `lib.rs` | Encaminhar `applicationShouldTerminate:` do AppKit ao coordenador compartilhado. O delegate de Tao é preservado por subclasse sem ivars; se já houver implementação do seletor, a instalação falha explicitamente em vez de sobrescrevê-la. A saída autorizada retorna `NSTerminateNow`. | Dock e Cmd+Q validados pelo usuário no macOS em modo de desenvolvimento. |

### Drenagem de subprocessos do servidor de streaming

No cancelamento do token, o servidor de streaming tenta encerrar os processos
ffmpeg de segmentos rastreados e limpar as sessões lineares (processo e
diretório temporário) antes do desligamento gracioso do Axum. Cobertura: teste unitário de
`ProcessManager::cancel_all` (Unix). Sem teste de integração com sessões
lineares reais.

### Orçamento global e fallback

`LifecycleRegistry::shutdown_all` drena as tarefas concorrentemente com um prazo
único de 5 s (antes 5 s por tarefa, em sequência); estouro faz abort e marca
`clean=false`. Testes: `lifecycle::shutdown_budget_tests` (2). Se a janela de
encerramento não puder ser criada, o título da janela principal passa a
"Mundam — Closing…".

### Admissão atômica e pool SQLite

`LifecycleRegistry` marca `closed` sob o mesmo lock da drenagem final; registros
posteriores têm token cancelado e handle abortado (teste
`registration_after_shutdown_is_rejected_and_aborted`). Após drenagem limpa,
`DbManager::close` fecha o pool SQLite com limite de 3 s (estouro marca
`clean=false`). Se qualquer tarefa falhar ou exceder o orçamento, o fechamento
explícito do pool é ignorado, pois recursos órfãos ainda podem tentar liberar
leases antes da saída do processo. `cargo clippy --lib --tests` sem avisos;
`cargo test --lib`:
269 aprovados (uma execução anterior teve 1 falha não identificada, não
reproduzida na reexecução).

### Acompanhamento de leases de processamento

Os executores de thumbnails, cores e fingerprints agora aguardam a tarefa de
renovação e liberação do lease ao concluir o processamento, inclusive quando a
operação retorna erro. A falha de liberação é registrada e propagada ao
chamador. Testes do lease verificam espera pela conexão SQLite e propagação de
falha; os testes de processamento foram executados após a alteração.

Esse acompanhamento cobre o caminho de conclusão dos executores, não o abort
forçado: nesse caso o `Drop` ainda dispara liberação assíncrona sem aguardar.
Extrações e computações em `spawn_blocking` também não são interrompidas pelo
abort de uma tarefa assíncrona. As durações excessivas observadas nos logs
ainda não têm causa específica confirmada. Foram adicionados logs para informar
quantas tarefas de thumbnail estão em drenagem e quando a extração nativa de
cores continua em andamento após cancelamento.

### Observabilidade da drenagem

- HLS registra quantas sessões estavam rastreadas e que os handles foram
  descartados; `kill_on_drop` não comprova espera pelo processo nem remoção do
  diretório.
- Streaming linear registra a contagem, tempo e erros de processo e diretório;
  tentativas de cancelamento de segmentos registram o resultado do comando, não
  uma confirmação de término. Os erros de limpeza ainda não alteram `clean`.
- O worker de duplicatas identifica se aguardava lote de fingerprints ou
  comparação no momento do cancelamento, sem interromper o trabalho observado.
  Thumbnail e cores mantêm os diagnósticos de drenagem e extração nativa.
- SQLite informa início, número de conexões e o resultado do fechamento; no
  encerramento degradado o fechamento explícito é omitido como antes.
- Os spans dos endpoints de streaming não incluem mais os parâmetros de
  consulta, que poderiam conter o token da sessão. Logs anteriores podem
  conter esse token e devem ser tratados com cuidado ao compartilhar.

Na investigação posterior, o usuário informou que realizou seeks durante a
reprodução de vários vídeos HLS e que existem trabalhos pendentes de
thumbnails, fingerprints e cores. Pendência em fila não é equivalente a uma
tarefa em execução nem, por si só, causa de timeout. A contagem `pending_tasks`
do `JoinSet` inclui tarefas ainda não coletadas, inclusive eventuais tarefas já
concluídas.

Para as próximas amostras, operações de thumbnail e fingerprint que continuarem
após o cancelamento registram a fase atual aproximadamente uma vez por segundo,
sem caminho, identificador do asset ou token. O monitor não interrompe a
operação nem prova que o código nativo foi encerrado. Para visualizar segmentos
transcodificados ou servidos pelo cache, habilitar logs de diagnóstico com
`RUST_LOG=info,mundam_lib::delivery::streaming::segment=debug npm run tauri dev`.
Os logs existentes de outras áreas podem conter dados sensíveis; revisar antes
de compartilhar.

As novas mensagens de cancelamento de segmento distinguem `superseded` (nova
requisição para o mesmo segmento, inclusive durante navegação), `stale` e
`shutdown`. Na amostra anterior, ocorreu uma tentativa antes do pedido de
encerramento e duas durante a limpeza do servidor; sem o campo de motivo não
é possível atribuir com certeza todas as tentativas ao seek. A remoção do
registro agora verifica o PID, impedindo que a conclusão de uma requisição
antiga remova a entrada de uma requisição mais recente para a mesma chave.
Ainda há corridas de criação e registro e a identificação por PID não prova a
finalização do processo.

Um teste adicional revelou que, neste ambiente, o futuro de fechamento do pool
retornou mesmo com uma conexão adquirida ainda em posse do teste, apesar do
contrato documentado pelo SQLx. A falha foi reproduzida isoladamente; o teste
de regressão permanece ignorado até a investigação da causa. Portanto, o log
de retorno de `close` não deve ser interpretado como prova de término de
operações em conexões já adquiridas; é necessária supervisão explícita.

### Testes, medições e validação desktop

| Data | Comando / cenário | Plataforma / ambiente | Resultado e evidência |
| --- | --- | --- | --- |
| 2026-11-09 | `npm run typecheck` | Ambiente de desenvolvimento macOS | Aprovado. |
| 2026-11-09 | `npm run lint` | Ambiente de desenvolvimento macOS | Aprovado após formatação. |
| 2026-11-09 | `npm run test -- src/core/startup/client.test.ts src/core/shutdown/client.test.ts` | Vitest | 7 testes aprovados. |
| 2026-11-09 | `npm run test -- src/components/features/shutdown/ShutdownScreen.spec.tsx` | Vitest/jsdom | 1 teste aprovado; não substitui leitor de tela real. |
| 2026-11-09 | `npm run build` | Vite | Aprovado; `dist/shutdown.html` presente. |
| 2026-11-09 | Prévia HTTP de `shutdown.html` | Navegador, sem runtime Tauri | Layout e árvore acessível observados; estado de espera por IPC demonstrado. Não valida janela nativa nem a ponte Tauri. |
| 2026-11-09 | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` e `git diff --check` | Ambiente de desenvolvimento macOS | Aprovados. |
| 2026-11-09 | `cargo test --manifest-path src-tauri/Cargo.toml feature::shutdown::tests --lib` | Ambiente de desenvolvimento macOS | 4 testes aprovados, incluindo pedidos concorrentes e limite da confirmação. |
| 2026-11-09 | `cargo test --manifest-path src-tauri/Cargo.toml feature::startup::tests --lib` | Ambiente de desenvolvimento macOS | 3 testes de regressão aprovados. |
| 2026-11-09 | Fechamento pelo botão da titlebar | macOS, relato do usuário | Janela apareceu corretamente e duração foi considerada agradável; teste anterior à alteração de AppKit. |
| 2026-11-09 | Dock → Encerrar e Cmd+Q | macOS, relato do usuário | Antes da correção, janela não apareceu e a limpeza aparentemente não foi executada. |
| 2026-11-09 | Titlebar, Dock → Encerrar e Cmd+Q, após correção | macOS, `npm run tauri dev`; relato e saída de terminal fornecidos pelo usuário | Janela exibida nos três casos; logs mostram um pedido de encerramento por execução, 12 handles registrados concluídos e `clean=true`. Dock e Cmd+Q também registram o pedido nativo do macOS. |
| 2026-11-09 | Titlebar, Dock → Encerrar e Cmd+Q em build empacotado | macOS, bundle; relato do usuário, sem logs | Janela exibida corretamente com duração considerada agradável; encerramento concluído nos três casos. |
| 2026-11-09 | `cargo check --manifest-path src-tauri/Cargo.toml --lib` | macOS | Compilação da ponte AppKit aprovada. |
| 2026-11-09 | `cargo test --manifest-path src-tauri/Cargo.toml bootstrap::macos_shutdown::tests --lib` | macOS | Teste Objective-C de herança e recusa de sobrescrita aprovado; não simula Dock, Cmd+Q nem exibição da janela. |
| 2026-11-09 | Testes `feature::shutdown::tests` e `feature::startup::tests` | macOS | 4 e 3 testes aprovados após integração AppKit. |
| 2026-11-10 | Encerramento após alterações no lifecycle e pool | macOS, `npm run tauri dev`; relato e saída colada pelo usuário | Usuário relata funcionamento correto. Logs mostram pedido nativo, 12 handles registrados concluídos e `clean=true`; servidor informa zero processos de segmento e zero sessões lineares. O meio de encerramento não foi especificado. |
| 2026-11-10 | Execução com reprodução de mídia e encerramento | macOS, `npm run tauri dev`; logs colados pelo usuário | O servidor drenou 2 sessões lineares e 0 processos de segmento. Dois workers (`duplicate_worker`, `thumbnail_worker`) excederam o limite global de 5 s; resultado `clean=false`. Depois, três liberações de lease falharam porque o pool já estava fechado. Isso evidencia trabalho filho/desanexado fora do handle do worker; não é um encerramento limpo. |
| 2026-11-10 | Build após correção para não fechar pool quando a drenagem falha | `cargo fmt`, `cargo check --lib`, `cargo clippy --lib --tests` em macOS | Formatação, compilação e clippy concluídos sem avisos. O caminho degradado foi observado nos logs posteriores; ainda requer validação abrangente. |
| 2026-11-10 | Novo encerramento com sessões lineares | macOS, `npm run tauri dev`; logs fornecidos pelo usuário | Servidor relata 2 sessões lineares e 0 processos de segmento; `duplicate_worker` concluiu em aproximadamente 3,3 s, mas `thumbnail_worker` excedeu o prazo e foi abortado. Resultado `clean=false`; log confirma que o fechamento explícito do pool foi omitido. Não houve avisos de falha de release de lease nesta amostra. |
| 2026-11-10 | Encerramento durante transcodificação de segmento | macOS, `npm run tauri dev`; logs fornecidos pelo usuário | Gerenciador registrou cancelamento de 1 segmento; requisição registrou falha de FFmpeg e o servidor terminou. Os workers de thumbnails e duplicatas excederam o prazo e foram abortados; `clean=false` e fechamento explícito do pool omitido. Nenhuma falha de release de lease apareceu no log. |
| 2026-11-10 | Encerramento durante reprodução e processamento | macOS, `npm run tauri dev`; logs fornecidos pelo usuário | Gerenciador relatou cancelamento de 1 segmento e limpeza de 3 sessões lineares; `duplicate_worker` concluiu, mas `thumbnail_worker` e `color_worker` excederam o prazo e foram abortados. Resultado `clean=false`; fechamento explícito do pool omitido. A finalização dos subprocessos não foi confirmada independentemente. |
| 2026-11-10 | Testes de processamento e clippy após acompanhamento dos leases | macOS | `cargo test --lib processing_`: 81 aprovados; `cargo test --lib processing::`: 102 aprovados, incluindo espera por liberação de lease e falha do pool. `cargo clippy --lib --tests` sem avisos. Os testes não reproduzem o timeout nativo observado nos logs. |
| 2026-11-10 | Encerramento após acompanhamento de leases | macOS, `npm run tauri dev`; logs fornecidos pelo usuário | Thumbnail informou 2 tarefas em drenagem e excedeu o orçamento; foi abortado e o resultado foi `clean=false`. Duplicatas e cores concluíram; fechamento explícito do pool foi omitido. Servidor relatou 1 segmento e 0 sessões lineares; a requisição registrou `SEGMENT_ERROR` após o cancelamento. Não houve falha de release de lease no log. |
| 2026-11-10 | Nova execução com streaming e processamento | macOS, `npm run tauri dev`; logs fornecidos pelo usuário | Uma tarefa de thumbnail em drenagem e extração nativa de cores concluíram; o worker de duplicatas excedeu 5 s e foi abortado. Servidor relatou limpeza de 2 sessões lineares e nenhum segmento; resultado `clean=false` e fechamento explícito do pool omitido. Ainda não havia identificação da fase de duplicatas nem confirmação detalhada da limpeza. |
| 2026-11-10 | Testes após atualização da observabilidade | macOS, `cargo test --lib` e `cargo clippy --lib --tests` | 274 testes aprovados, 2 ignorados; clippy sem avisos. Entre os ignorados, o novo teste de espera por conexão adquirida falhou duas vezes quando executado anteriormente e requer investigação. Novas mensagens ainda não foram validadas no app em execução. |
| 2026-11-10 | Encerramento durante reprodução de vídeo HLS, após atualização da observabilidade | macOS, `npm run tauri dev`; relato e logs fornecidos pelo usuário | HLS factory: 0 sessões rastreadas; servidor: 2 tentativas de cancelamento de segmentos, sendo uma com comando aceito e outra com retorno não zero. Sessões lineares: 0. Thumbnails e duplicatas excederam o prazo; `clean=false` e fechamento explícito do pool omitido. |
| 2026-11-10 | Instrumentação por fase e correção da remoção de segmento | macOS, `cargo test --lib`, testes de streaming e `cargo clippy --lib --tests` | Suíte: 276 aprovados e 2 ignorados; testes de streaming após ajustes finais: 4 aprovados; clippy aprovado. O linker emitiu o aviso já registrado de versão mínima de macOS. Diagnósticos ainda precisam de validação no aplicativo. |
| 2026-11-10 | Execução com logs de fase e segmentos em DEBUG | macOS, `npm run tauri dev`; logs fornecidos pelo usuário | Logs mostram segmentos transcodificados e servidos pelo cache. Após cancelamento, duas tarefas de thumbnail permaneceram em `thumbnail_extraction` e um fingerprint em `visual_hash` até o prazo; os dois workers foram abortados e `clean=false`. Limpeza linear informou 2 sessões, sem falhas relatadas; HLS factory e processos de segmento rastreados estavam vazios no shutdown. |
| 2026-11-10 | Encerramento com processamento e reprodução ativos em perfil release | macOS, `RUST_LOG=info,mundam_lib::delivery::streaming::segment=debug npm run tauri dev -- --release`; logs fornecidos pelo usuário | Backend Rust em `release`, frontend servido pelo Vite de desenvolvimento. Thumbnail informou 0 tarefas no `JoinSet` e terminou; duplicatas aguardou `fingerprint_batch`, com observação em `provider_extraction`, e concluiu antes do prazo. Os 12 handles registrados terminaram com `clean=true`; pool informou 20 conexões ociosas de 20 e fechamento retornou em 63 ms. Servidor registrou uma tentativa de cancelamento de segmento e limpeza de 3 sessões lineares sem falhas relatadas. |
| 2026-11-10 | Primeira indexação após zerar dados da aplicação | macOS, `tauri dev` com backend `release`; logs e contexto fornecidos pelo usuário | `BatchCreate` registrou `database is locked` durante o scan. No encerramento, thumbnails, cores e duplicatas concluíram; 11 handles, `clean=true`. Pool informou 6 conexões, 5 ociosas, e retorno do fechamento em 17 ms. |
| 2026-11-10 | Reinício, processamento e reprodução | macOS, `tauri dev` com backend `release`; logs e contexto fornecidos pelo usuário | Boot scan falhou com `database is locked`. No encerramento, workers concluíram, mas `streaming_server` excedeu 5 s após iniciar drenagem HTTP e foi abortado; `clean=false` e fechamento explícito do pool omitido. Duas tentativas de cancelamento de segmento e uma sessão linear sem falhas de limpeza relatadas. |
| 2026-11-10 | Nova execução com processamento e reprodução | macOS, `tauri dev` com backend `release`; logs e contexto fornecidos pelo usuário | Boot scan concluiu sem erros relatados; thumbnails e duplicatas terminaram antes do prazo; 12 handles, `clean=true`. Uma tentativa de cancelamento de segmento e duas sessões lineares sem falhas relatadas. Pool informou 20 conexões, 19 ociosas, e retorno do fechamento em 52 ms. |
| 2026-11-10 | Testes de streaming após tornar esperas lineares canceláveis | macOS, `cargo test --lib delivery::streaming::`, `cargo clippy --lib --tests`, `cargo fmt --check` e `git diff --check` | 7 testes aprovados, incluindo arquivo pronto, timeout, cancelamento e teste HTTP local que valida resposta 503 e término do servidor. Clippy e verificações de formatação aprovados. Não substitui teste do handler real com Tauri, player e FFmpeg. |
| 2026-11-10 | Dados zerados, primeira indexação após alteração do streaming | macOS, `tauri dev` com backend `release`; logs fornecidos pelo usuário | `BatchCreate` falhou com SQLite 517. Encerramento: 11 handles, `clean=true`; thumbnail e cores concluíram e fingerprint em `provider_extraction` concluiu; servidor terminou com 0 segmentos e sessões lineares registrados. Pool: 11 conexões, todas ociosas; fechamento retornou em 47 ms. |
| 2026-11-10 | Segunda execução após alteração do streaming | macOS, `tauri dev` com backend `release`; logs fornecidos pelo usuário | Boot scan falhou com SQLite 5. Encerramento: 12 handles, `clean=true`; servidor terminou após duas tentativas de cancelamento de segmento, uma delas com comando não bem-sucedido, e limpeza de uma sessão linear sem falhas relatadas. Diagnóstico observou um handler `segment` ativo antes da parada. Pool: 9 conexões, 8 ociosas; fechamento retornou em 58 ms. |
| 2026-11-10 | Terceira execução após alteração do streaming | macOS, `tauri dev` com backend `release`; logs fornecidos pelo usuário | `Final BatchCreate` falhou com SQLite 517, seguido de scan que reportou `Errors: 0`. Encerramento: 12 handles, `clean=true`; servidor terminou após tentativa de cancelamento de um segmento e limpeza de duas sessões lineares sem falhas relatadas. Pool: 20 conexões, todas ociosas; fechamento retornou em 234 ms. |
| 2026-11-10 | Regressão de snapshot SQLite e recuperação de scan | macOS, `cargo test --lib`, testes direcionados e `cargo clippy --lib --tests` | Suíte: 282 aprovados e 2 ignorados antes do último ajuste no progresso; testes direcionados e clippy aprovados após o ajuste. Testes reproduziram SQLite 517 com WAL e transação diferida, verificaram lote concorrente com outro escritor e falhas injetadas de lote cheio e final seguidas de novo scan. Execução real no aplicativo continua pendente. |
| Pendente | Matriz desktop | macOS, Windows e Linux | Registrar resultado efetivo antes do aceite. |

Na amostra fornecida, o intervalo entre o log do pedido e o log de conclusão
da drenagem registrada foi de aproximadamente 3,5 ms pela titlebar, 78,8 ms
pelo Dock e 3,3 ms por Cmd+Q. Esses tempos **não medem** duração da janela,
apresentação, término do processo ou desempenho sob carga. Não houve mensagens
de erro, timeout ou aviso na saída compartilhada. O `clean=true` refere-se aos
handles registrados, não comprova a finalização dos subprocessos, watchers
nativos ou operações não rastreadas. A versão do macOS e as condições exatas
de carga não foram informadas; o build empacotado no macOS foi validado apenas por relato, sem logs; Windows e Linux continuam pendentes.

Na amostra anterior de 2026-11-10, decorreram aproximadamente 3,3 ms entre o
pedido e a conclusão dos handles registrados e 127 ms até o log do coordenador.
Não são medições da duração visível da janela nem do término do processo.

A amostra anterior relatou a limpeza de duas sessões lineares, mas não testou
processos de segmento. Também expôs que os workers de duplicatas e thumbnails
não concluem dentro do orçamento: tarefas filhas podem continuar com trabalho
de lease depois que o handle principal é abortado. As tentativas de release
falharam porque o pool fora fechado sem considerar o resultado degradado. A
sequência foi corrigida para não fechar explicitamente o pool quando a drenagem
não é limpa. Na nova amostra, esse caminho foi executado e não houve aviso de
falha de release; isso não comprova que todas as liberações terminaram antes da
saída ou que a corrida foi eliminada em todos os cenários. O servidor relatou
novamente duas sessões lineares e nenhum processo de segmento; como os erros da
limpeza não são propagados, o log não comprova o término dos subprocessos ou a
remoção dos diretórios. `duplicate_worker` terminou, mas `thumbnail_worker`
excedeu o prazo e o encerramento permaneceu degradado. O tempo entre o pedido
e o log final foi de aproximadamente 5,0 s, não uma medição da janela ou da
saída do processo. O bundle anterior não comprova as alterações posteriores.

Na amostra seguinte, o gerenciador registrou o cancelamento de um segmento
ativo e a requisição informou falha de FFmpeg durante o encerramento. Isso é
consistente com a interrupção solicitada, mas o código não verifica o resultado
do comando de cancelamento, portanto o log isoladamente não comprova que ele
terminou o processo; o `SEGMENT_ERROR` também não distingue cancelamento
esperado de erro inesperado. Ambos os workers de thumbnails e duplicatas
excederam novamente o orçamento de 5 s; não houve aviso de falha de release de
lease, mas isso não garante sua conclusão. Aproximadamente 5,01 s decorreram
entre o pedido e o log final do coordenador, sem medir a apresentação nem a
saída efetiva do processo.

Na amostra anterior, o diagnóstico registrou 2 tarefas de thumbnail
em andamento no início da drenagem; não identifica quais ainda permaneciam nem
a fase em que ficaram ao atingir o prazo. O worker de duplicatas terminou
aproximadamente 2,46 s após o pedido, e o de cores também terminou. Após cerca
de 5,0 s, o worker de thumbnail foi abortado e o coordenador reportou
`clean=false`. O aviso do pool confirma a omissão do fechamento explícito, mas a
ausência de erro de release não comprova que todas as liberações terminaram.
O servidor relatou 1 segmento cancelado e nenhuma sessão linear; o erro de
FFmpeg subsequente é consistente com o cancelamento, sem comprovar por si só
a causa. Antes do encerramento, houve também uma limpeza por timeout de
segmento antigo. Na compilação, o linker alertou que um objeto foi compilado
para macOS 13.1 enquanto o alvo da vinculação era 10.12; compatibilidade com
versões antigas requer investigação separada.

Na nova execução com reprodução HLS, o `HlsManager` relatou zero sessões
rastreadas e o gerenciador linear também informou zero sessões. O servidor de
segmentos tentou cancelar dois processos: um comando retornou sucesso e outro
retornou código 1, possivelmente porque o processo já havia terminado, mas a
causa não foi comprovada. A falha da requisição de segmento ocorreu após o
pedido de cancelamento. O servidor terminou, sem prova independente de que
ambos os processos foram encerrados. O worker de thumbnails iniciou a drenagem
com duas tarefas pendentes; o de duplicatas informou que ainda aguardava um
lote de fingerprints. Ambos foram abortados depois do orçamento de 5 s; não há
evidência suficiente para determinar a operação exata que os reteve.
O fechamento explícito do pool foi omitido, portanto seus novos logs de sucesso
não foram exercitados. Nos spans HTTP visíveis não aparecem parâmetros de
consulta, embora a amostra não prove cobertura completa de todos os logs.

Na amostra com instrumentação por fase, os logs de DEBUG confirmam exemplos
de segmentos transcodificados e de cache atendido durante a execução. Na
drenagem, o monitor registrou duas operações de thumbnail em
`thumbnail_extraction` e uma de fingerprint em `visual_hash` durante o
orçamento, até o abort dos workers. Essas fases localizam a espera, mas não
identificam o formato, o provider nem a função interna exata: não permitem
afirmar que houve travamento permanente ou identificar a causa raiz. A limpeza
linear relatou duas sessões sem erros de processo ou diretório; não havia
segmentos rastreados para cancelar nem sessões da HLS factory no encerramento.
Como o resultado foi degradado, o pool não foi fechado explicitamente e seu
caminho de sucesso continua sem validação nessa execução.

Na amostra seguinte, com backend Rust otimizado em perfil `release`, os logs
registraram aproximadamente 677 ms entre o pedido de encerramento e a
conclusão dos handles registrados e 740 ms até a conclusão informada pelo
coordenador. O fingerprint observado estava em `provider_extraction`, não em
`visual_hash`; thumbnails não tinha tarefas no `JoinSet` naquele momento.
Essas diferenças de carga e fase impedem concluir que a otimização eliminou o
timeout anterior ou estabelecer uma comparação controlada de desempenho. O
fechamento explícito do pool foi exercitado com todas as conexões informadas
como ociosas; não resolve a divergência observada no teste com conexão
adquirida. O servidor registrou uma tentativa de cancelamento de segmento por
`shutdown`, seguida de `SEGMENT_ERROR`, e três sessões lineares sem falhas
relatadas, em 76 ms. O erro do segmento é consistente com o cancelamento, mas
o registro não confirma independentemente o término do subprocesso. A HLS
factory não tinha sessões rastreadas. Foram também registrados avisos de
extração de cores sem pixels opacos e de decodificação de imagem pelo provider;
os logs não estabelecem relação desses avisos com o encerramento. O aviso do
linker sobre a versão mínima do macOS reapareceu.

Nas três execuções seguintes em `release`, os intervalos entre o pedido e o
log final do coordenador foram aproximadamente 337 ms, 5,01 s e 2,33 s,
respectivamente; não medem duração visível da janela nem saída do processo.
Na segunda, o servidor informou conclusão das tentativas de limpeza e início
da drenagem HTTP, mas não registrou `Streaming server stopped` antes do abort.
É necessário instrumentar e investigar conexões e requisições HTTP em voo;
os logs não identificam qual delas reteve o servidor nem provam a causa.
Os workers terminaram, e a publicação de eventos por thumbnails após o
cancelamento encontrou o barramento sem assinantes. A consequência funcional
precisa ser avaliada antes de definir a ordem de drenagem.

Na primeira e na terceira execução, os logs do pool indicaram respectivamente
6 conexões com 5 ociosas e 20 com 19 ociosas no início do fechamento. Isso
indica uma conexão não ociosa em cada fotografia, mas não identifica o seu
possuidor nem comprova que permanecia adquirida no momento em que `close`
retornou. A limitação do teste de conexão adquirida continua aberta.
Na primeira indexação houve falha de `BatchCreate` por bloqueio do SQLite e,
no reinício seguinte, o boot scan falhou pelo mesmo erro; o terceiro boot scan
concluiu sem erros relatados, sem comprovar recuperação completa dos dados.
A indexação disparada ao adicionar uma pasta usa tarefa destacada, não
registrada no lifecycle, e descarta o resultado do scan. Os logs não provam
que essa tarefa ainda estivesse ativa durante o shutdown; sua supervisão e
propagação de erros devem ser investigadas junto à contenção no SQLite.
Na implementação à época dessas amostras, um `BatchCreate` que falhava durante
a classificação era registrado no log, mas o contador de descobertos já tinha
sido incrementado e o contador de erros não incluía essa falha. Portanto, os
contadores dessas amostras não comprovam que todos os assets descobertos foram
persistidos.
O ledger abria uma transação diferida e a criação consultava dados antes da
escrita; a concorrência de escrita em WAL foi posteriormente reproduzida em
teste isolado, como descrito abaixo, sem identificar o escritor concorrente nas
amostras reais.
Também apareceram falhas de extração de thumbnails, inclusive conflitos de
filtros simples e complexos do FFmpeg na terceira execução; são problemas a
investigar separadamente, sem atribuir causalidade ao shutdown.

### Drenagem das requisições HTTP de streaming

O handler linear aguardava até 60 s por uma playlist e até 5 s por um
segmento sem observar o cancelamento. Essas esperas são uma hipótese para o
timeout observado, não sua causa comprovada. As duas esperas agora consultam o
token de shutdown e retornam HTTP 503 quando canceladas, sem abandonar
operações de criação ou encerramento dos subprocessos. Os limites e a
verificação da disponibilidade dos arquivos permanecem nos caminhos normais.

Durante a drenagem HTTP, um diagnóstico registra aproximadamente a cada
segundo quantos handlers ainda aguardam a geração da resposta, por categoria
de rota, sem caminhos, identificadores ou parâmetros de consulta. O registro
termina quando o handler devolve a resposta ou é descartado: não acompanha
corpos de resposta em transmissão nem conexões abertas. Assim, contagem zero
não prova que o Axum terminou de drenar. O monitor não cancela os handlers e
não confirma o encerramento de subprocessos. Uma execução real com player
ativo ainda é necessária para verificar se as esperas canceláveis resolvem o
caso observado e identificar outras requisições potencialmente demoradas. A
admissão e a criação de sessões ou processos concorrentes com a limpeza ainda
precisam de supervisão; a mudança das esperas, isoladamente, não fecha essa
corrida.

Nas três amostras posteriores, o coordenador concluiu em aproximadamente
1,54 s, 0,71 s e 0,58 s após o pedido, respectivamente, com `clean=true` para
os handles registrados. Na segunda amostra, o diagnóstico identificou um
handler `segment` ativo durante a drenagem; ele e o servidor terminaram em
seguida. Uma das duas tentativas de cancelamento de segmentos retornou status
não zero; o processo pode já ter terminado, mas isso não foi comprovado. Na
terceira amostra o snapshot indicou zero handlers, e o servidor terminou.
Na primeira, o servidor terminou sem snapshot visível, o que é compatível com
a conclusão rápida em relação ao monitor. Nenhuma amostra registrou o
cancelamento de espera linear: não demonstram que a correção resolveu o
timeout anterior nem cobrem respostas em transmissão ou conexões persistentes.

O SQLite retornou o código estendido 517 (`SQLITE_BUSY_SNAPSHOT`) em falhas de
`BatchCreate` na primeira e na terceira execução. Esse código caracteriza
conflito ao tentar promover uma leitura com snapshot desatualizado para escrita
em WAL, reforçando a hipótese de contenção no caminho transacional; os logs
não identificam a instrução ou a transação concorrente exata. A segunda
execução reportou apenas o código 5, insuficiente para concluir o subtipo.
Na terceira, o scan reportou 14 descobertos e `Errors: 0` apesar da falha do
lote final: o log demonstra que o resultado não deve ser interpretado como
confirmação da persistência dos assets descobertos.

### Persistência de lotes do scan

Um teste com SQLite em WAL reproduziu o código 517: uma transação diferida
leu um snapshot, outra conexão confirmou uma escrita e a primeira tentou
escrever. No ledger, `BatchCreate` agora inicia com `BEGIN IMMEDIATE`, como
outros caminhos de escrita existentes, adquirindo a reserva de escrita antes
das leituras do lote. Um teste com outro escritor ativo verificou que o lote
espera e persiste após a liberação. Isso não prova que todos os bloqueios
observados, inclusive o código 5 no boot scan, tenham a mesma origem.
A transação reserva a escrita durante o trabalho de cada lote, inclusive
operações de filesystem já presentes no handler; a contenção sob carga real e
a duração dessa reserva precisam ser medidas.

Se um lote falhar, o scan aguarda o produtor de classificação terminar, retorna
o erro e não executa pruning nem publica `ScanCompleted`; lotes anteriores
confirmados permanecem persistidos e um scan posterior pode retomá-los. A
contagem de descobertos só é incrementada após confirmação do lote. Não se
publica progresso de 100% após falha do lote final. O scan manual destacado
agora registra o erro retornado; a notificação de falha foi adicionada como
descrito abaixo, mas a tarefa ainda não é supervisionada no encerramento.
Testes injetam falhas no lote final e no primeiro lote cheio, verificam
ausência de evento de conclusão e de pruning e executam um novo scan.
Falhas individuais de classificação e de criação de pastas continuam com as
políticas anteriores e não são cobertas por esta mudança.

### Novas amostras e falha de indexação

Nas quatro amostras adicionais, o primeiro scan manual falhou com código SQLite
5; o segundo boot scan concluiu; o terceiro falhou com código 5; e o quarto,
com logging DEBUG, falhou com código estendido 517. Os quatro encerramentos
registraram `clean=true`, relativo aos handles registrados, não uma garantia de
drenagem integral de operações destacadas ou recursos nativos.

No último log, após um lote de assets, aparecem a consulta de asset e a escrita
de `library_file_identities` no contexto do scan, próximas à falha. Isso é um
forte indício de que a transação diferida de observação de identidades também
pode promover um snapshot obsoleto; o log não prova a causa de todos os códigos
5 nem identifica o escritor concorrente. A observação agora usa
`BEGIN IMMEDIATE` por lote de até 256 identidades; a criação de pastas também
passa a reservar a escrita antes de suas leituras. A reserva inclui verificações
de filesystem, portanto o impacto na contenção e na latência sob carga real
precisa ser medido.

Ao falhar, o indexador publica `ScanFailed`, preserva o erro original e não
publica conclusão de sucesso. A ponte emite `indexer:failed`: o frontend limpa
o progresso, atualiza os dados que podem ter sido parcialmente persistidos e
exibe uma notificação de erro. Falhas dessas atualizações são registradas. O
ledger deixou de emitir progresso de 100% por lote, que não representava o
progresso do scan inteiro. A contagem do scan ainda pode alcançar o total
antes de outras etapas, como identidades e pruning; apenas `ScanCompleted`
representa sucesso.

Limitações: o progresso continua global, sem identificador de operação; scans
simultâneos podem interferir nos indicadores ou notificações. Os listeners são
instalados após o início do backend, portanto um boot scan muito rápido ainda
pode terminar antes de o frontend receber seus eventos. Nenhuma destas
alterações comprova a resolução dos bloqueios no banco real; repetir o cenário
com uma biblioteca limpa e durante processamento concorrente.

**Validação automatizada desta etapa:** a suíte Rust em release passou com
288 testes e dois ignorados. Inclui concorrência real em WAL para criação de
lote, criação de pasta e observação de identidades; uma identidade antiga não é
atribuída a um arquivo substituído. Testes do indexador cobrem falha antes e
depois de `ScanStarted`, falhas de lote e de identidade, preservação do erro,
ausência de conclusão falsa e recuperação posterior. Cinco testes frontend
cobrem progresso, resultado, notificações e erro ao atualizar os dados;
typecheck, ESLint, Clippy em release com `-D warnings`, formatação Rust e
`git diff --check` passaram.
Os dois testes ignorados incluem o teste do pool com conexão adquirida, cuja
limitação permanece aberta. A validação desktop parcial é descrita a seguir.

### Três execuções posteriores no macOS

O usuário executou `tauri dev -- --release`, com logs de segmentos em DEBUG,
e informou que a primeira execução usou pasta de aplicação limpa e que a
indexação e a árvore de pastas voltaram a funcionar.

| Execução | Scan | Encerramento observado |
| --- | --- | --- |
| 1 — banco novo | Scan manual: 787 descobertos, nenhum inalterado, nenhum erro reportado, em 2,63 s. | `clean=true`; 11 tarefas registradas drenadas; aproximadamente 1,23 s entre a solicitação e a conclusão. |
| 2 | Boot scan: 14 descobertos, 773 inalterados, nenhum erro reportado, em 0,75 s. | `clean=true`; 12 tarefas registradas drenadas; aproximadamente 2,22 s até a conclusão. |
| 3 | Boot scan: 14 descobertos, 773 inalterados, nenhum erro reportado, em 0,87 s. | `clean=true`; 12 tarefas registradas drenadas; aproximadamente 1,37 s até a conclusão. |

Nenhuma das três amostras contém `database is locked`, `SQLITE_BUSY` ou aborto
forçado por orçamento de shutdown. Isso é evidência favorável para este cenário,
mas não demonstra ausência de contenção em outras cargas nem identifica a causa
de todos os erros anteriores. A falha de scan e sua notificação visual não foram
exercitadas por essas amostras.

O número 14 se repete nas duas inicializações; a investigação posterior desses
registros está detalhada abaixo.

Na primeira e na segunda execução, havia cancelamento de segmentos e uma sessão
linear na finalização; a limpeza linear reportou zero falhas de processo e
diretório (56 ms e 45 ms). Na primeira, uma solicitação de término de segmento
retornou status 1; não é possível concluir apenas pelo log se o processo já
havia terminado. Em ambas, houve `SEGMENT_ERROR` após o cancelamento, sem
evidência suficiente para tratar qualquer erro semelhante como cancelamento
esperado. Na terceira não havia segmentos ou sessões lineares registrados na
limpeza. Os handlers HTTP chegaram a zero nas duas primeiras, mas o contador
não rastreia corpos de resposta nem conexões.

As tarefas de thumbnail e fingerprint em andamento concluíram dentro do prazo
nestas amostras. Na segunda, o pool informou 13 conexões, 12 ociosas, antes de
`close`, que retornou em 84 ms; essa observação não comprova drenagem da
conexão restante, dado o teste ignorado e o trabalho não supervisionado. Na
primeira e na segunda, thumbnails ainda publicaram eventos após o encerramento
da ponte e sem assinantes. Persistência e reconciliação desses eventos seguem
sem comprovação.

Foram observados também erros de extração de thumbnails, inclusive previews
ausentes/inválidos e conflito de filtros simples e complexos do FFmpeg, além
de avisos de fontes ausentes e de extração de cores. Eles são distintos dos
erros de banco e merecem investigação própria; os logs não identificam com
segurança todos os assets nem demonstram que os arquivos sejam inválidos.
`Errors: 0` no scan não significa que processamento posterior tenha terminado
sem erros.

### Diagnóstico dos 14 assets redescobertos

Foi realizada consulta somente leitura ao banco de desenvolvimento fechado,
sem recriação da biblioteca nem alteração dos dados. Havia 787 registros de
assets e, no histórico de `CREATE_ASSET`, três grupos posteriores de 14
operações com exatamente os mesmos IDs. Na comparação com a listagem do
filesystem, os 14 caminhos estavam em NFC no catálogo e com caracteres
decompostos na listagem; os pares foram confirmados como o mesmo objeto físico.
Os tamanhos e tempos de modificação dos 787 registros consultados estavam
compatíveis com os arquivos existentes. Nessa amostra, portanto, os 14 assets
estavam persistidos: eram reclassificados por grafia distinta do caminho, não
há evidência de 14 novos registros a cada scan.

O cache do scan compara caminhos exatos. O ledger, por outro lado, resolve
aliases mediante identidade nativa e retorna o registro existente. Essa
diferença explicava a repetição dos contadores e das operações de criação.
Agora, quando uma grafia observada não está no cache, uma chave de busca
seleciona candidatos; o resolvedor existente verifica a identidade da entrada
antes de reutilizar metadados. A chave não é prova de identidade: entradas
distintas em volumes sensíveis e hardlinks continuam distintas. Pastas
observadas também são protegidas da poda por ID resolvido, mesmo quando sua
grafia diverge.

Para observar scans futuros, usar o comando abaixo em `src-tauri` ou na raiz.
Os módulos de classificação e indexação em DEBUG registram candidatos ausentes,
diferenças de metadados e aliases verificados; segmentos de streaming também
permanecem em DEBUG. Os logs podem conter caminhos e nomes de arquivos, que
devem ser revisados antes do compartilhamento.

```sh
RUST_LOG=info,mundam_lib::feature::library::indexer=debug,mundam_lib::feature::library::classifier=debug,mundam_lib::delivery::streaming::segment=debug npm run tauri dev -- --release
```

Uma validação representativa deve manter a pasta da aplicação existente,
executar a aplicação ao menos duas vezes sem alterar os arquivos e observar
os resultados dos scans e do encerramento. Para a biblioteca atual, espera-se
que os 14 aliases deixem de ser contabilizados como descobertas e que o total
de `CREATE_ASSET` não cresça apenas por reiniciar a aplicação. Os logs de
aliases contêm tanto a grafia observada quanto a grafia do catálogo.

**Testes:** a suíte Rust em release passou com 289 testes e dois ignorados;
o teste de scan repetido com grafias Unicode passou no macOS, assim como o
teste existente de aliases e hardlinks. Clippy em release com `-D warnings`
passou. O linker emitiu o aviso conhecido de versão mínima do macOS para
`zstd`. A validação real em macOS está descrita abaixo; a execução nos demais
sistemas permanece pendente.

**Validação real em macOS após a correção:** nas duas execuções em
`tauri dev -- --release`, cada scan registrou 14 aliases verificados e
`Discovered: 0`, `Unchanged: 787`, `Pruned: 0`, `Errors: 0`. Os tempos foram
1,08 s e 0,67 s. Consulta posterior ao banco fechado, somente leitura,
confirmou 787 assets e nenhum novo grupo de operações `CREATE_ASSET` após as
operações anteriores à correção. Os resultados comprovam o comportamento
esperado para esta biblioteca nas duas amostras, não em todos os volumes.

Os dois encerramentos terminaram com `clean=true`, respectivamente em cerca de
192 ms e 141 ms entre solicitação e conclusão do backend; esses tempos não são
uma medição da duração visual da janela. Não houve timeout de tarefa ou erro
de bloqueio SQLite nos logs. Nos snapshots anteriores ao fechamento, todas as
conexões do pool estavam ociosas (20/20 e 14/14). Os logs mostram segmentos
transcodificados antes da saída e tentativas de cancelamento durante ela:
na primeira, uma sessão linear foi limpa sem falhas reportadas; na segunda,
uma das tentativas de terminar segmento retornou status 1, sem comprovação de
que o processo já estivesse encerrado. Em ambas houve `SEGMENT_ERROR` após
iniciar o cancelamento. Esses resultados não eliminam os riscos de drenagem,
nem provam a conclusão de processos e recursos não supervisionados.

Se houver alterações reais em arquivos,
o caminho `ON CONFLICT` de criação ainda merece investigação própria: nessa
ramificação, tamanho e data não são atualizados; não foi a causa observada nos
14 casos. Os erros de thumbnail relativos a formatos conhecidos estão fora
desta investigação.

### Supervisão de processos de segmentos HLS

O cancelamento anterior identificava processos por PID e invocava ferramentas
externas de término. Isso não comprovava que o processo havia sido aguardado e
permitia corridas com encerramento natural, substituição de pedidos e reuso de
PID. O controle de segmentos agora registra cada proprietário com identificador
próprio e mantém o `Child` na tarefa responsável pelo processo. Pedidos
substituídos, processos antigos e shutdown recebem sinais tipados; o registro
permanece até a conclusão da tarefa proprietária. O início do processo e sua
admissão são protegidos pela mesma trava que fecha a admissão no shutdown.

O proprietário continua ativo mesmo se a requisição HTTP for descartada, lê as
duas saídas em paralelo e aguarda o filho após solicitar seu término. O servidor
aguarda os proprietários registrados antes de concluir sua limpeza, sujeito ao
mesmo orçamento global. O contador `segment_cancellations` informa quantos
processos receberam seu primeiro pedido de cancelamento no
shutdown, e não o número total de processos ativos ou comprovadamente
encerrados. Processos já sinalizados mantêm a primeira causa.

Em Unix, um término solicitado com sucesso e observado como `SIGKILL` é tratado
como cancelamento esperado, em vez de `SEGMENT_ERROR`; outras falhas
permanecem explícitas. Em Windows, o status ainda não é usado para comprovar
essa distinção e uma saída malsucedida continua sendo registrada como erro.
O log `all registered segment owners finished` significa que as tarefas
registradas terminaram, não que todas as operações de wait e leitura
necessariamente tiveram sucesso: essas falhas são registradas separadamente
e agora propagadas ao resultado `clean` do lifecycle. Uma tarefa que
falhe por panic pode manter o registro até o timeout. Um timeout ou abort do
servidor também não comprova conclusão das tarefas destacadas. Permanecem
fora dessa garantia processos ou operações anteriores à admissão e recursos
de streaming não cobertos por esse registro. Uma requisição tardia para
segmento que chega à admissão fechada ainda é registrada como `SEGMENT_ERROR`;
isso não significa que um processo novo tenha sido iniciado.

**Testes automatizados:** `cargo clippy --release --lib --tests -- -D warnings`
passou; os nove testes direcionados de streaming em release passaram no
macOS. Eles incluem cancelamento e espera de filho real, falha genuína,
requisição descartada, supersessão e recusa de admissão após shutdown.
`cargo fmt --check` e `git diff --check` passaram. O linker voltou a avisar
sobre a versão mínima de macOS do objeto `zstd`; não houve falha no teste.

**Validação real pendente:** reproduzir no macOS com HLS ativo ao fechar e
com vários seeks rápidos antes do fechamento; coletar logs completos desde o
início. Quando um processo estiver ativo e o término for solicitado, observar
`Segment process termination awaited` e, se a requisição ainda estiver
presente, `Segment request cancelled`. Observar também
`all registered segment owners finished`, erros e timeouts. A ausência dos
dois primeiros eventos não prova falha caso não haja processo ativo ou a
requisição já tenha sido descartada. Verificar os processos reais, comportamento
do player e integridade do reinício; repetir a validação em Windows e Linux.

**Duas novas execuções reais em macOS (11 de novembro):** ambas indexaram
787 arquivos inalterados, com 14 aliases verificados, zero descobertas,
remoções ou erros. Na primeira, houve 16 segmentos transcodificados e 51
servidos do cache; dois processos substituídos foram aguardados com `SIGKILL`.
No shutdown, um segmento recebeu cancelamento, teve o término aguardado com
`SIGKILL` e a requisição registrou cancelamento por `Shutdown`. Os proprietários
registrados terminaram e o servidor parou. Na segunda, houve 31 segmentos
transcodificados, cinco servidos do cache e um processo substituído aguardado;
nenhum segmento recebeu primeiro cancelamento no shutdown. Duas sessões
lineares foram limpas em 156 ms, sem falhas de processo ou diretório
reportadas, e uma espera por playlist foi cancelada.

Ambos os encerramentos reportaram `clean=true` e zero handlers ativos no
snapshot; o pool tinha 20 conexões, todas ociosas, antes do fechamento.
Os intervalos entre solicitação e conclusão do backend foram aproximadamente
130 ms e 196 ms, sem medir a apresentação visual. Não apareceram erros de
segmento ou timeouts nesses logs. Essas amostras comprovam os caminhos
observados, não os recursos não supervisionados, corpos HTTP ou todas as
plataformas. O encerramento efetivo de processos não foi verificado por
inspeção externa do sistema.

### Admissão de sessões lineares

Uma revisão posterior identificou que duas requisições podiam criar sessões
para o mesmo asset simultaneamente e substituir um `Child` sem aguardá-lo.
Também era possível criar uma sessão depois da limpeza do shutdown. A criação
agora é serializada, a admissão é fechada antes de aguardar uma criação já
admitida e o processo é iniciado e registrado sem ponto de suspensão entre
essas operações. O shutdown aguarda a criação admitida antes de drenar as
sessões; pedidos aguardando admissão são recusados após o fechamento. Um
processo iniciado tem `kill_on_drop` como proteção adicional, que não
substitui o wait. Ao substituir uma sessão já encerrada, a remoção da pasta
anterior e a verificação de estado reportam falhas explicitamente.

A serialização é global: uma criação lenta pode atrasar outras criações e o
shutdown, sujeito ao orçamento global. A síntese MIDI usa `spawn_blocking`;
se o handler for descartado durante a síntese, esse trabalho nativo pode
continuar destacado e não está coberto pela trava. Falhas de limpeza de
sessões lineares no shutdown agora degradam `clean`; falhas da limpeza
periódica anterior ao shutdown ainda não são abrangidas. Validação real dos
caminhos de erro permanece pendente.

**Testes:** os 11 testes direcionados de streaming em release passaram no
macOS, incluindo dois testes da barreira de admissão e os testes existentes de
processos de segmentos. O Clippy em release com `-D warnings` passou. Os
novos testes exercitam a sincronização da barreira, não uma sessão linear
completa com FFmpeg e requisições HTTP.

**Três execuções posteriores em macOS (11 de novembro):** com trocas rápidas,
reprodução mais prolongada e vídeos concluídos, os scans repetiram
`Discovered: 0`, `Unchanged: 787`, `Pruned: 0`, `Errors: 0` e 14 aliases
verificados. Os shutdowns processaram respectivamente 4, 5 e 1 sessões
lineares, em 318 ms, 277 ms e 8 ms, sem falhas reportadas de processo ou
diretório. Na segunda execução, seis esperas por playlist foram canceladas.
Na segunda e terceira, um segmento substituído teve sua terminação aguardada
em cada execução; nenhum segmento recebeu primeiro cancelamento no shutdown.
Não houve warnings, erros ou timeouts registrados. Os três encerramentos
reportaram `clean=true`, com 20 conexões do pool, todas ociosas, antes do
fechamento; o tempo do backend foi cerca de 385 ms, 359 ms e 92 ms,
respectivamente, sem medir a apresentação visual. A ausência de eventos de
admissão impede demonstrar, somente por esses logs, que o cenário de corrida
entre a admissão e o shutdown tenha ocorrido. Os logs não comprovam que todos
os recursos não registrados foram encerrados.

### Propagação de falhas de supervisão de streaming

O lifecycle agora aceita um registro explícito de falha de subsistema. Durante
a drenagem, o servidor informa falhas reportadas na limpeza de sessões
lineares e erros dos proprietários de segmentos; o resultado é degradado e o
fechamento explícito do pool é evitado. Falhas de supervisão de proprietários
são acumuladas durante a vida do servidor, mesmo se ocorrerem antes do pedido
de shutdown: é uma política conservadora, pois a conclusão segura do recurso
pode não ter sido comprovada. Cancelamentos tipados e erros de transcodificação
com status de saída conhecido não contam como falhas de supervisão. Um erro
retornado pelo loop do servidor também é registrado como falha de subsistema.

Essa propagação não cobre trabalho não registrado, falhas silenciosas da
limpeza periódica linear, tarefas nativas destacadas, ou abort após timeout.
`clean=true` permanece restrito ao que foi supervisionado.

**Testes:** 13 testes direcionados de streaming e quatro testes do orçamento
do lifecycle passaram em release no macOS; Clippy passou com `-D warnings`.
Os testes cobrem, separadamente, o registro de falha do proprietário, o
resultado de limpeza linear e a degradação do lifecycle quando um subsistema
reporta falha. Não há teste de integração que injete esses erros no servidor
completo. O linker emitiu novamente o aviso conhecido de versão mínima de
macOS para `zstd`.

### Protocolo de assets e fechamento do banco

**Logs de 12 de novembro:** as duas execuções indexaram 787 arquivos
inalterados, sem descobertas, remoções ou erros e com 14 aliases verificados.
Na primeira, foram registradas 67 transcodificações e 19 segmentos servidos
do cache; 23 segmentos substituídos tiveram término aguardado e suas
requisições registraram cancelamento. No shutdown, três sessões lineares
foram processadas sem falhas reportadas, sem novo cancelamento de segmento.
Na segunda, foram registradas 32 transcodificações e cinco segmentos servidos
do cache; não havia sessões lineares nem novos cancelamentos de segmento no
shutdown. Ambos os resultados registrados foram `clean=true`, com cerca de
92 ms e 88 ms de duração no backend, respectivamente.

Na primeira execução, porém, o protocolo de assets tentou consultar o banco
após o início do fechamento do pool: ocorreram 16 erros `PoolClosed`, inclusive
após o registro de conclusão do backend. Antes do fechamento, o snapshot do
pool mostrava 20 conexões ociosas; isso não impediu novas requisições. A segunda
execução não apresentou o erro e tinha 19 conexões ociosas. Os logs não
comprovam perda de dados nem que as requisições tenham iniciado antes do
fechamento; comprovam que o resultado anterior não cobria essas operações.

Para `asset://` e `thumb://`, a admissão agora é fechada atomicamente quando
o shutdown é solicitado. Requisições tardias recebem resposta 503 antes de
consultar o banco; requisições já admitidas mantêm um guard até o retorno do
handler. O coordenador aguarda sua drenagem em paralelo à drenagem de tarefas
registradas, com timeout de cinco segundos. Se a drenagem não concluir,
`clean=false` e o fechamento explícito do pool é evitado. Os logs informam a
contagem ativa ao fechar a admissão e a conclusão ou timeout da drenagem.

Essa barreira não cobre comandos IPC, protocolos `video://`/`audio://`,
corpos entregues pelo runtime após o retorno do handler, ou trabalho nativo
destacado que sobreviva ao cancelamento de um handler. A validação real deve
verificar os novos eventos e a ausência de `PoolClosed`, inclusive depois do
log de conclusão; sem requisições próximas ao fechamento a ausência isolada
de erro não demonstra a corrida. A duração visual não é medida pelo backend.

**Testes:** nove testes direcionados de protocolos e quatro de lifecycle
passaram em release no macOS, assim como o Clippy com `-D warnings`. Os
testes verificam o bloqueio de admissões tardias, a espera pelos guards
existentes e a resposta 503 do handler para os dois esquemas com Tauri mock.
A reprodução em runtime ainda requer observação de requisições ativas ou
tardias no instante do shutdown.

**Duas novas execuções reais em macOS (12 de novembro):** nas duas o scan
registrou 787 arquivos inalterados, 14 aliases verificados e nenhuma
descoberta, poda ou erro. A admissão do protocolo fechou com `active=0`, e
`Asset protocol handlers drained` precedeu o fechamento do pool. Nenhum
`PoolClosed` apareceu, inclusive após a conclusão do backend. As sessões
lineares processadas foram três e nove, sem falhas reportadas; na segunda
execução quatro esperas por playlist foram canceladas. Os resultados foram
`clean=true`, com cerca de 318 ms e 452 ms entre o pedido e a conclusão do
backend. Em ambos os casos o pool tinha 20 conexões ociosas antes do
fechamento. Não foi registrado nenhum cancelamento novo de segmento no
shutdown. A contagem zero na admissão não comprova drenagem de uma requisição
ativa nem que uma requisição tardia tenha sido recusada em runtime; o filtro
de logs utilizado também não habilitava o DEBUG do protocolo.

Na segunda execução, houve dois erros de saída vazia para o segmento 5 do
asset `d3bd415a-e418-4bd8-b42b-46934bf8c433`, durante a reprodução e antes
do shutdown. Consulta somente leitura ao banco fechado identificou o
arquivo `Audio/amr/AUD001.amr`. O `ffprobe` local informou duração estimada
de 20,02575 s e advertiu que a estimativa por bitrate pode ser imprecisa;
os últimos pacotes observados terminam em 19,4 s. A execução local de
FFmpeg com início em 20 s reproduziu saída vazia. A playlist de quatro
segundos solicita esse sexto segmento por causa da duração estimada. Trata-se
de problema de reprodução independente do shutdown; não reduzir a duração
por margem arbitrária, pois isso poderia cortar conteúdo válido. Investigar
estratégia de duração efetiva e custo de leitura para formatos estimados.

**Teste adicional:** dez testes de protocolos passaram em release e Clippy
passou com `-D warnings`. O novo teste usa Tauri mock e SQLite real com o
schema da aplicação: mantém a única conexão ocupada para garantir uma consulta
admitida em espera, fecha a admissão, verifica que a drenagem não termina
antes da liberação, confirma resposta 503 para requisição tardia, permite a
conclusão da consulta e só então fecha o pool. O teste não executa o
coordenador Tauri completo nem substitui a validação em runtime.

**Três execuções adicionais (12 de novembro):** todos os scans registraram
787 arquivos inalterados, 14 aliases verificados e nenhuma descoberta, poda
ou erro. Os três encerramentos terminaram `clean=true`, sem `PoolClosed` ou
timeout registrado, com a admissão do protocolo fechada em `active=0` e
drenada antes do fechamento do pool. Assim, a corrida com handler ativo ou
requisição tardia permanece não demonstrada em runtime.

Na primeira execução, sete sessões lineares foram limpas em 2.288 ms, sem
falhas reportadas; seis esperas por playlist foram canceladas. O backend
levou aproximadamente 2.380 ms do pedido à conclusão. Nas duas seguintes,
um segmento por execução foi cancelado por `Shutdown`, com `SIGKILL`
aguardado e cancelamento registrado pela requisição. As limpezas lineares
processaram uma e zero sessões, respectivamente; os tempos totais do backend
foram aproximadamente 89 ms e 115 ms. Em cada execução o pool tinha 20
conexões ociosas antes do fechamento. Esses tempos não medem a duração visual.

A limpeza linear é sequencial; a amostra de 2.288 ms demonstra consumo
relevante do orçamento de cinco segundos, mas o log anterior não distingue
tempo aguardando processos de tempo removendo diretórios. Foram acrescentados
ao resumo de limpeza `running_processes`, `exited_processes`,
`process_wait_ms`, `directory_cleanup_ms` e `slowest_session_ms`; os dois
tempos acumulados representam intervalos sequenciais, não tempo de CPU. Em
DEBUG, cada sessão também registra os tempos, sem identificadores ou
caminhos. Esses dados orientarão eventual paralelização, sem presumir sua
causa ou prometer redução de tempo antes de medições.

**Validação da instrumentação:** Clippy em release com `-D warnings` e os
13 testes direcionados de streaming passaram no macOS. A medição real dos
novos campos permanece pendente; os testes existentes não reproduzem uma
limpeza linear com sete sessões e diretórios grandes.

### Workers ativos e snapshot do pool (12 de novembro)

Na primeira das duas novas execuções, com pasta da aplicação e banco
reinicializados pelo usuário, o scan concluiu com 787 arquivos descobertos,
sem erros ou poda. Na seguinte, registrou 787 inalterados, 14 aliases
verificados e nenhuma descoberta ou erro. Não apareceram `PoolClosed` nem
`database is locked` nessas amostras.

No primeiro encerramento, havia duas tarefas de thumbnail em andamento, com
fases de extração de metadados e thumbnail; os diagnósticos também registraram
espera por extração nativa de cores e por lote de fingerprints. Foram
limpas 12 sessões lineares em 1.959 ms: nove processos ativos, três já
encerrados, 1.928 ms acumulados em espera de processos, 6 ms na remoção
de diretórios e 780 ms na sessão mais lenta, sem falhas reportadas. Nesta
amostra a espera sequencial dominou a limpeza, mas não demonstra que
paralelizá-la seja sempre segura ou vantajosa. Um segmento foi cancelado
por `Shutdown` e seu processo aguardado. Quatro requisições chegaram depois
do fechamento da admissão de segmentos e foram registradas como
`SEGMENT_ERROR`, embora essa recusa seja esperada no shutdown.

O backend levou aproximadamente 5.064 ms do pedido à conclusão, com
`clean=true`. O prazo compartilhado do lifecycle começou depois do pedido;
a drenagem registrada levou aproximadamente 4.901 ms a partir desse início.
O tempo total também inclui etapas fora desse prazo, inclusive o fechamento
do pool: não se trata, portanto, de um limite de cinco segundos para toda
a experiência de encerramento.

Na segunda execução, as duas tarefas de thumbnail estavam ativas no
shutdown; os workers de duplicatas e thumbnails concluíram aproximadamente
1,62 s e 2,09 s após o pedido. Quatro sessões lineares foram limpas em
242 ms, com 236 ms de espera de processos e 2 ms de remoção de diretórios.
Dois segmentos cancelados por `Shutdown` foram aguardados. No snapshot
anterior ao fechamento havia nove conexões no pool e oito ociosas;
`close` retornou em 24 ms e o resultado anterior foi `clean=true`. Isso
não identifica o proprietário nem comprova perda de dados: uma conexão
não ociosa pode também refletir atividade interna do pool.

Para evitar classificação enganosa, a recusa de segmento agora usa erro
próprio, distinto de um processo efetivamente cancelado, e é mapeada para
HTTP 503 com log informativo; falhas reais continuam produzindo HTTP 500
e `SEGMENT_ERROR`. O teste de admissão fechada usa um executável inexistente:
receber o erro tipado confirma que a tentativa foi recusada antes do spawn.
O mapeamento de ambos os tipos de falha também tem teste direcionado.

O coordenador agora aguarda até 250 ms pelo retorno de conexões ao estado
ocioso e considera o encerramento degradado, sem fechar explicitamente o
pool, se o snapshot continuar indicando conexões não ociosas. A espera
limitada contempla a devolução assíncrona do SQLx e é adicional aos demais
prazos de shutdown. Um teste
com SQLite mantém uma conexão adquirida, verifica a recusa de fechamento,
libera-a e verifica o fechamento quando o pool está ocioso. Essa é uma
verificação conservadora, não uma barreira: as contagens não são atômicas,
podem refletir atividade transitória e novas operações IPC ainda podem
adquirir conexões após o snapshot. Mesmo `clean=true` não comprova drenagem
integral de trabalho não supervisionado. A queda do processo ao sair ainda
libera recursos; omitir o fechamento explícito não garante a conclusão
de operações em andamento.

**Validação no macOS:** `cargo fmt --check`, `git diff --check` e Clippy
em release com `-D warnings` passaram; os 14 testes direcionados de
streaming e o teste de conexão adquirida do coordenador passaram em
release. A primeira versão do teste evidenciou que a devolução da conexão
ao pool é assíncrona; após a introdução da espera limitada, o teste passou.
Esses testes não substituem a validação do coordenador completo em runtime.

Eventos de metadados e thumbnail novamente foram publicados depois de o
bridge do frontend parar, gerando `No active subscribers`; a consequência
funcional e a reconciliação desses eventos continuam pendentes de análise.
As falhas de suporte de thumbnails e os avisos de fontes SVG não são
atribuídos a essas alterações.

### Quatro encerramentos adicionais e admissão HTTP (12 de novembro)

Na ordem cronológica dos arquivos de log, o usuário encerrou pela barra
de título, duas vezes por Cmd+Q e uma pelo Dock. Todos os scans registraram
787 arquivos inalterados, nenhuma descoberta, poda ou erro. Os quatro
encerramentos chegaram ao coordenador e registraram `clean=true`; os três
últimos também registraram o callback nativo de terminação do macOS.

| Entrada | Tempo entre pedido e conclusão do backend | Sessões lineares | Limpeza linear | Pool antes do fechamento |
| --- | ---: | ---: | ---: | --- |
| Barra de título | 859 ms | 2 | 110 ms | 11/11 ociosas |
| Cmd+Q | 1.312 ms | 3 | 433 ms | 19/19 ociosas após espera de 12 ms |
| Cmd+Q | 4.206 ms | 6 | 545 ms | 9/9 ociosas |
| Dock | 1.953 ms | 9 | 1.875 ms | 19/19 ociosas |

Na terceira execução, o worker de duplicatas ainda aguardava extração de
fingerprint por cerca de quatro segundos e terminou antes do prazo do
lifecycle. Na quarta, a limpeza das nove sessões teve 1.857 ms acumulados
de espera por processos e 10 ms de remoção de diretórios. Não houve falhas
reportadas nessas limpezas. Não havia tarefas de thumbnail pendentes no
instante do cancelamento. O protocolo de assets fechou com `active=0` nas
quatro amostras; a corrida com handler ativo continua sem confirmação em
runtime. Não houve `PoolClosed` ou erro de banco trancado nos logs.

As recusas esperadas de segmentos ocorreram 2, 11, 22 e 0 vezes,
respectivamente, sem `SEGMENT_ERROR` correspondente. Os logs confirmam
o caminho de classificação e os testes confirmam seu status HTTP 503; não
há registro independente do código HTTP de cada resposta em runtime.
Em três execuções, processos de segmentos efetivamente cancelados por
`Shutdown` também tiveram seu término aguardado.

A investigação das requisições tardias identificou que o futuro usado como
sinal de graceful shutdown do Axum aguardava toda a limpeza linear antes de
concluir. Assim, até o fim dessa limpeza, o servidor ainda podia aceitar
conexões e requisições novas. O sinal HTTP agora é ativado diretamente pelo
cancelamento, enquanto a limpeza dos processos ocorre em paralelo, dentro
do mesmo handle supervisionado. A conclusão do servidor continua aguardando
tanto a drenagem HTTP quanto a limpeza de processos; os limites existentes
do lifecycle continuam se aplicando. Requisições já admitidas podem ainda
alcançar as barreiras de segmentos e sessões e ser recusadas.

O novo teste com servidor TCP real mantém uma requisição ativa, cancela o
servidor, confirma que ele deixa de aceitar conexões antes de terminar essa
requisição e verifica que a resposta admitida é drenada normalmente. O
teste não executa o coordenador completo nem prova comportamento de todos
os clientes ou plataformas. As requisições repetidas, por si só, não provam
que players antigos tenham vazado; a origem desses clientes não foi
identificada. A mudança na admissão HTTP ainda precisa de validação real.

**Validação no macOS:** os 15 testes direcionados de streaming passaram em
release, inclusive o novo teste TCP; Clippy em release com `-D warnings`,
`cargo fmt --check` e `git diff --check` passaram. O linker emitiu o aviso
conhecido de objeto zstd compilado para macOS 13.1 em alvo 10.12; não houve
falha de compilação.

### Três novas execuções e supervisão do scan manual (12 de novembro)

Na ordem cronológica dos logs, os encerramentos foram pelo Dock, Cmd+Q e
botão da barra de título. Os três scans concluíram com 787 arquivos
inalterados, 14 aliases identificados e nenhuma descoberta, poda ou erro.
Todos os encerramentos registraram `clean=true`, sem `PoolClosed` ou
`database is locked`. O callback nativo do macOS apareceu nas duas
primeiras execuções, conforme esperado para Dock e Cmd+Q.

| Entrada | Pedido até conclusão do backend | Limpeza linear | Pool antes do fechamento |
| --- | ---: | --- | --- |
| Dock | 496 ms | 4 sessões, 439 ms, sem falhas | 20/20 ociosas |
| Cmd+Q | 318 ms | nenhuma sessão | 20/20 ociosas |
| Barra de título | 130 ms | 2 sessões, 105 ms, sem falhas | 6/6 ociosas |

O Cmd+Q cancelou quatro segmentos e o botão da barra de título cancelou
dois; os respectivos processos tiveram o término aguardado. Não houve
requisições de segmento registradas como recusadas após o fechamento, nem
`SEGMENT_ERROR`. Não se deve concluir apenas disso que a porta deixou de
aceitar imediatamente: os logs não registram o instante exato do fechamento
do listener nem demonstram que houve tentativa de nova conexão. O teste
TCP direcionado continua sendo a evidência direta desse comportamento.
As três execuções fecharam a admissão do protocolo de assets com `active=0`
e não tinham tarefas de thumbnail pendentes no cancelamento; esses cenários
ativos permanecem não exercitados nessas amostras.

O scan disparado pelo comando de indexação manual anteriormente era
destacado. Agora o comando admite e registra cada scan com nome único no
lifecycle antes de iniciá-lo; scans concorrentes não substituem o handle
um do outro. Se a admissão observar o cancelamento ou o registro fechado,
o comando retorna erro explícito e não inicia o scan. A sincronização da
admissão e da drenagem garante que uma tarefa aceita fique no conjunto
aguardado, mesmo se o encerramento concorrer com sua criação. Handles
concluídos permanecem no registro até o shutdown, preservando a observação
de panics, com pequeno custo por scan.

O scan admitido é aguardado até o prazo compartilhado do lifecycle, não
interrompido imediatamente ao cancelar. Se exceder o prazo, seu handle é
abortado e o encerramento degradado; isso não comprova o término do
`spawn_blocking` de caminhada nem do produtor de classificação destacado.
Assim, a supervisão é parcial e não substitui cancelamento cooperativo e
propriedade de todas as tarefas filhas. O watcher criado pelo comando e
outras operações IPC também ainda exigem uma barreira de admissão própria.

**Validação no macOS:** os seis testes direcionados do lifecycle e os cinco
testes do indexador passaram em release; Clippy em release com `-D warnings`,
`cargo fmt --check` e `git diff --check` passaram. Os novos testes verificam
que o shutdown aguarda uma tarefa dinâmica admitida, que uma tarefa tardia
não é executada e que nomes duplicados não substituem o handle existente.
Não exercitam o comando Tauri completo nem a corrida com um scan real ativo.

### Scan manual ativo e corrida na preparação (12 de novembro)

Nos dois primeiros novos logs, um scan manual já registrado estava ativo
quando o shutdown começou. No primeiro, concluiu após o pedido com 193
descobertas e sem erros; o lifecycle aguardou seu handle antes de fechar
o pool. No boot seguinte, os 193 arquivos foram encontrados inalterados.
No segundo, outro scan concluiu após o pedido com 147 descobertas e sem
erros, também aguardado antes do fechamento; no boot seguinte, os 147
arquivos foram encontrados inalterados. Isso confirma a persistência e a
drenagem do handle principal nesses dois cenários, não em casos que
ultrapassem o orçamento ou envolvam tarefas filhas não supervisionadas.
Os três encerramentos registraram `clean=true`, com cerca de 533 ms,
317 ms e 218 ms do pedido à conclusão do backend, respectivamente.

No primeiro caso, `ScanCompleted` foi publicado após o bridge de eventos
do frontend ter encerrado, gerando `No active subscribers`. O catálogo
persistiu e foi observado no boot seguinte, mas o evento não foi entregue
nessa execução. A ordem do bridge, seus consumidores e a reconciliação
permanecem pendentes de investigação; não tratar a persistência como prova
de entrega da notificação.

O terceiro log revelou outra corrida: depois de o lifecycle concluir e
enquanto o pool fechava, um comando de indexação iniciou um watcher;
o registro do watcher e a admissão do scan foram recusados, mas o comando
ainda informou que o watcher fora registrado. Esse `clean=true` não
representa uma garantia de que todo o trabalho IPC tivesse terminado.

A preparação do watcher agora faz parte da mesma tarefa do scan, admitida
atomicamente no lifecycle antes de executar qualquer preparação. O comando
aguarda a conclusão da preparação como antes, enquanto a tarefa permanece
supervisionada até o fim do scan; uma admissão recusada não executa nem a
preparação nem o scan. A falha de inicialização do watcher continua sendo
registrada e não impede o scan, preservando o comportamento anterior. O
registro fictício de handle vazio do watcher manual foi removido: não
demonstrava a conclusão do watcher nativo, cujo ciclo de vida continua
uma limitação separada. Um comando admitido antes do cancelamento pode
prosseguir durante a drenagem; um timeout ainda não garante interrupção de
código nativo ou tarefas filhas.

Essa correção cobre apenas a preparação e o scan deste comando. A operação
de adicionar a localização pode começar antes dele e outras operações IPC
continuam sem barreira abrangente; o snapshot do pool também não é atômico.

**Validação no macOS:** os dois testes direcionados da admissão de indexação
e os seis testes de lifecycle passaram em release; Clippy em release com
`-D warnings`, `cargo fmt --check` e `git diff --check` passaram. O teste
controla a preparação e o scan separadamente e verifica que o shutdown
aguarda ambos; outro verifica que preparação e scan não executam após
cancelamento. Não há teste do comando Tauri completo nem validação runtime
da nova sequência nesta amostra.

### Novas amostras e ordem do bridge de eventos (12 de novembro)

Nos três logs, o encerramento ocorreu pelo callback nativo macOS e foi
registrado como `clean=true`, com aproximadamente 164 ms, 1.068 ms e
1.334 ms entre pedido e conclusão do backend. A origem específica do
callback (Dock ou Cmd+Q) não está identificada pelo log. Não apareceram
`PoolClosed`, bloqueios de banco nem falhas de limpeza linear.

Na primeira execução, um comando de scan foi recusado após a drenagem do
lifecycle; diferentemente da amostra anterior, não há log de inicialização
do watcher pelo comando recusado. É evidência favorável para a nova ordem,
não uma prova de que todas as operações de adicionar localização estejam
protegidas: o comando foi chamado após a drenagem e o pool já fechava. Na
segunda, um scan manual de 193 arquivos terminou antes do encerramento;
duas tarefas de thumbnail ainda estavam ativas e publicaram metadados e
thumbnails depois de o bridge parar, resultando em quatro avisos de
ausência de assinantes.

Na terceira execução, o scan manual foi registrado antes da preparação do
watcher, começou antes do pedido e terminou durante a drenagem com 787
descobertas e zero erros; seu handle foi aguardado antes de fechar o pool.
Os logs mostram localizações sendo removidas e readicionadas nessas
amostras, com novos IDs de raiz; as contagens de descoberta devem ser
interpretadas nesse contexto, e não como prova de falha de persistência.
Não há boot posterior nesta amostra para comprovar persistência desse último
scan. Seus eventos de progresso e conclusão foram publicados depois do
bridge parar e não tiveram assinantes ativos.

Para preservar o atendimento dos eventos de produtores supervisionados,
o bridge agora usa token independente do cancelamento inicial. O lifecycle
aguarda primeiro os demais handles e só então sinaliza o bridge, que
processa eventos já disponíveis antes de sair. Ambas as fases compartilham
o mesmo prazo global: um bridge que não termine dentro do prazo degrada o
resultado. Se um produtor exceder o prazo ou trabalho não supervisionado
publicar posteriormente, a entrega desses eventos continua sem garantia.
O canal é limitado e pode perder mensagens por lag; a emissão Tauri ou
a entrega ao frontend também não são comprovadas apenas pela presença de
assinante. A ordenação não substitui estado recuperável nem correlação de
operações de scan.

**Validação no macOS:** oito testes do lifecycle e dois testes do bridge
passaram em release; Clippy em release com `-D warnings`,
`cargo fmt --check` e `git diff --check` passaram. Os testes cobrem a ordem
de término dos produtores, o prazo compartilhado, a fila pendente e o
registro de lag. Eles não executam o frontend nem comprovam a recepção de
eventos por uma janela real; a nova ordem ainda requer validação runtime.

### Bridge com produtores ativos e watcher no boot (12 de novembro)

Os três logs registram encerramento pelo callback nativo macOS; não é
possível distinguir Dock de Cmd+Q apenas por esse registro. Os tempos do
pedido à conclusão do backend foram aproximadamente 4.548 ms, 2.765 ms e
4.436 ms. Todos registraram `clean=true`, sem `PoolClosed`, bloqueio SQLite
ou ausência de assinantes do event bus. Os erros de thumbnail e avisos de
formatos/fontes registrados continuam sendo problemas conhecidos fora do
escopo deste encerramento.

Nos dois primeiros encerramentos, scans manuais ativos concluíram com 193
e 787 descobertas, respectivamente, após o pedido e antes do término do
bridge. Nas reinicializações seguintes, os respectivos scans registraram
193 e 787 arquivos inalterados e zero erros. As localizações foram removidas
e readicionadas durante os testes; novas descobertas após a readição não
representam, por si, falha de persistência. Havia duas tarefas de thumbnail
pendentes no cancelamento em cada execução. O bridge só registrou término
depois dos handles supervisionados, sem avisos de ausência de assinantes
ou lag. Esses logs não demonstram, por si, que cada evento foi recebido
pelo bridge ou processado pelo frontend.

Para permitir essa distinção, foi adicionado um resumo agregado de eventos
recebidos pelo bridge após o cancelamento: total, progresso, conclusão e
falha de scan, thumbnail, metadados e mensagens perdidas por lag. O resumo
não contém caminhos nem identificadores. É uma contagem de recebimento
no backend, não confirmação de emissão Tauri nem entrega ao frontend; se
o bridge for abortado antes de terminar, o resumo pode não ser registrado.

A investigação do watcher confirmou que o boot registrava handles vazios
por raiz no lifecycle, embora o watcher nativo e seus loops destacados
não fossem aguardados por esses handles. Esses registros fictícios foram
removidos para não produzir logs de conclusão enganosa. O watcher real
continua sem encerramento supervisionado: seu token por raiz não controla
os loops, e seu callback utiliza envio bloqueante. Uma correção completa
precisa preservar eventos e coordenar parada da origem nativa, debouncer
e publicador, inclusive sob filas cheias e falha de shutdown. O registro
removido não era uma barreira e sua ausência não comprova liberação dos
recursos. Outras operações IPC continuam também sem barreira abrangente.

**Validação no macOS:** três testes do bridge e oito testes do lifecycle
passaram em release; Clippy em release com `-D warnings`,
`cargo fmt --check` e `git diff --check` passaram. O teste novo verifica a
classificação das contagens; os logs reais com o novo resumo e a liberação
do watcher nativo ainda não foram validados.

### Scans grandes e prazo do bridge (12 de novembro)

No primeiro dos três logs, um scan manual de 787 arquivos terminou após
o pedido, antes do shutdown do bridge e do fechamento do pool. O resumo
registrou 205 eventos recebidos durante o encerramento, incluindo oito
progressos e uma conclusão de scan, sem lag. O encerramento levou cerca
de 2.645 ms e registrou `clean=true`. O boot seguinte reconheceu os 787
arquivos como inalterados. A contagem confirma recebimento pelo bridge,
não entrega ou processamento pelo frontend.

No segundo, um scan manual de uma localização em volume externo estava
ativo no encerramento. A caminhada encontrou 42.568 arquivos suportados
e 12 subdiretórios; o scan principal excedeu os cinco segundos do lifecycle
e foi abortado. Não foi registrado `ScanCompleted` para ele. O bridge,
cancelado somente depois de expirar o mesmo prazo, também foi abortado;
nenhum resumo de recebimento foi registrado. O resultado foi `clean=false`
e o fechamento explícito do pool foi omitido. Na reinicialização seguinte,
o cache dessa localização tinha 500 entradas; isso demonstra persistência
parcial, não conclusão da biblioteca inteira.

No terceiro, o boot scan da mesma localização ainda estava em andamento ao
encerrar e também excedeu o prazo. O worker de thumbnails, com duas tarefas
diagnosticadas na fase `publication`, igualmente excedeu o prazo. O bridge
foi abortado após o prazo compartilhado; seu resumo apareceu na corrida com
o abort, contendo seis progressos e nenhuma conclusão de scan. A fase
`publication` abrange a verificação da tentativa, a escrita do arquivo
e o commit de processamento; o diagnóstico não identifica qual dessas
operações aguardava. Não se pode inferir a causa precisa da espera nem que
os trabalhos abortados e suas tarefas filhas terminaram. O resultado foi `clean=false`
e o pool não foi fechado explicitamente. Não houve log de conclusão desse
scan nem uma nova execução para verificar a recuperação integral.

Corrigida a ausência de tempo de drenagem do bridge depois que outro handle
esgota o prazo: após cancelar o bridge, ele dispõe de pelo menos 250 ms para
processar a fila, ainda com timeout e abort. Essa tolerância pode acrescentar
até 250 ms ao prazo de cinco segundos, além das demais etapas do coordenador;
um timeout de produtor continua degradando `clean`, mesmo se o bridge
terminar. A tolerância não garante entrega, nem impede publicações posteriores
de trabalho não supervisionado. A interrupção cooperativa dos scans grandes,
a propriedade das tarefas filhas e a espera em `publication` de thumbnails
continuam pendentes; não há evidência suficiente para tratar a causa da
contenção como conhecida ou alterar o timeout arbitrariamente.
Em particular, a caminhada usa `spawn_blocking` e o produtor de classificação
é iniciado em uma tarefa separada: abortar o scan que os aguarda não prova
que esses trabalhos cessaram. O produtor agora pertence a um `JoinSet` do
scan; quando o scan é abortado, o descarte do conjunto solicita o abort do
produtor e, ao descartá-lo, de suas tarefas de classificação. Isso elimina
o destaque involuntário, mas não espera a conclusão dos aborts e não interrompe
a caminhada bloqueante nem operações nativas já iniciadas. Uma solução
cooperativa deve supervisionar esses trabalhos e lidar
com transações e lotes já comprometidos, impedindo pruning e conclusão falsa
quando houver interrupção.

**Validação:** Clippy em release com `-D warnings` e os nove testes de
`lifecycle::shutdown_budget_tests` passaram após a alteração do bridge,
incluindo o cenário em que o produtor excede o prazo e o bridge ainda consegue
drenar. Depois da alteração de cancelamento, Clippy, os sete testes de
`feature::library::indexer::tests`, os dois testes de
`feature::library::event_handler::tests`, os nove testes de
`lifecycle::shutdown_budget_tests`, `cargo fmt --check` e `git diff --check`
passaram.

### Novos logs e cancelamento cooperativo (12 de novembro)

Foram recebidos quatro logs com encerramento por Cmd+Q, botão de fechamento
da splash, titlebar e Dock, conforme a ordem informada. Os timestamps internos,
porém, colocam o arquivo `125556` às 16:17, antes dos outros três (16:20,
16:22 e 16:23); portanto não é possível associar com segurança esse arquivo
ao quarto método de encerramento.

No log mais antigo, a localização em volume externo não estava montada:
`WalkDir` retornou `No such file or directory`, e o scan registrou erro sem
pruning. O encerramento subsequente foi limpo (`clean=true`), com bridge
drenado sem lag, sete conexões ociosas e fechamento do pool concluído em
58 ms. Esse erro de acesso à localização não é evidência de corrupção nem
de remoção dos registros.

Nos outros três logs, o scan de boot da localização montada encontrou 42.568
arquivos suportados e 12 subdiretórios. O cache observado no início desses
scans foi de 11.100 entradas nos dois primeiros logs e 13.500 no terceiro;
isso é compatível com retomada de persistência parcial, mas os logs não provam
quantas entradas cada execução confirmou no banco. Os scans não publicaram
conclusão antes do encerramento. Em todos os três casos, `boot_scan` excedeu
o orçamento de cinco segundos, foi abortado e o shutdown terminou `clean=false`;
por segurança, o pool SQLite não foi fechado explicitamente. Não foram
encontrados erros SQLite de banco ocupado/trancado nesses arquivos.

O trabalho de outros recursos não foi idêntico em todas as amostras: no
primeiro dos três logs, o worker de thumbnails e o de duplicatas terminaram
depois do pedido, enquanto o scan excedeu o prazo. Nos outros dois, além do
scan, thumbnails e duplicatas também excederam o prazo. Os resumos do bridge
registraram respectivamente 13, 10 e 6 eventos `scan_progress`, sem
`scan_completed` e sem lag. Isso confirma a utilidade da margem de drenagem
de 250 ms para o bridge, mas não confirma entrega ao frontend. As linhas de
classificação continuaram durante os cinco segundos de drenagem, evidência
de que os scans não observavam o cancelamento até então.

Para corrigir esse comportamento, a indexação agora recebe o token do dono
do scan (boot, operação manual e reconciliação do watcher) e verifica
cancelamento entre etapas e itens da caminhada, durante classificação e
resolução de aliases, e entre lotes de persistência. Ao cancelar antes do
pruning, a operação retorna `Cancelled`, não publica `ScanFailed`, não inicia
pruning nem emite `ScanCompleted`; lotes já confirmados permanecem válidos e
serão reconciliados na próxima varredura. O producer continua sob `JoinSet`
proprietário do scan, para que suas tarefas de classificação também recebam
abort no drop.

O pruning só começa depois que caminhada, classificação, gravação dos lotes
e observação de identidades terminaram sem cancelamento. Se o cancelamento
chegar enquanto a fase de pruning já está em andamento, ela para entre
operações; remoções já concluídas vieram de uma caminhada completa, mas
podem deixar o pruning parcial para ser reconciliado na próxima varredura.

Essa cooperação não cancela uma chamada de filesystem que esteja bloqueada
dentro do `WalkDir`, nem uma operação de banco já iniciada; o token é observado
entre lotes e assim que o controle retorna desses limites. O orçamento do
lifecycle continua sendo um limite para aguardar handles assíncronos, não uma
garantia de limite absoluto para I/O bloqueante. A nova implementação ainda
foi validada nos três novos cenários reais de runtime descritos abaixo.

### Validação runtime após o cancelamento cooperativo (12 de novembro)

Os três logs mais recentes correspondem, na ordem informada pelo usuário, ao
fechamento pela titlebar (`144037`), Cmd+Q (`150141`) e Dock (`150329`).
Em todos, a localização externa estava montada; a caminhada encontrou 42.568
arquivos suportados e 12 subdiretórios. Os caches carregados antes do scan
foram de 18.300, 25.600 e 28.200 registros. O número crescente é compatível
com a retomada da indexação entre execuções, mas não permite afirmar que a
biblioteca inteira já foi indexada.

Em cada execução, `boot_scan` reconheceu o pedido de encerramento e parou
cooperativamente, sem exceder o prazo nem ser abortado. Do pedido de shutdown
à conclusão do processo transcorreram aproximadamente 2,135 s, 0,889 s e
1,899 s. Todos os três encerramentos terminaram `clean=true`; o pool foi
fechado após snapshot ocioso e o fechamento explícito terminou em 57 ms,
57 ms e 64 ms, respectivamente. Não houve `ScanCompleted` para o scan
interrompido, como esperado.

O bridge terminou sem lag em todos os logs. Recebeu respectivamente um,
dois e quatro eventos durante a drenagem, sem `scan_completed`. Isso é
consistente com o scan cancelado sem emitir uma conclusão falsa. O usuário
confirmou que notificações e atualizações no viewport continuam normais e
que a tela de fechamento se comporta como esperado. A observação visual do
usuário é evidência funcional do frontend; o resumo backend do bridge, por
si só, continua sem provar entrega individual de eventos.

Foi encontrada, contudo, contenção SQLite durante o processamento normal,
antes dos pedidos de encerramento: um erro `SQLITE_BUSY` no processamento de
fingerprint após uma renovação de lease que demorou cerca de 10,75 s, e um
erro `SQLITE_BUSY` em processamento de thumbnail no terceiro log. O erro do
fingerprint ocorreu aproximadamente 54 s antes do encerramento; o erro de
thumbnail, cerca de 1,17 s antes. O worker de duplicatas aplicou sua política
de retry. Os logs não identificam com certeza qual transação concorrente
reteve a escrita, portanto não atribuímos a contenção ao scan ou aos workers
sem mais evidência. Apesar dessas falhas durante a execução, os workers
supervisionados encerraram sem abort e o pool estava ocioso ao final; não há
evidência de corrupção ou de falha no fechamento do banco.

### Validação em desenvolvimento no Windows (12 de novembro)

Foram analisadas cinco execuções em `tauri dev --release`. O usuário relatou
três fechamentos pelo botão da titlebar e dois pela opção de fechar janela
na barra de tarefas. Em ordem cronológica dos timestamps internos, os logs
são `172148`, `175724`, `175939`, `180112` e `180334`. Os registros não
identificam a origem do pedido; a associação dos três primeiros à titlebar e
dos dois últimos à barra de tarefas depende de essa ser também a ordem
cronológica informada.

| Log | Pedido até conclusão do backend | Tarefas registradas | Pool: conexões ociosas / total | Bridge: eventos / lag |
| --- | ---: | ---: | ---: | ---: |
| `172148` | 26 ms | 10 | 7 / 7 | 0 / 0 |
| `175724` | 2.968 ms | 12 | 7 / 7 | 4 / 0 |
| `175939` | 907 ms | 11 | 8 / 8 | 1 / 0 |
| `180112` | 1.174 ms | 12 | 20 / 20 | 7 / 0 |
| `180334` | 2.034 ms | 11 | 20 / 20 | 5 / 0 |

Cada log contém um pedido e uma conclusão `clean=true`; as tarefas
registradas e os handlers do protocolo foram drenados, e o fechamento
explícito do pool foi concluído. Os tempos medem apenas o intervalo entre
os registros de pedido e conclusão do backend, não a duração da janela nem
o momento de saída do processo. O snapshot ocioso do pool não é uma
barreira atômica para operações não supervisionadas.

Nos logs `175939`, `180112` e `180334`, o boot scan registrou parada após
o pedido; em `175724` e `180112`, um scan em segundo plano também registrou
parada. Havia thumbnails em andamento nas quatro últimas execuções e
fingerprint em `175724`. Os resumos do bridge indicam recebimento no backend,
não entrega individual ao frontend. O usuário observou notificações,
atualizações do viewport e tela de encerramento funcionando como esperado,
sem processos remanescentes visíveis no Gerenciador de Tarefas.

Após a conclusão do backend, todos os logs registraram
`Failed to unregister class Chrome_WidgetWin_0. Error = 1412`. No Windows,
o código 1412 significa que a classe ainda possui janelas abertas. A
mensagem vem do componente gráfico baseado em Chromium; estes logs não
identificam a janela responsável nem demonstram a causa, impacto persistente
ou relação com a nova janela de encerramento. Em quatro logs também aparecem
avisos do event loop Tao antes do shutdown. Investigar a recorrência em
bundle e, se necessário, comparar com outros fluxos antes de atribuir a
origem ao Mundam ou ao runtime.

Essas amostras validam os fluxos observados em desenvolvimento, não o bundle
Windows, reprodução de mídia ativa, acessibilidade real, encerramento de
sessão do sistema ou liberação abrangente de subprocessos e recursos não
supervisionados. Não alteram o aceite pendente.

### Bloqueios ao testar o bundle Windows (12 de novembro)

No aplicativo instalado, a splash permaneceu aberta apesar do backend atingir
`BackendReady`. A inspeção da janela principal reproduziu um erro de IPC:
`get_startup_state` foi chamado antes de `StartupService` ser registrado.
O comando subsequente `fail_frontend_startup` também falhou pelo mesmo motivo,
deixando a splash sem estado de falha. Para eliminar a corrida, o estado de
startup foi movido do callback `setup` para o builder Tauri, anterior à criação
das janelas. Um teste automatizado de IPC anterior ao setup passou. Uma nova
build de produção sem instalador foi executada em uma pasta temporária com
recursos do pacote, exceto o Assimp incompatível: a janela principal ficou
visível, o snapshot chegou a `ready` sem falha e o fechamento registrou
`clean=true`. A validação em uma nova build instalada ainda é necessária.

O executável Assimp instalado e a DLL `assimp-vc143-mt.dll` correspondem aos
arquivos preparados para a build, mas o executável importa
`assimp-vc145-mt.dll`, ausente. A captura enviada confirma essa DLL ausente.
Não há evidência de que esse erro tenha causado a corrida da splash. O
instalador MSI não ter mostrado o alerta não demonstra que o Assimp funcione.

Com aprovação do usuário, foi compilado o Assimp oficial `v6.0.6`, commit
`a79ff6bbce54d3e01186a918dc6374ed55317426`, usando Visual Studio 2022,
MSVC 19.44 e Windows SDK 10.0.26100.0, em x64 Release. O CMake habilitou
bibliotecas compartilhadas, CLI e zlib incluído; testes, exemplos e instalação
foram desabilitados. A DLL e o executável foram gerados juntos com sufixo
`vc143`; não é necessário o toolset `v145` para esse par.

Os dois arquivos e a licença fornecida no código-fonte foram preparados em
`src-tauri/resources/assimp/windows-x64/`. O executável anterior e a DLL
anterior foram preservados na pasta de arquivos desta sessão. O teste
`assimp version` identificou o commit esperado, e o comando usado pelo
Mundam (`export ... -fglb2`) converteu um OBJ de teste em GLB v2 válido.
Isso não valida todos os formatos nem a instalação final. A build usa o
runtime MSVC dinâmico; a disponibilidade do redistribuível em máquinas
limpas deve ser verificada. Antes de redistribuição, revisar todos os avisos
e obrigações das dependências incorporadas.

O executável preparado é ignorado pelo Git, enquanto a DLL é rastreada. O
setup Windows ainda reconhece um executável existente apenas pela presença
do arquivo e não verifica se suas DLLs são compatíveis; não confiar nessa
verificação como prova de funcionamento em outro ambiente. Após a build de
produção sem bundle usada para validar a splash, `tauri bundle --ci` gerou
novos instaladores MSI e NSIS; suas definições geradas incluem o executável,
a DLL e a licença preparados. A instalação e os recursos extraídos ainda
precisam ser verificados. Os avisos sobre `__TAURI_BUNDLE_TYPE` persistiram.

### Validação do bundle Windows instalado (12 de novembro)

O usuário executou os cenários em sequência: fechamento pelo X com aplicativo
ocioso (`211058`), pela barra de tarefas com aplicativo ocioso (`211107`),
pelo X durante indexação (`211135`) e pelo X durante reprodução (`211219`).
Informou que indexação, thumbnails, players, splash de abertura e tela de
fechamento se comportaram como esperado. A associação dos métodos aos logs
depende da ordem informada; a origem do pedido não é registrada nos logs.

| Log | Pedido até conclusão do backend | Resultado | Evidência relevante |
| --- | ---: | --- | --- |
| `211058` | 36 ms | `clean=true` | 10 tarefas registradas drenadas; pool fechado com 5 de 5 conexões ociosas. |
| `211107` | 45 ms | `clean=true` | 10 tarefas registradas drenadas; pool fechado com 7 de 7 conexões ociosas. |
| `211135` | 70 ms | `clean=true` | Scan em segundo plano parou após o pedido; 11 tarefas drenadas; pool fechado com 5 de 5 conexões ociosas. |
| `211219` | 5.015 ms | `clean=false` | Worker de thumbnails excedeu o prazo e foi abortado; fechamento explícito do pool foi omitido. |

Os tempos vão do pedido à conclusão do backend; não medem a duração da
apresentação nem o momento da saída final do processo. O quarto log registra
que o boot scan da localização anteriormente adicionada completou antes do
shutdown, com 787 descobertas e zero erros. Ao encerrar, havia duas tarefas
de thumbnail pendentes: os diagnósticos mostraram fases de extração de
metadados e thumbnail até o timeout. O handle do worker foi abortado e
reportado como cancelado, mas isso não comprova o término de toda tarefa
filha ou operação nativa. Um fingerprint em andamento terminou antes do
encerramento do worker de duplicatas. Não há evidência suficiente para
identificar a operação específica que reteve os thumbnails nem para afirmar
corrupção de dados.

Na mesma execução, a limpeza do streaming linear reportou uma sessão, um
processo já encerrado, nenhuma falha de processo ou diretório e a parada do
servidor. Isso não comprova encerramento de um processo ativo durante o
shutdown. Antes do pedido, três tentativas de segmento registraram erro
FFmpeg com stderr vazio, imediatamente após registros de término aguardado
por `Superseded`. No Windows a implementação não confirma que o status
decorra da terminação solicitada; portanto, os logs não distinguem um
cancelamento esperado de outra falha e não estabelecem impacto perceptível
na reprodução. O relato do usuário é favorável, mas não elimina a necessidade
de investigar essas mensagens.

Os quatro bridges terminaram sem eventos durante a drenagem e sem lag.
Cada execução registrou um pedido e uma conclusão; isso não testa pedidos
simultâneos. O erro gráfico `Chrome_WidgetWin_0`, código 1412, voltou a
aparecer após a conclusão do backend nos quatro logs de stderr, inclusive
no bundle instalado. Sua origem e impacto permanecem indeterminados. Os
resultados não comprovam a ausência de subprocessos ou recursos não
supervisionados. O encerramento degradado sob carga mantém o aceite pendente.

O relato em [tauri-apps/tauri#7606](https://github.com/tauri-apps/tauri/issues/7606)
associa essa mensagem à limpeza interna do Chromium e inclui relatos de
supressão do código 1412 ao fechar ou destruir a janela antes de sair; não
estabelece uma correção universal. No Mundam, o fechamento da janela principal
é interceptado, a apresentação de encerramento oculta as janelas anteriores
e, após a drenagem e a espera pela apresentação, o coordenador chama
`handle.exit(0)`. Portanto, o ciclo de vida das janelas é uma hipótese a
testar, não uma causa comprovada. Chamar `close()` indiscriminadamente não
é equivalente a destruí-las: os pedidos de fechamento das janelas principal
e de encerramento são interceptados. Qualquer ajuste precisa preservar a
apresentação e o protocolo de saída. O aviso também ocorreu em encerramentos
limpos, de modo que não explica por si só o timeout dos thumbnails. Não há
evidência nestes testes que justifique reparar o WebView2 ou atualizar
dependências como solução presumida.

#### Acompanhamento: cancelamento dos thumbnails

O worker aguardava cada tarefa em andamento, mas não consultava o sinal de
encerramento entre as fases. Assim, uma extração que terminasse depois do
cancelamento podia iniciar outra extração ou codificação. O worker agora
verifica o sinal entre essas fases e antes da publicação; ao interromper uma
tentativa sem resultado, libera a reserva sem consumir uma nova tentativa,
procura reconciliar artefatos não publicados e mantém o trabalho pendente. Uma operação
já iniciada continua sendo aguardada, pois descartar sua future não garante a
parada do trabalho bloqueante ou do subprocesso. Erros reais continuam sendo
tratados como erros, e uma publicação já iniciada não é interrompida por
esses pontos de verificação. A verificação não é atômica com o início de cada
operação, e o timeout global ainda pode abortar a tarefa.

O diagnóstico após cancelamento inclui o ID do asset e, com debug do módulo
de thumbnails, o provider selecionado. Os logs podem conter identificadores
de assets. A mudança não altera o orçamento de cinco segundos nem garante que
uma operação nativa ou subprocesso em andamento termine nesse prazo: por
exemplo, extratores de vídeo usam FFprobe com timeout de 15 segundos. O caso
observado ainda não identifica o provider ou a operação concreta. São
necessários novos testes Windows e, se persistir o timeout, investigação do
cancelamento e da supervisão da operação específica.

Em 12 de novembro, a suíte focada do worker passou no Windows (7 testes).
O novo teste cancela durante a extração de metadados ou de thumbnail, verifica
que a fase em andamento é aguardada, que a tentativa permanece pendente sem
consumir retry e que um preview não publicado é removido. A reprodução no
aplicativo foi repetida como registrado abaixo; a validação de uma nova build
empacotada permanece pendente.

#### Repetição em desenvolvimento com biblioteca em rede (12 de novembro)

O usuário executou, nesta ordem, fechamento ocioso com biblioteca vazia,
fechamento logo após iniciar a indexação, reprodução de vídeos com diferentes
estratégias e reprodução de áudio após reiniciar. A correspondência abaixo usa
a ordem cronológica dos quatro logs e o relato; os logs não identificam o
gesto de fechamento. Os tempos medem apenas o pedido até a conclusão do backend.

| Log | Tempo | Resultado | Evidência |
| --- | ---: | --- | --- |
| `232729` | 44 ms | `clean=true` | Nenhum thumbnail pendente na drenagem; pool 5/5 ocioso e fechado. |
| `234708` | 46 ms | `clean=true` | Scan manual iniciado e encerrado cooperativamente; nenhum thumbnail pendente na drenagem; pool 6/6 ocioso e fechado. Não exercita o cancelamento de uma extração ativa. |
| `234816` | 5.013 ms | `clean=false` | Dois thumbnails em publicação ou reconciliação; worker de thumbnails, scan de inicialização, verificador de ferramentas e worker de duplicatas excederam o prazo. Pool não foi fechado explicitamente. |
| `235144` | 5.008 ms | `clean=false` | Dois thumbnails em extração de metadados; worker de thumbnails, scan de inicialização e servidor de streaming excederam o prazo. Pool não foi fechado explicitamente. |

No terceiro teste, antes do shutdown, o SQLite registrou `database is locked`
em processamento de thumbnail e fingerprint, além de renovações de lease com
duração de aproximadamente 13 a 14 segundos. Isso demonstra contenção naquela
execução, mas não identifica o detentor do lock nem comprova que ele tenha
causado o timeout posterior. A biblioteca indexada estava em um compartilhamento
de rede; isso não estabelece que o banco SQLite estivesse na rede nem prova
que o compartilhamento causou a contenção. A inspeção do código identificou
uma hipótese concreta: a criação de lote abre `BEGIN IMMEDIATE` e consulta
identidades e existência de arquivos durante a transação. Essas operações
podem aguardar a rede mantendo o lock de escrita. Os logs não demonstram que
esse lote era o detentor do lock observado.

No quarto teste, os dois thumbnails ainda ativos correspondiam ao provider
Hasselblad RAW e permaneciam em `metadata_extraction`. O provider aguarda
extração síncrona em `spawn_blocking`; a fase registrada não distingue leitura
de arquivo, FlashRaw, fallback ou EXIF. A extração não é interrompida pelos
pontos de verificação entre fases. O scan de inicialização também não terminou
no prazo; a fase específica que o reteve não está identificada.

No terceiro teste, o servidor de streaming confirmou o término dos donos de
segmento e parou; um processo foi terminado com motivo `Shutdown`, seguido
por registro de erro FFmpeg com stderr vazio. Antes do fechamento, registros
análogos ocorreram após `Stale` e `Superseded`. A implementação Windows não
confirma pelo status que a terminação foi a causa da saída; a classificação
continua pendente. No quarto teste, a limpeza linear observou uma sessão cujo
processo já tinha saído, mas uma requisição de playlist permaneceu ativa e o
servidor foi abortado. Isso não comprova a conclusão dessa requisição nem
identifica em qual etapa estava bloqueada. O handler de playlist chama um
probe que usa `std::process::Command::output()` em uma função async, sem
timeout explícito; isso é um risco concreto a investigar, mas não está
confirmado como a etapa retida nesse teste.

#### Supervisão do FFprobe do streaming

O probe das rotas de playlist e metadados foi migrado para processo assíncrono
supervisionado pelo mesmo registro usado nos segmentos, com identificador
independente para cada probe. A tarefa proprietária permanece registrada mesmo
se a requisição for descartada, lê as duas saídas em paralelo e aguarda o
processo após solicitar sua terminação. A admissão é verificada sob a trava do
registro e também rejeitada quando o token de shutdown já está cancelado.
Falhas de supervisão são reportadas ao lifecycle; se não for possível drenar
o proprietário dentro do orçamento global, o encerramento continua degradado.
O status do processo é registrado após a solicitação de término, mas não prova
que esta causou a saída em uma corrida com encerramento natural. Assim como nos
segmentos, no Windows o status malsucedido é preservado como erro, mesmo após
o pedido de cancelamento, em vez de assumir que o pedido causou a saída.

Como o registro também cancela processos antigos, probes agora estão sujeitos
ao limite existente de 30 segundos, verificado a cada 10 segundos; antes não
existia limite explícito para esse processo. O contador de log do shutdown
passa a se chamar `process_cancellations` e inclui probes e segmentos. A
verificação de existência e a leitura de MIDI foram movidas para operações
bloqueantes fora das threads assíncronas, mas não são interrompíveis: uma
leitura de rede já iniciada ainda pode atrasar a requisição. A validação de
caminhos e outras operações do servidor também não foram integralmente auditadas.
Esta alteração não comprova qual etapa reteve a playlist no quarto teste,
nem resolve os timeouts de thumbnails, scan ou a contenção SQLite. A validação
em desenvolvimento e em bundle continua pendente.

Na suíte focada `cargo test --manifest-path src-tauri\Cargo.toml --lib
delivery::streaming::`, os 17 testes passaram no Windows, incluindo filhos de
teste reais, pedido descartado, cancelamento por idade e corrida de admissão.
Um teste anterior de drenagem HTTP exigia recusa imediata de conexão e falhou:
neste Windows, uma nova conexão ao listener fechado permaneceu pendente mesmo
em reprodução independente. O teste agora limita cada tentativa e continua
rejeitando conexões bem-sucedidas durante o prazo de observação. Isso valida
o comportamento do teste no ambiente, não substitui a validação do aplicativo.

Os logs mostram que alguns assets voltaram a ser selecionados após reiniciar,
mas não comprovam que todas as tentativas ou o processamento pendente tenham
terminado. O aviso `Chrome_WidgetWin_0`, código 1412, reapareceu nos quatro
encerramentos. Não há evidência de corrupção, mas os dois resultados
`clean=false` mantêm a validação sob carga pendente.

### Continuação no Windows: reprodução e novos logs (13 de novembro)

**Fase atual:** etapa 6 (integração e aceite), com correções da etapa 3
(drenagem real) ainda abertas. A janela e o coordenador já estão implementados;
os testes anteriores no macOS estão registrados acima. Isso não significa
aceite integral da matriz desktop. No início desta continuação, o Git já
continha mudanças de supervisão do FFprobe, cancelamento entre fases dos
thumbnails, registro antecipado do estado de startup e recursos Assimp Windows.
Essas mudanças foram preservadas.

Os logs `mundam-streaming-20261113-062259.log` e
`mundam-streaming-20261113-064105.log` acrescentam as seguintes evidências:

| Log | Evidência | Limite da conclusão |
| --- | --- | --- |
| `062259` | Segmentos de vídeo foram gerados e servidos do cache. Um segmento levou 15.531 ms; houve pedidos repetidos de playlist. Servidor de streaming drenou; boot scan e thumbnails excederam o prazo; `clean=false`. | Produzir MPEG-TS não comprova reprodução pelo WebView. Os pedidos repetidos não identificam sua causa no frontend. |
| `064105` | Playlist de áudio solicitada às 09:47:00.883 UTC; FFprobe iniciou às 09:47:16.999 UTC. No fechamento, seu processo foi terminado e aguardado, e o servidor parou. Apenas boot scan excedeu o prazo; `clean=false`. | Há demora antes do início do processo, mas os logs anteriores não separam consulta SQLite, validação do caminho e verificação de existência. Não comprovam falha de decodificação do áudio. |

O segundo log também contém três erros de thumbnail com a mensagem de que
filtros simples e complexos não podem ser usados juntos. O comando do extrator
usa `-vf`; não foi demonstrado qual arquivo ou comportamento interno do FFmpeg
introduziu o grafo complexo. Esse problema permanece separado da reprodução.
Os erros de FFprobe com stderr vazio nestas amostras ocorreram depois da
solicitação de terminação no shutdown; não demonstram a causa da falha anterior
de reprodução. O aviso gráfico `Chrome_WidgetWin_0` voltou a aparecer.

#### Correções e diagnóstico de reprodução

- O elemento de vídeo deixa de vincular a URL HLS diretamente a `src`;
  o gerenciador mantém a propriedade desse atributo para native HLS ou
  MediaSource. O binding anterior podia competir com o endereço MediaSource
  mantido pelo hls.js ao trocar a fonte. Recolocar apenas o binding antigo
  também passou no teste de troca de fonte: a alteração centraliza a
  propriedade de `src`, mas não reproduz isoladamente a falha relatada.
- O retry genérico de vídeo deixa de chamar `video.load()` para fontes HLS;
  essa chamada reinicializava o elemento durante a recuperação do hls.js.
  Recolocar o retry anterior faz a regressão falhar porque `load()` é chamado
  após um erro simulado; a correção foi restaurada depois dessa verificação.
- Ao anexar outra fonte, o gerenciador destrói a instância anterior, remove
  seus listeners e registra callbacks antes de anexar o novo elemento.
- Eventos de erro do hls.js passam pelo bridge de telemetria existente até
  o log Rust: tipo, detalhe, fatalidade, status HTTP, número de segmento e
  estado do elemento. Não são serializados URLs, objetos de loader ou
  mensagens brutas de erro que possam conter o token da sessão. Manifesto
  carregado e seleção de native HLS/MediaSource são registrados em `info`;
  eventos de carregamento, espera e reprodução ficam em `debug`.
- A canonicalização das raízes autorizadas passa para `spawn_blocking`, como
  já ocorria para o arquivo, preservando as comparações de escopo. I/O de rede
  em andamento continua não interrompível. Logs de tempo separam a resolução
  do arquivo do restante da validação de escopo.
- O FFprobe usa nível `error` em vez de `quiet` e registra tempo e status ao
  concluir, para evitar falhas naturais sem diagnóstico em stderr.

A regressão de frontend cobre troca de assets HLS, retorno à fonte nativa e
erro do elemento sem reinicialização por `load()`. Testes do gerenciador
cobrem substituição da instância, descarte dos listeners e diagnóstico sem
token. São testes com MediaSource simulado; a reprodução real no Windows
continua pendente. Essas alterações não comprovam nem resolvem por si só a
falha de áudio, o timeout do scan, os extratores de thumbnail ou a contenção
SQLite.

#### Próxima validação manual

Executar separadamente `video-local`, `audio-local` e `rede`. Nos dois
primeiros cenários, usar cópias locais de um vídeo WMV/MKV e de um áudio
FLAC/AIFC que apresentaram falha. No terceiro, usar os originais em rede.
Abrir um arquivo por vez, clicar Play, aguardar até 60 segundos, registrar
se houve imagem, som, avanço do contador ou mensagem de erro e testar um
seek se reproduzir. Em seguida fechar pelo X e aguardar o prompt. No cenário
de rede, testar primeiro o vídeo e depois o áudio antes de fechar.

Antes de **cada execução**, ajustar `$cenario` e rodar no PowerShell:

```powershell
Set-Location -LiteralPath 'C:\Users\Marcus Maia\Desktop\Aplicativos\Mundam-v2'
$cenario = 'video-local' # depois: audio-local e rede
$env:RUST_LOG = 'info,mundam_lib::delivery::streaming=debug,mundam_lib::delivery::tauri::commands::mutations=debug,mundam_lib::processing::workers::thumbnails=debug,mundam_lib::bootstrap::shutdown=debug,mundam_lib::lifecycle=debug,mundam_lib::bootstrap::system=debug'
$desktop = [Environment]::GetFolderPath('Desktop')
$log = Join-Path $desktop ("mundam-streaming-{0}-{1}.log" -f $cenario, (Get-Date -Format 'yyyyMMdd-HHmmss'))
Write-Host "Log: $log"
npm run tauri dev -- --release 2>&1 | Tee-Object -FilePath $log

# Executado depois que o aplicativo fechar e o comando acima terminar.
Get-Process -Name mundam,ffmpeg,ffprobe -ErrorAction SilentlyContinue |
    Select-Object Id, ProcessName, StartTime, Path |
    Out-String | Tee-Object -FilePath $log -Append
Write-Host "Log final: $log"
```

A listagem é somente observacional: um processo FFmpeg de outro aplicativo
não é evidência de vazamento do Mundam. Se o prompt não voltar após 30 segundos
do fechamento, registrar esse fato e preservar o log parcial; não iniciar outra
instância sobre a execução pendente. Enviar os três logs com o resultado por
arquivo e qualquer processo remanescente. Os logs existentes do backend podem
conter caminhos de arquivos, mesmo que a telemetria nova do player não inclua
URLs de sessão.

#### Verificação automatizada desta continuação

- `cargo test --manifest-path src-tauri/Cargo.toml --lib delivery::streaming::`:
  17 testes aprovados no Windows.
- `npm test -- src/components/ui/VideoPlayer/playback.spec.tsx src/lib/hls-manager.spec.ts src/lib/stream-utils.spec.ts`:
  9 testes aprovados na versão final, incluindo retorno de fonte nativa para HLS.
- `npm run typecheck`: aprovado.
- `npm run build`: aprovado na versão final; 2.753 módulos transformados.
- ESLint dos cinco arquivos TypeScript/TSX alterados: aprovado, sem warnings.
- Prettier dos cinco arquivos TypeScript/TSX alterados: aprovado.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` e `git diff --check`:
  aprovados após os ajustes de formatação.

O teste que falhou ao recolocar intencionalmente o retry antigo foi uma
verificação da regressão, não uma falha restante da versão final. Nenhum
teste automatizado substitui as três execuções do aplicativo descritas acima.

### Quatro cenários de rede e disco local: HLS e apresentação (13 de novembro)

Os quatro arquivos recebidos têm o mesmo prefixo `mundam-streaming-video-local`,
mas o cenário abaixo segue o relato do usuário e os caminhos nos logs, não o
rótulo do arquivo. O compartilhamento está em um MacBook conectado por Wi-Fi.

| Sufixo do log | Cenário relatado | Resultado do backend |
| --- | --- | --- |
| `120851` | Adição de pasta em rede, processamento e reprodução de vídeos. | Scan: 787 descobertos em 92,61 s. Shutdown: 5.018 ms, `clean=false`; thumbnails e duplicatas excederam o prazo. |
| `121941` | Áudios em rede; alguns tocaram após nova tentativa. | Shutdown: 5.005 ms, `clean=false`; thumbnails excederam o prazo. |
| `122433` | Banco/pastas da aplicação recriados pelo usuário; biblioteca local e vídeos. | Scan: 783 descobertos em 2,01 s. Shutdown: 1.826 ms, `clean=true`; pool 6/6 ocioso e fechado. HLS continuou sem reproduzir segundo o relato. |
| `123117` | Áudios locais reproduziram; mídia continuou audível durante o fechamento. | Scan de boot: 783 inalterados em 1,55 s. Shutdown: 2.643 ms, `clean=true`; pool 15/15 ocioso e fechado. |

Esses números são amostras distintas, não um benchmark controlado. O resultado
`clean=true` comprova as verificações registradas do backend; não comprova que
a janela principal foi ocultada nem que a reprodução parou imediatamente.
O aviso `Chrome_WidgetWin_0`, código 1412, reapareceu inclusive nas saídas limpas.

#### Evidência concreta no fluxo HLS

O diagnóstico dos áudios registra `Attaching native HLS` no Windows. O player
de vídeo também tinha um atalho que escrevia a playlist diretamente em `src`
quando `canPlayType` indicava HLS nativo, sem passar pelo gerenciador e sem
seus novos logs. Portanto, a hipótese anterior de que este Windows estava
sempre usando MediaSource não se confirmou.

A inspeção read-only com o FFprobe empacotado encontrou timestamps reiniciados
em segmentos consecutivos já existentes no cache:

- Vídeo H.264, segmentos 0 e 1 da mesma variante: ambos com `start_time=1.463044`.
- Áudio AAC, segmentos 0, 1 e 2 da mesma variante: todos com `start_time=1.400000`.

Cada segmento é produzido por uma execução independente do FFmpeg, que também
reinicia seus continuity counters. A playlist não sinalizava essas quebras.
A [RFC 8216, seção 3](https://datatracker.ietf.org/doc/html/rfc8216#section-3)
exige continuidade ou sinalização explícita da descontinuidade; quebras não
sinalizadas podem provocar erros de reprodução. Isso confirma uma
inconsistência do stream, mas a recuperação no WebView ainda requer teste.

Foi acrescentado `EXT-X-DISCONTINUITY` entre os segmentos independentes.
A correção usa o formato real existente e vale também para segmentos já
armazenados; não exige apagar banco ou cache. No Windows, o gerenciador passa
a preferir hls.js/MediaSource quando disponível, mantendo HLS nativo como
fallback. No macOS, a preferência anterior por HLS nativo é preservada. O hook
de vídeo agora delega a escolha ao mesmo gerenciador usado pelo áudio.

Pedidos simultâneos do mesmo segmento ainda podem gerar `Superseded` e erro
temporário na primeira requisição: isso apareceu nos logs locais e de rede.
A deduplicação das requisições em andamento permanece pendente; não se deve
interpretar essas terminações solicitadas como falha espontânea do codec.

#### Probe, acesso em rede e preview

Na primeira execução, a validação de escopo levou até 57.821 ms em um pedido
de segmento linear. Probes também chegaram ao limite de processos antigos;
um MP4 terminou com erro após aproximadamente 39 s no probe. O player podia
reproduzir a URL nativa mesmo quando essa análise falhava, mas o renderer e
o inspector mantinham o preview bloqueado enquanto aguardavam o probe. Agora
uma URL disponível permite montar o player imediatamente, e uma falha de
metadados não exibe erro de reprodução sobre essa fonte. A análise continua
em segundo plano e pode ajustar a estratégia de codec depois de concluir.

A diferença entre disco local e compartilhamento é consistente com espera
de I/O, mas não determina se a causa está em SMB, Wi-Fi, disco remoto, consultas
ou processamento concorrente. Não houve `database is locked` nessas quatro
amostras. A ausência desse erro não encerra a investigação de contenção.

#### Mídia e janelas no encerramento

O backend agora emite `shutdown:requested` à janela principal quando aceita
o pedido. O frontend pausa e silencia áudio/vídeo, desativa autoplay e pausa
novos eventos de reprodução durante a drenagem. A limpeza do backend continua
independente da entrega desse evento.

O adaptador que confirma a apresentação retornava antes de ocultar a janela
principal quando o handshake chegava depois dos 1.500 ms. Agora uma confirmação
tardia, durante um shutdown ainda não autorizado a sair, pode ocultar as
janelas anteriores sem reiniciar ou estender o prazo mínimo de apresentação.
O log registra se a confirmação foi pontual. Esse caminho explica uma
possibilidade concreta no código; os logs recebidos não comprovam que foi
o caminho ocorrido na quarta execução.

A chamada de conversão Assimp não configurava `CREATE_NO_WINDOW`; foi
acrescentada a mesma flag usada nos demais subprocessos Windows. Isso trata
uma fonte possível do prompt piscando, ainda sujeita à validação visual.
As falhas de extração HEIC/Blender/Assimp permanecem em investigação.

#### Verificação e próxima execução

Os sete arquivos de teste focados do frontend passaram: 19 testes, incluindo
seleção de engine por plataforma, passagem pelo gerenciador mesmo com suporte
nativo anunciado, preview com probe lento/falho, pausa de mídia e descarte dos
listeners. Typecheck, ESLint dos arquivos alterados e build Vite foram aprovados.
Os dois testes novos da playlist também passaram em compilação isolada com
`rustc --test src-tauri/src/delivery/streaming/playlist.rs`.

Na compilação completa do aplicativo, `cargo test --manifest-path
src-tauri/Cargo.toml --lib delivery::streaming::` passou com 19 testes, e
`cargo test --manifest-path src-tauri/Cargo.toml --lib shutdown` passou com
23 testes. As suítes têm testes em comum; as contagens não representam 42
testes distintos. Nenhum teste foi ignorado nessas duas execuções.

Para validar no aplicativo, usar a biblioteca atual e repetir o bloco PowerShell
da seção anterior com `$cenario = 'video-local-v2'`, depois `'audio-local-v2'`
e, se os dois locais funcionarem, `'rede-v2'`.
No primeiro cenário local, abrir WMV ou MKV com duração suficiente para atravessar ao
menos três segmentos; tocar por 12 segundos ou mais, fazer seek e trocar
qualidade. Conferir também MP4 nativo e um vídeo linear; voltar ao HLS e fechar
durante a reprodução. No segundo cenário, tocar áudio e fechar pelo X:
verificar pausa imediata, substituição da janela principal pela apresentação
e retorno ao prompt. Repetir vídeo e áudio em rede depois de validar
localmente. Enviar os logs e resultado por arquivo/estratégia.
A correção do protocolo e os testes simulados ainda não equivalem ao aceite
de reprodução no WebView real.

### Validação v2 e delimitação do escopo de fechamento (13 de novembro)

O usuário confirmou a reprodução de vídeos locais por estratégias HLS, linear
e nativa, além de áudio com waveform e seek. Em seguida repetiu os cenários
com o compartilhamento do MacBook conectado por cabo. A estratégia de
reprodução é determinada automaticamente pelo provider do formato; os testes
consistem em abrir arquivos representativos, não em selecionar manualmente
um modo. A escolha de engine para executar HLS não cria essa escolha no produto.

**Delimitação atual, conforme orientação do usuário:** suporte a formatos,
falhas conhecidas de thumbnails, MP2 e desempenho da rede ficam fora desta
implementação. Só voltam a este escopo quando houver evidência de erro que
afete a finalização, drenagem, integridade ou propriedade dos recursos.
Prompts durante reprodução/extração também não serão investigados aqui sem
essa relação. Essa delimitação prevalece sobre encaminhamentos anteriores
de investigação de suporte e desempenho neste relatório; os registros
históricos permanecem como evidência, não como trabalho atual autorizado.

Os três logs registram execução em `tauri dev --release`:

| Sufixo do log | Cenário informado | Pedido até conclusão do backend | Resultado e pool SQLite |
| --- | --- | ---: | --- |
| `134821` | Vídeos locais; fechamento com MPG linear tocando. | 1.818 ms | `clean=true`; 19/19 conexões ociosas; pool fechado em 117 ms. |
| `141611` | Áudios locais; fechamento com WMA tocando. | 90 ms | `clean=true`; 17/17 conexões ociosas; pool fechado em 79 ms. |
| `144512` | Biblioteca em rede por cabo, após recriação pelo usuário. | 2.116 ms | `clean=true`; 7/7 conexões ociosas; pool fechado em 61 ms. |

Em todas as execuções, as tarefas registradas terminaram sem timeout/abort,
o servidor HTTP parou, os proprietários de subprocessos registrados terminaram
e os handlers do protocolo de assets drenaram. A limpeza linear reportou zero
falhas de processo e de diretório; nas duas sessões encontradas nos testes
de vídeo, o processo linear já havia saído. Na execução em rede houve uma
solicitação de cancelamento de processo de segmento no shutdown, seguida da
espera por sua saída e da conclusão do proprietário. Esses registros não
garantem a ausência de todo processo externo não inventariado.

A telemetria de pausa de mídia chegou 4,8 ms, 3,3 ms e 5,5 ms após os pedidos,
respectivamente, registrando dois, um e dois elementos de mídia. É evidência
da execução da ação no frontend; não uma medição acústica independente.

#### Visibilidade da janela e avisos do runtime

No teste de áudio, a conclusão do backend ocorreu aos 90 ms e a confirmação
da apresentação aos 409 ms. No teste em rede, a confirmação ocorreu aos
1.345 ms, antes da conclusão do backend aos 2.116 ms. Ambos registraram
`hiding previous windows`, com `timely=true` e sem erro de `hide()`. O teste
`134821` não contém esse registro de confirmação. O log de pedido para
ocultar e o retorno sem erro não comprovam a visibilidade percebida pelo
usuário. A rapidez do fechamento pode reduzir a percepção, mas não foi
demonstrada como causa única do comportamento relatado.

Conforme orientação do usuário, a permanência breve da principal durante
uma saída normal, sem travamento e sem prolongar a reprodução, fica
registrada como observação não bloqueante. Não será acrescentado atraso
artificial para tornar sua ocultação perceptível.

Os avisos `NewEvents emitted without explicit RedrawEventsCleared` e
`RedrawEventsCleared emitted without explicit MainEventsCleared` também
apareceram durante o uso normal, antes dos pedidos de fechamento. O usuário
os correlacionou com retirar/restaurar foco. Foi conferida a implementação
instalada do Tao 0.34.5 em `src/platform_impl/windows/event_loop/runner.rs`:
esses ramos detectam transições fora da sequência esperada, emitem o aviso
e despacham os eventos intermediários. Não são, por si, um erro de worker,
FFmpeg ou fechamento do SQLite. Essas amostras terminaram limpas apesar dos
avisos; sua causa concreta ao alternar foco e eventual impacto visual não
foram determinados. Não haverá atualização de dependências nem supressão
dos avisos como parte desta validação sem evidência de impacto no fechamento.

O aviso `Chrome_WidgetWin_0`, código 1412, também permanece nos três finais.
Não há evidência nestes logs que o vincule a falha de drenagem ou corrupção.

**Resultado desta rodada:** validação funcional de encerramento em
desenvolvimento Windows positiva nos três cenários, inclusive com biblioteca
em rede. Isso não fecha a matriz de bundle, inicialização parcial, acessibilidade
ou plataformas restantes. Nenhuma alteração de código foi necessária nesta
rodada; foi atualizada a documentação. Não foram solicitados novos testes de
suporte a formatos, thumbnails ou desempenho de rede.

### Desvios, limitações e riscos residuais

| Item | Impacto | Mitigação / encaminhamento | Estado |
| --- | --- | --- | --- |
| Admissão de operações IPC durante shutdown não é barrada. | Comandos já em voo podem usar o banco até o fechamento do pool. | Registro de tarefas já é fechado atomicamente; barreira para IPC pendente. | Parcial. |
| Scan iniciado ao adicionar pasta contém I/O de filesystem bloqueante. | Uma chamada nativa já bloqueada pode não observar o token imediatamente; o prazo do lifecycle não é limite absoluto para I/O bloqueante. | Scans manuais, de boot e reconciliação recebem token; producer é supervisionado por `JoinSet`; cancelamento evita conclusão falsa e pruning prematuro. Três encerramentos reais pararam o boot scan cooperativamente e terminaram `clean=true`. | Comportamento validado em macOS; I/O bloqueante continua como limite residual. |
| Contenção SQLite entre indexação e workers de processamento. | Os logs mais recentes registram um erro `SQLITE_BUSY` em fingerprint e outro em thumbnail; o retry reduz perda permanente, mas uma tentativa pode falhar e renovar lease pode atrasar. | `BatchCreate`, criação de pastas e observação de identidades usam transações imediatas; fingerprints têm retry limitado. Determinar qual escrita concorrente retém o lock e reproduzir sob carga antes de alterar o protocolo de transação. | Parcial; investigar concorrência em runtime. |
| Eventos de indexação sem identidade de operação e listeners iniciados após o backend. | Scans simultâneos podem interferir; eventos de boot podem ser perdidos. | Definir estado recuperável e correlacionar os ciclos de scan antes de declarar cobertura completa. | Pendente. |
| Servidor de streaming excedeu o prazo após iniciar drenagem HTTP. | Um encerramento em release foi degradado mesmo com workers concluídos. | Esperas por playlist e segmento lineares são canceláveis e há diagnóstico agregado de handlers; investigar também corpos e conexões, testar reprodução ativa e não presumir a causa. | Parcial; validação runtime pendente. |
| Eventos de scan e thumbnails foram publicados depois que o bridge parou. | Eventos de atualização não foram entregues nessas amostras; efeito funcional não determinado. | Bridge permanece ativo durante drenagem dos produtores registrados e consome a fila disponível; emissão, lag, produtores não supervisionados e reconciliação durável ainda exigem validação. | Parcial; pendente. |
| Fechamento do pool retornou enquanto uma conexão permanecia adquirida em teste no macOS. | O retorno de `close` pode não comprovar a conclusão de conexões em uso; operações IPC ou tarefas não rastreadas podem persistir. | Snapshot não ocioso agora degrada o resultado e impede fechamento explícito; não é barreira atômica. Investigar divergência com o contrato SQLx e supervisionar operações; teste de regressão original permanece ignorado. | Parcial; pendente. |
| Watcher e subprocessos de extração/waveform/probe/HLS não foram auditados integralmente. | Token cancelado ou handle concluído não prova liberação dos recursos. | Proprietários de segmentos são sinalizados e aguardados pelo servidor; falhas reportadas de supervisão e da limpeza linear no shutdown degradam `clean`. Inventariar os demais recursos e validar em runtime. | Parcial; pendente. |
| Workers de duplicatas, thumbnails e cores excederam o prazo em diferentes execuções; tarefas filhas podem ultrapassar o abort do handle principal. | `clean=false`; trabalho de lease pode persistir e acessar recursos durante a saída. | Manter pool SQLite aberto se a drenagem falhar; investigar cancelamento e propriedade das tarefas filhas e repetir encerramento com processamento ativo. | Requer investigação e validação runtime. |
| Abort não interrompe necessariamente código nativo. | Encerramento degradado é possível. | Orçamento global de 5 s com drenagem concorrente implementado; política por recurso depende do inventário. | Parcial. |
| Validação no macOS e Windows (dev e bundle); sem teste de logout/desligamento do SO ou leitores de tela reais. | Acessibilidade e suporte completo Windows/Linux não comprovados; uma execução Windows em bundle terminou degradada. | Investigar o worker de thumbnails e executar a matriz restante, inclusive Linux empacotado. | Parcial; pendente. |
| Erro gráfico Windows `Chrome_WidgetWin_0`, código 1412, após encerramentos em dev e bundle. | A classe ainda possui janelas abertas no momento da tentativa; causa e impacto não identificados. | Comparar outros fluxos; não inferir processo remanescente ou relação causal com o shutdown. | Pendente de investigação. |
| Se a janela não for criada, só o título da janela principal muda para "Mundam — Closing…". | Feedback mínimo, sem progresso detalhado. | Fallback básico implementado; não testado com falha real de criação. | Parcial. |
| Callback de encerramento nativo no macOS depende da estrutura do delegate Tao. | Alterações futuras no runtime podem introduzir implementação própria do seletor. | Falhar explicitamente na inicialização em vez de substituir a implementação; verificar em atualizações do Tauri/Tao e repetir testes de Dock e Cmd+Q. | Teste Objective-C aprovado; fluxos básicos validados em desenvolvimento e no bundle macOS. |

### Aceite

**Estado:** pendente.  
**Responsável, data e observações:** a preencher após revisão.
