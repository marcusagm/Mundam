# Levantamento e plano: janela de finalização do Mundam

**Data:** 9 de novembro de 2026  
**Estado:** plano aprovado; primeira etapa de implementação em andamento  
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

- [ ] Confirmar decisões de UX, tempos e cobertura de saída com o responsável pelo produto.
- [ ] Inventariar entradas reais de saída por plataforma e classificar as não interceptáveis.
- [ ] Inventariar recursos, proprietários, dependências, operações críticas e política de timeout/recuperação.
- [ ] Registrar baseline de tempo e consumo nas plataformas disponíveis.
- [ ] Implementar coordenador único, estado revisionado e saída final sem recursão.
- [ ] Garantir fechamento atômico da admissão e supervisão do boot, incluindo criação/registro concorrentes.
- [ ] Garantir drenagem verificável dos recursos no escopo, watchers e tarefas filhas.
- [ ] Auditar e tratar servidor, requisições e processos filhos sob propriedade do Mundam.
- [ ] Preservar transações, journal, leases, originais e recuperação após reinício.
- [ ] Implementar janela independente, entrada Vite e empacotamento.
- [ ] Criar capability mínima e validar autorização de IPC por janela.
- [ ] Implementar protocolo evento + consulta, reconciliação e descarte dos listeners.
- [ ] Implementar apresentação mínima, limite de handshake, fallback e mensagens factuais.
- [ ] Validar acessibilidade, teclado, leitores de tela, contraste, temas e movimento reduzido.
- [ ] Cobrir falhas, timeouts, fechamento repetido, saída do SO e inicialização parcial.
- [ ] Executar testes automatizados, typecheck, lint e build pertinentes.
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
| Tarefas registradas | Token raiz cancelado; `shutdown_all` aguarda handles. | Resultado do handle; panic/timeout marcam resultado degradado. | Timeout existente de 5 s **por tarefa**, não global; abort não prova término de trabalho nativo. | Parcial; política definitiva e cobertura pendentes. |
| Watchers, operações IPC, servidor, subprocessos e tarefas não registradas | Ainda utilizam mecanismos existentes. | Não estabelecida de forma abrangente nesta etapa. | Limitações e recovery existentes precisam ser auditados. | Pendente; não inferir limpeza integral. |

### Alterações realizadas

| Data | Arquivos / área | Alteração e justificativa | Referência |
| --- | --- | --- | --- |
| 2026-11-09 | `src-tauri/src/feature/shutdown.rs`, `bootstrap/shutdown.rs`, `delivery/tauri/shutdown.rs`, `lib.rs` | Estado revisionado, pedido único, eventos/consulta, confirmação e janela dedicada; saída interceptada quando o Tauri permite. | Fluxos básicos testados pelo usuário no macOS em modo de desenvolvimento; demais cenários pendentes. |
| 2026-11-09 | `bootstrap/startup.rs`, `delivery/tauri/startup.rs`, `lifecycle.rs` | Aguardar supervisor do boot; impedir apresentação da principal e avanços de boot após pedido; registrar tarefas com falha/timeout como degradadas. | Ainda não fecha atomicamente toda admissão de trabalho. |
| 2026-11-09 | `shutdown.html`, `src/core/shutdown/`, `src/components/features/shutdown/`, capability/permissão próprias, `vite.config.ts` | Apresentação independente e indeterminada, com tema e redução de movimento compartilhados com a splash. | Janela observada no macOS em modo de desenvolvimento; acessibilidade real e outros ambientes pendentes. |
| 2026-11-09 | `bootstrap/macos_shutdown.rs`, `bootstrap/shutdown.rs`, `lib.rs` | Encaminhar `applicationShouldTerminate:` do AppKit ao coordenador compartilhado. O delegate de Tao é preservado por subclasse sem ivars; se já houver implementação do seletor, a instalação falha explicitamente em vez de sobrescrevê-la. A saída autorizada retorna `NSTerminateNow`. | Dock e Cmd+Q validados pelo usuário no macOS em modo de desenvolvimento. |

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
| Pendente | Matriz desktop | macOS, Windows e Linux | Registrar resultado efetivo antes do aceite. |

Na amostra fornecida, o intervalo entre o log do pedido e o log de conclusão
da drenagem registrada foi de aproximadamente 3,5 ms pela titlebar, 78,8 ms
pelo Dock e 3,3 ms por Cmd+Q. Esses tempos **não medem** duração da janela,
apresentação, término do processo ou desempenho sob carga. Não houve mensagens
de erro, timeout ou aviso na saída compartilhada. O `clean=true` refere-se aos
handles registrados, não comprova a finalização dos subprocessos, watchers
nativos ou operações não rastreadas. A versão do macOS e as condições exatas
de carga não foram informadas; o build empacotado no macOS foi validado apenas por relato, sem logs; Windows e Linux continuam pendentes.

### Desvios, limitações e riscos residuais

| Item | Impacto | Mitigação / encaminhamento | Estado |
| --- | --- | --- | --- |
| Admissão e registro ainda não são atômicos durante shutdown. | Novas operações ou recursos podem surgir durante a drenagem; a janela não garante que todo trabalho cessou. | Implementar barreira por proprietário, cobrir corridas e operações IPC antes de marcar entrega completa. | Pendente. |
| Watcher, subprocessos e tarefas não rastreadas não foram auditados integralmente. | Token cancelado ou handle concluído não prova liberação dos recursos. | Inventariar propriedade, drenar e testar explicitamente. | Pendente. |
| Timeout continua sendo por tarefa e abort não interrompe necessariamente código nativo. | Espera cumulativa e encerramento degradado são possíveis. | Definir política segura por recurso e orçamento global após inventário. | Pendente. |
| Validação limitada ao macOS (dev e bundle); sem teste de logout/desligamento do SO ou leitores de tela reais. | Acessibilidade e suporte Windows/Linux não comprovados. | Executar matriz em builds empacotados nesses sistemas. | Parcial; pendente. |
| Se a janela não for criada, a janela anterior permanece visível sem overlay próprio. | O fallback pode não transmitir o progresso. | Implementar e testar feedback alternativo independente da janela dedicada. | Pendente. |
| Callback de encerramento nativo no macOS depende da estrutura do delegate Tao. | Alterações futuras no runtime podem introduzir implementação própria do seletor. | Falhar explicitamente na inicialização em vez de substituir a implementação; verificar em atualizações do Tauri/Tao e repetir testes de Dock e Cmd+Q. | Teste Objective-C aprovado; fluxos básicos validados em desenvolvimento e no bundle macOS. |

### Aceite

**Estado:** pendente.  
**Responsável, data e observações:** a preencher após revisão.
