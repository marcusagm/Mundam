# Levantamento e plano: janela de finalização do Mundam

**Data:** 9 de novembro de 2026  
**Última revisão:** 19 de novembro de 2026
**Estado:** sessão Linux concluída e aceita pelo usuário em desenvolvimento/release, binário direto, Debian e AppImage; pendências gerais restantes consolidadas na seção 14
**Escopo:** aplicações desktop Tauri em macOS, Windows e Linux  
**Destino:** `docs/report/`, conforme solicitado; este documento combina levantamento e planejamento técnico.

**Leitura atual (19 de novembro):** as seções 1, 9, 10, 12, 14 e 23
consolidam o estado vigente. As seções 4 a 8 preservam o levantamento/proposta
originais; a seção 13 mantém a evolução macOS/Windows e as seções 15 a 22
mantêm as rodadas Linux, inclusive resultados intermediários depois superados.
Afirmações sobre ausência de teste em uma rodada não anulam validações posteriores.
Somente a seção 14 enumera pendências atuais; observações históricas não criam
novas obrigações. Testes de acessibilidade permanecem dispensados.

**Decisão de encerramento Linux:** por orientação expressa do usuário em
18 de novembro, a sessão está concluída. Ele informou testes adicionais fora
desta coleta e considera os problemas intermitentes relacionados ao computador
ou à instalação do Ubuntu. Sem causa demonstrada no Mundam, essas ocorrências
ficam como notas de investigação futura, sem pendência de implementação,
reteste obrigatório ou bloqueio de aceite Linux. Os logs são preservados sem
converter essa decisão de escopo em diagnóstico técnico comprovado de hardware.

## 1. Resumo executivo

A janela de finalização independente foi implementada, integrada ao coordenador
único e validada nos cenários registrados em macOS, Windows e Linux. A limpeza
começa sem depender do frontend; a apresentação usa estado revisionado, evento
e consulta, mínimo de 600 ms após confirmação e janela de confirmação limitada
a 1.500 ms. Há pausa de mídia, fallback e saída autônoma perante indisponibilidade
do IPC ou da montagem da apresentação nos casos testados.

A sessão Linux de 17–18 de novembro está **concluída e aceita pelo usuário**:
desenvolvimento com backend release, binário direto, pacote Debian instalado
e AppImage foram exercitados. Houve indexação de biblioteca nova, thumbnails,
cores e fingerprints, reprodução nativa/HLS/linear, waveform, seek, velocidade,
X, Alt+F4, splash, pedidos repetidos, principal minimizada e falhas injetadas
na apresentação/IPC, conforme a cobertura de cada rodada. Não se afirma que
cada cenário foi repetido em cada formato de pacote.

Foram corrigidos o transporte HTTP autenticado de mídia nativa Linux e sua
classificação HLS, a resiliência do cliente de shutdown e a supervisão das
rotinas de manutenção do banco. Foram implementadas e validadas preferências
Linux independentes para compatibilidade de vídeo e gráfica, lidas antes das
janelas. No host Ubuntu 26.04.1/GNOME Wayland/NVIDIA nouveau, o AppImage final
foi validado com essas opções habilitadas; seu launcher GTK seleciona X11.

Ocorrências intermitentes de heap, foco, lentidão, SQLite/WebProcess e avisos
de runtime ficam apenas como notas futuras (seção 23), conforme decisão do
usuário. Não são defeitos confirmados da aplicação nem tarefas desta conclusão.
O aceite Linux é funcional e limitado aos ambientes/artefatos registrados;
não comprova suporte universal ou término de todo recurso não supervisionado.

R1–R4 estão concluídos nos caminhos e cenários registrados, com implementação
e cobertura automatizada de R4 na seção 31 e validação Windows na seção 32.
R5 está concluído nos cenários registrados: quatro encerramentos iniciais e um reteste do pacote corrigido com saída zero e `clean=true`, incluindo
boot parcial antes de Media/BackendReady (seções 34 e 36). A regressão de streaming
do pacote foi reproduzida como bloqueio CORS da origem Windows e corrigida;
o usuário confirmou funcionamento correto no pacote instalado, com mídia/processamento e IPC ativo drenados antes do pool.
Restam R7 (smoke
macOS final), R8 (decidir medições restantes fora da sessão Linux encerrada)
e R9 (aceite geral). R6 está concluído.

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

O levantamento inicial foi estático. As medições, testes desktop e validações
visuais posteriores estão registrados nas seções 13 a 22; a cobertura vigente
está consolidada na seção 10.2. Não se trata mais de uma proposta sem execução.

## 4. Estado encontrado no levantamento inicial (histórico)

### 4.1 Sequência observada

1. O setup registra `LifecycleRegistry` e inicia o boot assíncrono em [bootstrap/startup.rs](../../src-tauri/src/bootstrap/startup.rs).
2. A splash é aberta pela configuração nativa; a principal começa oculta. Depois do handshake de readiness, a principal é exibida e a splash é fechada.
3. Em [lib.rs](../../src-tauri/src/lib.rs), `CloseRequested` da principal ou da splash antes de readiness é interceptado com `prevent_close()`. A splash pronta e janelas auxiliares são tratadas separadamente por `should_shutdown_on_close`.
4. Uma tarefa assíncrona chama `shutdown_all()` se o registro estiver disponível e então `handle.exit(0)`. O pedido não fecha imediatamente a janela interceptada; não existe apresentação ou estado de finalização específico nesse fluxo.
5. `shutdown_all()` cancela o token raiz, retira os handles registrados e os aguarda **sequencialmente**, com timeout de cinco segundos **por handle**; em timeout chama `abort()`. Erros são registrados em log e a função retorna `()`.

O cliente da splash já usa evento, consulta de snapshot, revisão monotônica e limpeza dos listeners. Sua UI usa mensagem em inglês, progresso indeterminado, `role="status"`, adaptação a tema do sistema e `prefers-reduced-motion`. É uma boa referência, mas não basta renomear as mensagens: o ciclo de finalização tem regras próprias.

### 4.2 Pontos relevantes para a solução

| Achado no código                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Consequência para o plano                                                                                                                                                                                                   |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `CloseRequested` não tem guarda de “já encerrando”; cada pedido pode iniciar outra tarefa.                                                                                                                                                                                                                                                                                                                                                                                              | A segunda execução pode encontrar o mapa já esvaziado e solicitar saída antes da primeira terminar. É necessário um dono único e um mecanismo atômico de adesão ao mesmo encerramento.                                      |
| Boot é disparado separadamente; `register()` aceita novos handles depois de `shutdown_all()` retirar o mapa.                                                                                                                                                                                                                                                                                                                                                                            | Fechar durante o boot pode concorrer com inicialização e registro tardio. Token cancelado por si só não comprova que não houve novo recurso ou tarefa fora da drenagem.                                                     |
| Cada handle recebe seu próprio timeout em sequência e a iteração é de `HashMap`.                                                                                                                                                                                                                                                                                                                                                                                                        | Não há limite global de cinco segundos, ordem determinística nem progresso por etapas inferível da ordem atual. O pior caso pode crescer com a quantidade de tarefas.                                                       |
| Timeout faz `abort()` sem aguardar novamente o handle; panic e timeout viram logs, não resultado estruturado.                                                                                                                                                                                                                                                                                                                                                                           | Não se pode afirmar que todas as operações internas ou nativas terminaram. Uma saída degradada precisa ser identificada, registrada e testada.                                                                              |
| Em [sensor.rs](../../src-tauri/src/processing/watcher/sensor.rs), `watch` ignora o token e cria loops com `tokio::spawn`; em [bootstrap/library.rs](../../src-tauri/src/bootstrap/library.rs), o handle registrado por watcher é uma tarefa vazia.                                                                                                                                                                                                                                      | O handle registrado não prova encerramento do watcher nativo nem de seus loops. O plano precisa de desligamento real e verificável, não apenas mudança de rótulo.                                                           |
| O servidor em [server.rs](../../src-tauri/src/delivery/streaming/server.rs) registra o handle de `axum::serve`, mas cria loops de limpeza independentes, sem seus handles na drenagem.                                                                                                                                                                                                                                                                                                  | Cancelamento de filhos não equivale a aguardar seu término. Requisições ativas, loops e gerenciadores precisam ter propriedade e conclusão explícitas.                                                                      |
| O [HlsManager](../../src-tauri/src/feature/transcoding/hls_manager.rs) limpa sessões com `kill_on_drop`; [LinearManager](../../src-tauri/src/delivery/streaming/linear_manager.rs) mata/aguarda filhos ao limpar sessões expiradas, mas o loop do servidor apenas interrompe na saída; [ProcessManager](../../src-tauri/src/delivery/streaming/process_manager.rs) rastreia PIDs e [segment.rs](../../src-tauri/src/delivery/streaming/segment.rs) aguarda subprocessos em requisições. | Não afirmar que todos os processos foram encerrados/recolhidos apenas porque o servidor retornou. Auditar cada dono e seus caminhos de cancelamento, inclusive requisições em voo e subprocessos fora desses gerenciadores. |
| [DbManager](../../src-tauri/src/infra/database/manager.rs) usa pool SQLx e WAL; o fluxo de saída inspecionado não invoca fechamento explícito desse pool.                                                                                                                                                                                                                                                                                                                               | Identificar proprietários e conexões ativas antes de decidir se há fechamento explícito seguro. Não adicionar checkpoint, flush ou mensagem “salvando” sem necessidade e sem comprovação.                                   |
| [Reservas de processamento](../backend/workers/2026-11-03-23-32-processing-leases.md) e [journal de operações](../backend/library-operations.md) oferecem mecanismos de recuperação com limites próprios.                                                                                                                                                                                                                                                                               | Preservar publicação transacional, propriedade das leases, arquivos originais e recuperação no boot; não prometer conclusão de jobs ou recuperação universal.                                                               |

O documento histórico de [controle dos workers](../backend/workers/2026-10-30-05-17-workers-processing-control.md) registra problemas anteriores de encerramento; seu diagnóstico é histórico e não deve ser assumido como atual sem conferir os arquivos presentes.

### 4.3 Entradas e diferenças entre plataformas

O botão customizado de Windows/Linux chama `getCurrentWindow().close()` em [WindowControls.tsx](../../src/components/layout/TitleBar/WindowControls.tsx); no macOS há controles nativos. O código inspecionado trata `WindowEvent::CloseRequested`, não `RunEvent::ExitRequested`. Portanto, **não está demonstrado** que todas as formas de “Quit” do menu, Dock, atalhos, encerramento de sessão do SO e outras origens passem pelo mesmo fluxo.

Na versão Tauri instalada, `AppHandle::exit` também dispara `ExitRequested`, e o evento distingue saída solicitada pelo usuário (`code: None`) de saída programática (`Some`). A API documenta que `prevent_exit` é ignorado para restart. A implementação deve verificar o comportamento real e a precedência dos eventos em cada plataforma, inclusive ao sair após o próprio coordenador, sem criar um ciclo infinito. A distinção por código é informação auxiliar, **não** prova suficiente de autorização para sair: usar o estado interno do coordenador como autoridade. Restart, término forçado e solicitações de shutdown do sistema podem ter restrições distintas e precisam de testes; não prometer interceptação universal.

## 5. Alternativas de apresentação

> Proposta histórica. Decisões implementadas e aceite vigente estão nas seções 9–14 e 23.

| Alternativa                        | Vantagens                                                                               | Limitações                                                                                                              | Avaliação                                                                    |
| ---------------------------------- | --------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Overlay na janela principal        | Imediato, reutiliza o frontend já carregado.                                            | Não cobre boot/falha da principal; pode herdar bloqueio, navegação ou estado quebrado.                                  | Útil apenas como fallback.                                                   |
| Transformar a splash existente     | Já possui identidade e cliente de estado.                                               | Foi fechada após startup; reaproveitar label e fluxo mistura semânticas e pode reabrir/alterar o comportamento do boot. | Reutilizar padrões ou componentes realmente comuns, não o estado de startup. |
| **Janela dedicada e independente** | Cobre saída normal e boot parcial; permissões restritas; superfície visual consistente. | Criação/renderização podem falhar ou demorar; há custo de WebView e detalhes nativos por plataforma.                    | **Recomendada**, com fallback e timeout.                                     |
| Diálogo nativo                     | Menor dependência do frontend em alguns ambientes.                                      | Aparência e recursos acessíveis variam; APIs e comportamento bloqueante exigem estudo.                                  | Fallback potencial, somente se disponível e testado.                         |

Janela dedicada **não** é garantia de visibilidade: compositor, minimização, virtual desktops, tela desconectada, acessibilidade do WebView e saída do SO podem impedir a apresentação. Não usar `alwaysOnTop` global por padrão nem roubar foco de outros aplicativos. Testar agrupamento na taskbar/Dock, foco e posição antes de escolher opções nativas definitivas.

## 6. Experiência proposta

> Proposta histórica. Decisões implementadas e aceite vigente estão nas seções 9–14 e 23.

### 6.1 Composição e mensagens

Tela enxuta, sem barras de ferramentas, em proporção próxima à splash atual (480 × 360 como ponto de partida, não como requisito rígido). Identidade Mundam, título explícito, status e indicador indeterminado. Centralizar na tela pertinente quando possível, respeitar escala e área útil, e permitir que o conteúdo se adapte a texto ampliado.

O produto usa inglês na splash existente; portanto, exemplos abaixo seguem esse padrão até existir infraestrutura de localização aplicável. Não duplicar strings em vários pontos; permitir futura tradução.

| Situação comprovada                      | Exemplo de texto                                                                          | Regra                                                                                                              |
| ---------------------------------------- | ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Pedido aceito                            | **Closing Mundam** / “Finishing up. Please wait.”                                         | Exibir imediatamente; não sugerir que o trabalho está concluído.                                                   |
| Admissão fechada e drenagem iniciada     | “Stopping background activity.”                                                           | Só anunciar após essa transição ter ocorrido.                                                                      |
| Espera prolongada, após 8 s              | “Taking longer than usual · 8s”                                                           | Tempo decorrido real e monotônico, atualizado visualmente sem anúncio por leitor de tela a cada segundo.           |
| Sem contato IPC por 5 s                  | “Waiting for the application to respond.”                                                 | Diferenciar falta de resposta de uma falha comprovada; pausar animação de progresso, continuar tentando consultar. |
| Falha conhecida ainda tratável           | “Mundam could not finish closing normally.”                                               | Mostrar código seguro e orientação adequada ao desfecho real; detalhes somente nos logs.                           |
| Resultado degradado com saída automática | Registrar diagnóstico e usar mensagem compatível apenas se houver tempo real de exibição. | Não apresentar “concluído com sucesso” após timeout/panic nem bloquear indefinidamente para exibir uma mensagem.   |

Os limiares de 5 e 8 segundos seguem a splash como **ponto de partida**, não constituem prova de travamento. Usar relógio monotônico e constantes nomeadas. As fases de trabalho só devem aparecer se corresponderem a marcos observáveis; não inferir progresso por nome de tarefa, quantidade de handles ou passagem de tempo. Sem porcentagem, ETA ou frases como “All changes saved” sem garantia real.

### 6.2 Interação e acessibilidade

**Atualização de escopo em 13 de novembro:** as recomendações abaixo preservam
as decisões de apresentação e o comportamento já implementado. Testes de
acessibilidade, leitores de tela, contraste, alto contraste, ampliação e
movimento reduzido não são exigências para concluir esta implementação.

- A saída já foi solicitada: não oferecer cancelamento/reinício quando não houver reversão segura. Durante o fechamento, nova tentativa de fechar a janela de progresso é interceptada e se junta ao mesmo fluxo; não cria outra instância nem força a saída.
- `main` com título semântico, indicador com `role="progressbar"` indeterminado e nome acessível, e uma única região `role="status"`/`aria-live="polite"` para mudanças relevantes. Nenhum `aria-valuenow` fabricado.
- Sem depender apenas de animação ou cor; respeitar `prefers-reduced-motion`. Contraste de texto e componentes não textuais no mínimo WCAG AA aplicável, inclusive estados de foco.
- Manter controles de fallback acessíveis via teclado quando existirem, com foco visível e sem armadilha de foco. Não criar botão decorativo sem ação. Verificar leitura inicial e atualizações com leitores de tela reais, pois um status criado antes de ganhar foco pode não ser anunciado.
- Usar tokens existentes e CSS colocalizado, com variantes claro/escuro. A splash usa preferência do SO; se o tema persistido estiver disponível sem dependência de boot, avaliar seu uso, mas na ausência dele seguir o SO sem bloquear a apresentação. Testar alto contraste, zoom, escalonamento e textos longos.

### 6.3 Tempo mínimo sem prejudicar limpeza

Recomendação inicial: **600 ms de visibilidade confirmada**, não 600 ms antes de começar a limpeza. O backend usa relógio monotônico. A UI sinaliza uma única vez que montou o conteúdo e teve oportunidade de renderizar; esse handshake é uma aproximação, não prova absoluta de pixels visíveis ou anúncio pelo leitor de tela. Testar apresentação real com o compositor.

O encerramento pode terminar antes do handshake; nesse caso, aguardar somente o menor tempo restante de uma janela de apresentação **limitada**, proposta de **1.500 ms desde o pedido**, e só cumprir o mínimo se houver confirmação válida e janela mostrada. Exemplo: limpeza em 100 ms, confirmação em 200 ms: sair em torno de 800 ms; sem confirmação até 1.500 ms: seguir sem espera adicional. O handshake jamais retarda o início do cancelamento. Em saída do SO sujeita a prazo externo, falha de criação ou indisponibilidade, priorizar a saída segura e não impor esse mínimo. Os valores devem ser validados e aprovados antes da implementação.

## 7. Arquitetura e protocolo recomendados

> Proposta histórica. Decisões implementadas e aceite vigente estão nas seções 9–14 e 23.

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

| Superfície                                                                  | Responsabilidade                                                                                                                           |
| --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `src-tauri/src/feature/shutdown/`                                           | Estado, transições, resultado e coordenação testável, independente de Tauri.                                                               |
| `src-tauri/src/bootstrap/` e `delivery/tauri/`                              | Composição, eventos de janela/aplicação, publicação e comandos finos.                                                                      |
| [lifecycle.rs](../../src-tauri/src/lifecycle.rs) e donos dos recursos       | Admissão/registro, cancelamento e drenagem reais, com resultado explícito.                                                                 |
| `src/core/shutdown/`                                                        | Contrato tipado, observação e reconciliação de IPC com limpeza de listeners.                                                               |
| `src/components/features/shutdown/`                                         | Componente Solid e estilos próprios; abstrair elementos compartilhados com startup apenas se houver responsabilidade comum real.           |
| `shutdown.html`, `src/shutdown.tsx`, [vite.config.ts](../../vite.config.ts) | Entrada independente, empacotada no build; apenas dependências essenciais.                                                                 |
| `src-tauri/capabilities/` e `permissions/`                                  | Capability exclusiva para a label escolhida, consulta de estado, escuta/remoção de listener e, se necessário, confirmação de apresentação. |

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

| Falha                                                   | Conduta proposta                                                                                                                        |
| ------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Janela não criada, não mostrada ou WebView não confirma | Log estruturado; manter feedback na origem quando possível; continuar limpeza e saída após limite de apresentação, sem espera infinita. |
| Evento perdido                                          | Recuperar por consulta do snapshot; logar falhas de emissão.                                                                            |
| IPC sem resposta                                        | Mensagem de espera, animação pausada, reconexão limitada por intervalos; não converter ausência de resposta em sucesso ou erro fatal.   |
| Panic, erro ou timeout de recurso                       | Registrar categoria e duração; resultado degradado. Aplicar somente política segura e verificável de cada recurso.                      |
| Fechamento repetido ou tentativa de fechar progresso    | Manter um coordenador; impedir saída prematura quando possível; não abrir múltiplas janelas.                                            |
| Boot parcial ou falho                                   | Drenar apenas recursos realmente adquiridos, com supervisão do boot; não assumir banco, configurações ou principal presentes.           |
| Processo morto externamente                             | Sem garantia de UI ou cleanup. Confiar somente nos mecanismos de recuperação efetivamente existentes e testados.                        |

Não exibir na janela dados internos de bibliotecas, caminhos, nomes de arquivos, stack traces ou argumentos de subprocessos. Usar `tracing` com início, origem normalizada, transições, duração por categoria, falhas e resultado; evitar telemetria remota e detalhes pessoais. O estado mantido em memória não substitui journal durável nem relatório de crash.

## 8. Segurança e compatibilidade

> Proposta histórica. Decisões implementadas e aceite vigente estão nas seções 9–14 e 23.

- A nova capability deve ser restrita à nova label; não herdar a capability ampla de [default.json](../../src-tauri/capabilities/default.json), acesso a banco, filesystem, biblioteca, opener ou comandos de mutação.
- Verificar tanto ACL quanto validação do chamador no backend para comandos sensíveis; eventos devem ser direcionados ou seus payloads limitados. Testar chamadas não autorizadas.
- Evitar recursos remotos; garantir que a nova entrada faça parte de `vite build` e do bundle Tauri em dev e produção. Manter a splash e o design system funcionando independentemente.
- Não alterar sem necessidade a política de fechamento de janelas auxiliares nem o comportamento de startup concluído. Fechamento da principal minimizada/oculta e múltiplas telas devem ser verificados.
- Não supor que toda solicitação de saída do SO possa ser adiada; documentar comportamento específico e exceções medidas por plataforma.

## 9. Plano de execução e estado atual

| Etapa                     | Trabalho e evidência                                                                                                        | Estado em 18 de novembro                                                                                                                             |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. Inventário e contratos | Entradas, donos, prazos e limitações registrados; evidências das três plataformas.                                          | Atualizado; R1–R4 concluídos nos escopos registrados, com limites nativos e política de saída degradada explícitos.                                                                                             |
| 2. Coordenação            | Coordenador único, snapshot, saída autorizada, registro atômico e supervisor de boot.                                       | Implementada; admissão de comandos de negócio validada em R3 (seções 29–30).                                                                                    |
| 3. Drenagem real | Protocolos, tarefas registradas, watcher, auxiliares HTTP/HLS por IPC, bridge, segmentos/probes, sessões lineares e manutenção de banco supervisionados. | R1–R4 concluídos nos caminhos/cenários registrados (seções 26–32); limites nativos e saída degradada documentados. |
| 4. Janela e protocolo     | Entrada Vite, capability, consulta/evento, handshake, mínimo, fallback e pausa de mídia.                                    | Implementados; saída autônoma com falhas injetadas validada no Linux.                                                                                |
| 5. Apresentação funcional | Janela observada nas três plataformas; preferências Linux de vídeo/gráfica persistentes.                                    | Sessão Linux concluída; acessibilidade dispensada; observações intermitentes sem bloqueio.                                                           |
| 6. Integração e aceite    | Windows dev e evidências macOS anteriores preservados; Linux dev/release/Debian/AppImage validado.                          | R6 concluído. Restam os itens gerais da seção 14, sem reabrir a sessão Linux.                                                                        |

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
- Montagem, confirmação de apresentação uma única vez, erro de montagem/IPC e fallback. Testes já existentes de semântica podem continuar, mas não há campanha de acessibilidade exigida para o aceite.
- Relógio controlado para limpeza rápida/lenta, handshake antes/depois do término, ausência ou confirmação tardia, prazo de 1.500 ms e mínimo de 600 ms. Testar que iniciar limpeza não aguarda o frontend.

**Verificações realizadas:** a rodada Windows registrou 19 testes frontend,
19 de streaming Rust e 23 filtrados por shutdown (com sobreposição entre suítes).
No Linux foram registrados, entre outras rodadas focadas, dois testes de
manutenção, 32 testes frontend da correção de mídia/classificação e 10 testes
Rust dos bootstraps de compatibilidade. Contagens pertencem às rodadas descritas,
não representam uma suíte única nem total de testes distintos. Typecheck, lint,
formatação e builds pertinentes foram aprovados conforme as seções 13–20.
Esta revisão é documental e não reexecutou essas suítes.

### 10.2 Matriz desktop consolidada

A tabela registra cobertura por plataforma, sem transformar a sessão Linux
encerrada em nova campanha obrigatória. Uma validação em determinado modo não
comprova repetição integral da matriz em todos os artefatos ou distribuições.

| Cenário                                                      | Windows                                                                                      | macOS                                                               | Linux — sessão concluída                                                                                                                               |
| ------------------------------------------------------------ | -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Saída ociosa e entradas normais                              | Dev e instalação R5 observados; quatro saídas limpas na seção 34.                                   | Titlebar, Dock e Cmd+Q observados em dev e bundle anterior.         | X e Alt+F4 em dev/release; ocioso instalado em Debian e AppImage.                                                                                      |
| Indexação/processing e mídia ativos                          | R5 concluído: reprodução/seek confirmados pelo usuário e saída sob mídia/processamento no pacote corrigido (seção 36).                             | Evidências anteriores sob carga; smoke final em R7.                 | Biblioteca nova de 787 assets, trabalhos pendentes, reprodução nativa/HLS/linear; Debian e AppImage exercitados.                                       |
| Fechamento na splash/boot                                    | R4 com backend pronto; R5 instalado cancelou em Database, antes de Media/BackendReady, com saída limpa (seção 34).                     | Evidência parcial preservada; política compartilhada verificada em R4, smoke final em R7.             | Splash fechada com backend já pronto; não equivale a falha em cada marco anterior do boot. Sessão aceita com esse limite.                              |
| Pedidos repetidos e auxiliares                               | Unidade e fluxos principais cobertos; complementar no bundle atualizado.                     | Entradas principais observadas; smoke final delimitado às mudanças. | Alt+F4 repetido, um único coordenador e uma única tela confirmados em desenvolvimento.                                                                 |
| Reabertura e recuperação                                     | Retomada em R4 e reaberturas instaladas R5; integridade integral no artefato final continua pendente.                     | Persistência e retomada observadas; recuperação compartilhada verificada em R4, smoke final em R7.    | Reabertura, catálogo preservado e preferências recuperadas; não comprova recuperação universal de transações/leases.                                   |
| Principal minimizada e indisponibilidade de apresentação/IPC | Minimizada/auxiliares relatados como corretos no instalado R5, com saída limpa; falhas injetadas preservadas em testes anteriores.                                              | Evidência anterior; smoke final em R7.                              | Minimizada e saída pelo dock; falhas injetadas de IPC e montagem com saída autônoma validadas em desenvolvimento.                                      |
| Processos, servidor, handlers e SQLite                       | Saídas limpas dos recursos registrados; R1–R4 concluídos nos escopos registrados, com limites nativos explícitos. | Mesma delimitação de implementação compartilhada.                   | Drenagem, terminação aguardada de segmentos e ausência de remanescentes em amostras limpas. Ocorrências intermitentes são notas, não pendências Linux. |
| Logout/terminação forçada                                    | Fora da garantia de saída normal, salvo requisito específico futuro.                         | Mesmo limite.                                                       | Mesmo limite; não reabre R6.                                                                                                                           |

Testes de acessibilidade estão dispensados. O ambiente Linux registrado foi
Ubuntu 26.04.1, GNOME/Wayland, WebKitGTK 2.52.6 e GStreamer 1.28.2; o AppImage
usa X11 pelo hook GTK. Os modos de compatibilidade foram validados no host
afetado, sem impor overrides globais a todos os hosts. A sessão não inclui
instalação limpa, outras distribuições ou uma matriz completa de hardware.

### 10.3 Medições de aceite

**Estado vigente:** medições amostrais Linux foram aceitas para encerrar R6.
A série estatística originalmente proposta não foi coletada integralmente e
não será exigida como pendência desta sessão Linux. R8 permanece somente para
decidir coleta ou dispensa das medições restantes no encerramento geral.
Os tópicos abaixo preservam a metodologia original, não novos roteiros Linux.

- Instrumentar: `pedido → primeira apresentação confirmada`, `pedido → cancelamento`, `pedido → término da drenagem`, `pedido → saída`; distinguir confirmação do frontend de visibilidade observada.
- Confirmar por teste automatizado que, em saída normal com apresentação confirmada, a janela permanece pelo menos **600 ms** a partir da confirmação; que a ausência de confirmação não adiciona mais de **1.500 ms desde o pedido** por motivo de apresentação; que a limpeza começa sem esperar handshake. Considerar precisão dos timers e registrar a tolerância adotada no teste.
- Medir distribuição, não apenas um caso: ao menos 10 execuções quentes e 10 frias por plataforma, em repouso e com carga representativa; registrar mediana, máximo, metodologia e hardware. Não estabelecer meta de latência de criação sem baseline.
- Comparar consumo e tempo com a versão anterior; o único atraso deliberado de saída normal deve ser o mínimo de apresentação definido, sobreposto à limpeza. Nenhum timeout de interface deve prolongar a saída indefinidamente.
- Registrar como pendente qualquer resultado ainda não observado; não interpretar teste unitário de mocks como evidência de funcionamento nativo ou processo filho encerrado. Acessibilidade está dispensada.

## 11. Decisões aprovadas e limites gerais

| Tema                   | Recomendação inicial                                                        | Risco/decisão                                                                                                                                        |
| ---------------------- | --------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Tempo mínimo           | 600 ms, medidos após confirmação, em paralelo à limpeza.                    | Acrescenta latência deliberada em saídas rápidas; validar percepção e aceitar valor.                                                                 |
| Limite de apresentação | 1.500 ms desde pedido; sem mínimo se não houver confirmação válida.         | Sob carga, janela pode não ser vista; registrar fallback e medir.                                                                                    |
| Saída degradada        | Prosseguir somente após aplicar política segura por recurso; log explícito. | Uma operação não interrompível pode impedir limite global rígido; acordar política após inventário.                                                  |
| Controles              | Sem cancelar nem forçar durante encerramento normal.                        | Se existir espera indefinida real, avaliar resposta acionável e recuperação segura em proposta separada; não oferecer botão sem semântica confiável. |
| Tema                   | Preferência do SO como fallback, semelhante à splash.                       | Aparência persistida pode diferir; avaliar disponibilidade segura e consistência.                                                                    |
| Escopo de saída        | Todos os caminhos normais que o SO/Tauri de fato permitem interceptar.      | Reinício, logout e terminação externa têm semânticas próprias; documentar cobertura por plataforma.                                                  |
| Localização            | Inglês conforme splash atual; mensagens centralizadas.                      | Traduzir quando houver padrão de localização, sem introduzir sistema novo apenas para esta tela.                                                     |

**Decisões vigentes:** janela independente, 600 ms/1.500 ms, ausência de ação
genérica de fechamento forçado e prioridade à integridade foram aprovadas e
implementadas. A política para operações não interrompíveis foi definida e
verificada em R4, com os limites e resultados das seções 31–32. A compatibilidade Linux é opt-in, persistente e validada;
não há aprovação pendente desses controles ou da sessão Linux.

## 12. Lista de entregas e acompanhamento

Atualizado em 19 de novembro contra o código e as evidências das três plataformas.
A sessão Linux foi encerrada por decisão do usuário; itens de implementação
compartilhada não são defeitos inferidos dos erros intermitentes do host.
As pendências detalhadas e a ordem de execução estão na seção 14.

- [x] Confirmar decisões de UX, tempos e cobertura de saída com o responsável pelo produto.
- [x] Registrar entradas conhecidas, inventário de recursos, políticas e lacunas verificadas.
- [ ] Concluir cobertura nativa das entradas restantes e das plataformas finais.
- [x] Registrar tempos amostrais de encerramento em macOS, Windows e Linux.
- [ ] Decidir coleta ou dispensa das medições gerais restantes (R8); não reabrir a sessão Linux já aceita.
- [x] Implementar coordenador único, estado revisionado e saída final sem recursão.
- [x] Fechar atomicamente o registro e admitir scans dinâmicos sob a mesma trava; reter e aguardar o supervisor do boot.
- [x] Drenar tarefas registradas concorrentemente, distinguir resultado degradado e drenar bridge por último.
- [x] Fechar admissão dos protocolos de assets/thumbnails e aguardar handlers admitidos.
- [x] Supervisionar e aguardar processos de segmentos, probes e sessões lineares do servidor HTTP.
- [x] Pausar mídia da principal quando o pedido de encerramento for aceito.
- [x] Implementar encerramento e espera do watcher nativo e seus loops com propriedade verificável (R1; seção 24).
- [x] Revalidar R1 no aplicativo após corrigir registro duplicado de raízes: duas saídas Windows v2 com clean=true (seção 26); bundle final continua em R5.
- [x] Implementar supervisão dos auxiliares HTTP/HLS por IPC e registrar auditoria dos caminhos alcançáveis e limites nativos (R2; seção 27).
- [x] Confirmar no aplicativo os auxiliares HTTP e o dono IPC antes do bridge/pool, com HLS e linear em uso: duas saídas clean=true (R2; seção 28).
- [x] Implementar admissão/drenagem de IPC que pode criar recursos ou acessar o banco durante a saída (R3; seção 29).
- [x] Confirmar no aplicativo a integração de R3: uso normal e transferências de texto relatados pelo usuário, scans manuais registrados, recusa tardia e fechamento limpo (seção 30). Scans terminaram antes do X; concorrência ativa coberta automaticamente.
- [x] Implementar política e cobertura automatizada de boot/I/O não interrompível, falhas e recuperação (R4; seção 31).
- [x] Validar a integração de R4 no aplicativo: saída pela splash relatada, processamento ativo drenado e reabertura com nova raiz/pendências preservadas (seção 32). As fases iniciais do backend já haviam terminado no primeiro fechamento; seu cancelamento e deadlines permanecem cobertos automaticamente.
- [x] Preservar leases ao adiar thumbnails por cancelamento entre fases, sem consumir nova tentativa.
- [ ] Confirmar recuperação de pendências e integridade no reinício dos artefatos finais.
- [x] Implementar janela independente, entrada Vite e empacotamento.
- [x] Criar capability mínima e validar autorização de IPC por janela.
- [x] Implementar protocolo evento + consulta, reconciliação e descarte dos listeners.
- [x] Implementar mínimo de 600 ms, handshake limitado a 1.500 ms, fallback e mensagens factuais.
- [x] Testar concorrência, timeouts e ausência de confirmação sem bloqueio indefinido.
- [x] Validar no Linux splash, pedidos repetidos, minimizada e falhas injetadas de apresentação/IPC nos cenários registrados.
- [ ] Completar o smoke macOS final de R7; Windows R5 concluído, preservando a política e os limites de R4.
- [x] Executar testes automatizados, typecheck, lint e build pertinentes.
- [x] Registrar a rodada final de desenvolvimento Windows: três encerramentos `clean=true`.
- [x] Concluir R5: quatro encerramentos iniciais e reteste do pacote corrigido aprovados; funcionamento confirmado pelo usuário, saída zero e clean=true sob mídia/processamento (seção 36).
- [x] Gerar e identificar os instaladores Windows, verificar recursos e preparar captura; pacote inicial na seção 33 e pacote corrigido na seção 35 (R5).
- [x] Analisar os quatro cenários instalados de R5: saída zero, clean=true, boot parcial cancelado, carga, auxiliares e minimizada registrados; regressão de mídia isolada para reteste após correção CORS (seção 34).
- [x] Concluir Linux em desenvolvimento/release, binário direto, Debian e AppImage, com limitações aceitas (R6).
- [x] Validar mídia nativa Linux e preferências persistentes independentes de vídeo e gráfica.
- [x] Supervisionar library_cleanup e saga_recovery, com regressões e validação runtime Linux.
- [ ] Fazer smoke macOS das mudanças compartilhadas posteriores à validação anterior (R7).
- [x] Atualizar tabelas, checklist, inventário e referências de startup/backend nesta revisão.
- [ ] Registrar decisões finais, desvios, riscos residuais, resultados e aprovação.

**Dispensado:** testes de acessibilidade não compõem este checklist de conclusão.
**Fora do escopo atual:** suporte adicional a formatos, thumbnails conhecidos,
prompts e desempenho de rede, salvo impacto demonstrado no fechamento. As
correções de mídia nativa/compatibilidade Linux foram uma ampliação autorizada
e já concluída. Notas intermitentes da sessão Linux não compõem o checklist.

## 13. Detalhes da implementação e evidências

**Histórico de 9 a 13 de novembro de 2026:** os quadros e relatos datados
preservam o estado observado em cada rodada, inclusive pendências superadas.
O inventário abaixo foi atualizado em 13 de novembro. Para tarefas atuais,
usar o checklist da seção 12 e o levantamento da seção 14; não acumular como
obrigações novas todas as propostas ou observações históricas. Não considerar
esta seção como aceite final.

### Decisões aprovadas

| Data       | Decisão                                                                                                                | Responsável / referência                               |
| ---------- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| 2026-11-09 | Plano aprovado; iniciar com janela independente usando Solid/Vite, simplificando para HTML/JS próprio caso necessário. | Solicitação do usuário nesta sessão.                   |
| 2026-11-09 | Primeira etapa mantém os tempos propostos de 600 ms e 1.500 ms e usa o estado de finalização separado do startup.      | Implementação em andamento; validação nativa pendente. |

### Inventário e política de recursos (reconciliado em 18 de novembro)

| Recurso / dono                         | Admissão e cancelamento                                                                                                                                                                            | Evidência de término                                                                                                            | Timeout / recuperação                                                                                               | Estado                                                                                                          |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Inicialização | Token raiz cancelado; fase admitida é aguardada e fases posteriores verificam cancelamento. Supervisor retido, inclusive após timeout. | Testes em cada fronteira, falhas por fase, esperas concorrentes e I/O nativo retido após deadline (seção 31). | Espera de startup de até 5 s; timeout degrada a saída e impede fechamento explícito do pool. Não aborta I/O nativo. | R4 concluído no escopo registrado; backend já pronto no teste pela splash, fases iniciais cobertas automaticamente (seção 32). |
| Registro de tarefas e scans dinâmicos  | `shutdown_all` fecha o registro sob trava; `spawn_unique` verifica cancelamento e reserva/inicia sob a mesma trava.                                                                                | Handles aguardados; testes de concorrência, rejeição tardia e nomes duplicados.                                                 | 5 s compartilhados, abort aguardado; grace de 250 ms para bridge.                                                   | Implementado para recursos admitidos por esses caminhos; não equivale a barreira global de IPC.                 |
| Bridge de eventos                      | Cancelado após os produtores registrados; fila consumida antes de parar.                                                                                                                           | Handle registrado e testes de ordem/drenagem.                                                                                   | Orçamento compartilhado mais grace limitado.                                                                        | Implementado; produtores não supervisionados continuam como limite.                                             |
| Asset/thumbnail protocols              | `ProtocolAdmission` fecha a admissão e conta guards já admitidos.                                                                                                                                  | Drenagem de handlers registrada nos três últimos logs Windows.                                                                  | Até 5 s; falha degrada o resultado e impede fechamento explícito do pool.                                           | Implementado; não prova conclusão de todo corpo HTTP já entregue.                                               |
| Servidor HTTP, segmentos e FFprobe     | Token raiz fecha servidor; processos registrados recebem cancelamento e seus donos aguardam saída/saídas de pipes.                                                                                 | 19 testes de streaming; servidor e donos concluíram nas três novas execuções Windows.                                           | Drenagem dentro do orçamento do lifecycle; falhas de supervisão degradam resultado.                                 | Implementado; CORS corrigido e pacote instalado validado em R5, com donos aguardados antes do pool (seção 36).                  |
| Sessões lineares HTTP                  | Admissão serializada e fechada; filhos e diretórios tratados em `LinearManager::shutdown`.                                                                                                         | Relatório de falhas de processo/diretório; testes e logs de sessões reais.                                                      | Lifecycle limita espera; falhas são propagadas.                                                                     | Implementado para HTTP; sessão com processo ativo aguardada e limpa no instalado R5 (seção 34).                       |
| Watcher nativo, debouncer e publicador | Token único, admissão fechada no cancelamento e dono library_watcher registrado. Registro repetido do mesmo caminho não chama novamente o backend nativo.                                          | Liberação do callback, conclusão dos loops e do dono antes do bridge confirmadas nos dois logs Windows v2.                      | Hints voláteis reconciliados no boot; timeout continua degradando o resultado, sem prometer interromper I/O nativo. | R1 concluído nos cenários dirigidos: clean=true nas duas execuções, com pool fechado explicitamente (seção 26). |
| Limpeza periódica HTTP e HLS do IPC | Cancelamento fecha agendamento HTTP e admissão do worker IPC; preparações/limpezas admitidas permanecem com seus donos. | Dois auxiliares HTTP e dono IPC concluídos antes do bridge nos dois logs Windows; testes com filhos IPC e FFmpeg real aprovados. | Prazo global existente; I/O nativo iniciado pode excedê-lo (R4). | R2 concluído no escopo auditado e cenários validados; runtime IPC ocioso, filhos ativos na cobertura automatizada (seções 27–28). |
| Manutenção SQLite                      | library_cleanup e saga_recovery admitidos no lifecycle; limpeza não inicia nova rodada após cancelamento.                                                                                          | Dois testes e runtime Linux, com espera antes do fechamento do pool.                                                            | Rodada já iniciada é aguardada sob o orçamento existente.                                                           | Corrigido e validado; não é mais lacuna autônoma de R2.                                                         |
| Workers e extratores | Cancelamento cooperativo e handles retidos nos donos; finish aguarda liberação de lease. Drop anormal solicita abort do heartbeat e deixa recuperação pela expiração. | Regressões de workers/leases, abort assíncrono com native ainda ativo, transações e tickets antigos (seção 31). | I/O/FFI e filhos já iniciados podem exceder o prazo; resultado degradado sem fechamento explícito do pool. Reservas de processamento expiram até 120 s após aquisição/renovação. | R4 concluído no escopo registrado, com drenagem sob processamento validada (seção 32); sem interrupção universal de operações nativas. |
| Banco SQLite e IPC | Pool fecha só após tarefas/protocolos limpos, admissão IPC fechada e drenada e snapshot totalmente ocioso. | Saídas Windows com pool fechado, recusa tardia real e consulta IPC ativa aguardada antes do bridge/pool; transação retida coberta automaticamente (seções 29–32). | Orçamento compartilhado de 5 s para drenagem; chamadas ativas impedem fechamento explícito do pool. Ociosidade até 250 ms e close até 3 s preservados. | R3 concluído nos caminhos registrados. I/O não interrompível em R4. |

### Alterações realizadas

| Data       | Arquivos / área                                                                                                             | Alteração e justificativa                                                                                                                                                                                                                                                                | Referência                                                                                              |
| ---------- | --------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| 2026-11-09 | `src-tauri/src/feature/shutdown.rs`, `bootstrap/shutdown.rs`, `delivery/tauri/shutdown.rs`, `lib.rs`                        | Estado revisionado, pedido único, eventos/consulta, confirmação e janela dedicada; saída interceptada quando o Tauri permite.                                                                                                                                                            | Fluxos básicos testados pelo usuário no macOS em modo de desenvolvimento; demais cenários pendentes.    |
| 2026-11-09 | `bootstrap/startup.rs`, `delivery/tauri/startup.rs`, `lifecycle.rs`                                                         | Aguardar supervisor do boot; impedir apresentação da principal e avanços de boot após pedido; registrar tarefas com falha/timeout como degradadas.                                                                                                                                       | Ainda não fecha atomicamente toda admissão de trabalho.                                                 |
| 2026-11-09 | `shutdown.html`, `src/core/shutdown/`, `src/components/features/shutdown/`, capability/permissão próprias, `vite.config.ts` | Apresentação independente e indeterminada, com tema e redução de movimento compartilhados com a splash.                                                                                                                                                                                  | Janela observada no macOS em modo de desenvolvimento; acessibilidade real e outros ambientes pendentes. |
| 2026-11-09 | `bootstrap/macos_shutdown.rs`, `bootstrap/shutdown.rs`, `lib.rs`                                                            | Encaminhar `applicationShouldTerminate:` do AppKit ao coordenador compartilhado. O delegate de Tao é preservado por subclasse sem ivars; se já houver implementação do seletor, a instalação falha explicitamente em vez de sobrescrevê-la. A saída autorizada retorna `NSTerminateNow`. | Dock e Cmd+Q validados pelo usuário no macOS em modo de desenvolvimento.                                |

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
leases antes da saída do processo. Clippy incluindo testes sem avisos;
execução da suíte Rust da biblioteca:
269 aprovados (uma execução anterior teve 1 falha não identificada, não
reproduzida na reexecução).

### Acompanhamento de leases de processamento

Os executores de thumbnails, cores e fingerprints agora aguardam a tarefa de
renovação e liberação do lease ao concluir o processamento, inclusive quando a
operação retorna erro. A falha de liberação é registrada e propagada ao
chamador. Testes do lease verificam espera pela conexão SQLite e propagação de
falha; os testes de processamento foram executados após a alteração.

Esse acompanhamento cobria o caminho de conclusão dos executores, não o abort
forçado: naquele estado o `Drop` disparava liberação assíncrona sem aguardar.
R4 substituiu esse fallback por abort solicitado ao heartbeat e recuperação da
reserva pela expiração, conforme a seção 31.
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

A instrumentação adicionada registrou a fase de operações de thumbnail e
fingerprint que continuavam após cancelamento. Os diagnósticos de segmentos
foram habilitados nas rodadas registradas; identificadores e caminhos podem
estar presentes nos logs históricos. Essa observabilidade não interrompe nem
comprova término do código nativo.

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

| Data                     | Comando / cenário                                                                 | Plataforma / ambiente                                                                                                                     | Resultado e evidência                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| ------------------------ | --------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2026-11-09               | verificação de tipos TypeScript                                                   | Ambiente de desenvolvimento macOS                                                                                                         | Aprovado.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 2026-11-09               | análise estática do frontend com ESLint                                           | Ambiente de desenvolvimento macOS                                                                                                         | Aprovado após formatação.                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 2026-11-09               | testes dos clientes de startup e encerramento                                     | Vitest                                                                                                                                    | 7 testes aprovados.                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 2026-11-09               | testes da tela de encerramento                                                    | Vitest/jsdom                                                                                                                              | 1 teste aprovado; não substitui leitor de tela real.                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 2026-11-09               | build do frontend com Vite                                                        | Vite                                                                                                                                      | Aprovado; `dist/shutdown.html` presente.                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 2026-11-09               | Prévia HTTP de `shutdown.html`                                                    | Navegador, sem runtime Tauri                                                                                                              | Layout e árvore acessível observados; estado de espera por IPC demonstrado. Não valida janela nativa nem a ponte Tauri.                                                                                                                                                                                                                                                                                                                                                                        |
| 2026-11-09               | verificação de formatação Rust e verificação de whitespace do diff                | Ambiente de desenvolvimento macOS                                                                                                         | Aprovados.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 2026-11-09               | execução dos testes do coordenador de encerramento                                | Ambiente de desenvolvimento macOS                                                                                                         | 4 testes aprovados, incluindo pedidos concorrentes e limite da confirmação.                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 2026-11-09               | execução dos testes de startup                                                    | Ambiente de desenvolvimento macOS                                                                                                         | 3 testes de regressão aprovados.                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 2026-11-09               | Fechamento pelo botão da titlebar                                                 | macOS, relato do usuário                                                                                                                  | Janela apareceu corretamente e duração foi considerada agradável; teste anterior à alteração de AppKit.                                                                                                                                                                                                                                                                                                                                                                                        |
| 2026-11-09               | Dock → Encerrar e Cmd+Q                                                           | macOS, relato do usuário                                                                                                                  | Antes da correção, janela não apareceu e a limpeza aparentemente não foi executada.                                                                                                                                                                                                                                                                                                                                                                                                            |
| 2026-11-09               | Titlebar, Dock → Encerrar e Cmd+Q, após correção                                  | macOS, execução do aplicativo em desenvolvimento; relato e saída de terminal fornecidos pelo usuário                                      | Janela exibida nos três casos; logs mostram um pedido de encerramento por execução, 12 handles registrados concluídos e `clean=true`. Dock e Cmd+Q também registram o pedido nativo do macOS.                                                                                                                                                                                                                                                                                                  |
| 2026-11-09               | Titlebar, Dock → Encerrar e Cmd+Q em build empacotado                             | macOS, bundle; relato do usuário, sem logs                                                                                                | Janela exibida corretamente com duração considerada agradável; encerramento concluído nos três casos.                                                                                                                                                                                                                                                                                                                                                                                          |
| 2026-11-09               | verificação de compilação da biblioteca Rust                                      | macOS                                                                                                                                     | Compilação da ponte AppKit aprovada.                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 2026-11-09               | execução dos testes de encerramento nativo macOS                                  | macOS                                                                                                                                     | Teste Objective-C de herança e recusa de sobrescrita aprovado; não simula Dock, Cmd+Q nem exibição da janela.                                                                                                                                                                                                                                                                                                                                                                                  |
| 2026-11-09               | Testes `feature::shutdown::tests` e `feature::startup::tests`                     | macOS                                                                                                                                     | 4 e 3 testes aprovados após integração AppKit.                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 2026-11-10               | Encerramento após alterações no lifecycle e pool                                  | macOS, execução do aplicativo em desenvolvimento; relato e saída colada pelo usuário                                                      | Usuário relata funcionamento correto. Logs mostram pedido nativo, 12 handles registrados concluídos e `clean=true`; servidor informa zero processos de segmento e zero sessões lineares. O meio de encerramento não foi especificado.                                                                                                                                                                                                                                                          |
| 2026-11-10               | Execução com reprodução de mídia e encerramento                                   | macOS, execução do aplicativo em desenvolvimento; logs colados pelo usuário                                                               | O servidor drenou 2 sessões lineares e 0 processos de segmento. Dois workers (`duplicate_worker`, `thumbnail_worker`) excederam o limite global de 5 s; resultado `clean=false`. Depois, três liberações de lease falharam porque o pool já estava fechado. Isso evidencia trabalho filho/desanexado fora do handle do worker; não é um encerramento limpo.                                                                                                                                    |
| 2026-11-10               | Build após correção para não fechar pool quando a drenagem falha                  | verificação de formatação Rust, verificação de compilação da biblioteca Rust, Clippy incluindo testes em macOS                            | Formatação, compilação e clippy concluídos sem avisos. O caminho degradado foi observado nos logs posteriores; ainda requer validação abrangente.                                                                                                                                                                                                                                                                                                                                              |
| 2026-11-10               | Novo encerramento com sessões lineares                                            | macOS, execução do aplicativo em desenvolvimento; logs fornecidos pelo usuário                                                            | Servidor relata 2 sessões lineares e 0 processos de segmento; `duplicate_worker` concluiu em aproximadamente 3,3 s, mas `thumbnail_worker` excedeu o prazo e foi abortado. Resultado `clean=false`; log confirma que o fechamento explícito do pool foi omitido. Não houve avisos de falha de release de lease nesta amostra.                                                                                                                                                                  |
| 2026-11-10               | Encerramento durante transcodificação de segmento                                 | macOS, execução do aplicativo em desenvolvimento; logs fornecidos pelo usuário                                                            | Gerenciador registrou cancelamento de 1 segmento; requisição registrou falha de FFmpeg e o servidor terminou. Os workers de thumbnails e duplicatas excederam o prazo e foram abortados; `clean=false` e fechamento explícito do pool omitido. Nenhuma falha de release de lease apareceu no log.                                                                                                                                                                                              |
| 2026-11-10               | Encerramento durante reprodução e processamento                                   | macOS, execução do aplicativo em desenvolvimento; logs fornecidos pelo usuário                                                            | Gerenciador relatou cancelamento de 1 segmento e limpeza de 3 sessões lineares; `duplicate_worker` concluiu, mas `thumbnail_worker` e `color_worker` excederam o prazo e foram abortados. Resultado `clean=false`; fechamento explícito do pool omitido. A finalização dos subprocessos não foi confirmada independentemente.                                                                                                                                                                  |
| 2026-11-10               | Testes de processamento e clippy após acompanhamento dos leases                   | macOS                                                                                                                                     | execução dos testes Rust com filtro `processing_`: 81 aprovados; execução dos testes Rust do módulo de processamento: 102 aprovados, incluindo espera por liberação de lease e falha do pool. Clippy incluindo testes sem avisos. Os testes não reproduzem o timeout nativo observado nos logs.                                                                                                                                                                                                |
| 2026-11-10               | Encerramento após acompanhamento de leases                                        | macOS, execução do aplicativo em desenvolvimento; logs fornecidos pelo usuário                                                            | Thumbnail informou 2 tarefas em drenagem e excedeu o orçamento; foi abortado e o resultado foi `clean=false`. Duplicatas e cores concluíram; fechamento explícito do pool foi omitido. Servidor relatou 1 segmento e 0 sessões lineares; a requisição registrou `SEGMENT_ERROR` após o cancelamento. Não houve falha de release de lease no log.                                                                                                                                               |
| 2026-11-10               | Nova execução com streaming e processamento                                       | macOS, execução do aplicativo em desenvolvimento; logs fornecidos pelo usuário                                                            | Uma tarefa de thumbnail em drenagem e extração nativa de cores concluíram; o worker de duplicatas excedeu 5 s e foi abortado. Servidor relatou limpeza de 2 sessões lineares e nenhum segmento; resultado `clean=false` e fechamento explícito do pool omitido. Ainda não havia identificação da fase de duplicatas nem confirmação detalhada da limpeza.                                                                                                                                      |
| 2026-11-10               | Testes após atualização da observabilidade                                        | macOS, execução da suíte Rust da biblioteca e Clippy incluindo testes                                                                     | 274 testes aprovados, 2 ignorados; clippy sem avisos. Entre os ignorados, o novo teste de espera por conexão adquirida falhou duas vezes quando executado anteriormente e requer investigação. Novas mensagens ainda não foram validadas no app em execução.                                                                                                                                                                                                                                   |
| 2026-11-10               | Encerramento durante reprodução de vídeo HLS, após atualização da observabilidade | macOS, execução do aplicativo em desenvolvimento; relato e logs fornecidos pelo usuário                                                   | HLS factory: 0 sessões rastreadas; servidor: 2 tentativas de cancelamento de segmentos, sendo uma com comando aceito e outra com retorno não zero. Sessões lineares: 0. Thumbnails e duplicatas excederam o prazo; `clean=false` e fechamento explícito do pool omitido.                                                                                                                                                                                                                       |
| 2026-11-10               | Instrumentação por fase e correção da remoção de segmento                         | macOS, execução da suíte Rust da biblioteca, testes de streaming e Clippy incluindo testes                                                | Suíte: 276 aprovados e 2 ignorados; testes de streaming após ajustes finais: 4 aprovados; clippy aprovado. O linker emitiu o aviso já registrado de versão mínima de macOS. Diagnósticos ainda precisam de validação no aplicativo.                                                                                                                                                                                                                                                            |
| 2026-11-10               | Execução com logs de fase e segmentos em DEBUG                                    | macOS, execução do aplicativo em desenvolvimento; logs fornecidos pelo usuário                                                            | Logs mostram segmentos transcodificados e servidos pelo cache. Após cancelamento, duas tarefas de thumbnail permaneceram em `thumbnail_extraction` e um fingerprint em `visual_hash` até o prazo; os dois workers foram abortados e `clean=false`. Limpeza linear informou 2 sessões, sem falhas relatadas; HLS factory e processos de segmento rastreados estavam vazios no shutdown.                                                                                                         |
| 2026-11-10               | Encerramento com processamento e reprodução ativos em perfil release              | macOS, desenvolvimento com backend release e logs de segmentos em DEBUG; logs fornecidos pelo usuário                                     | Backend Rust em `release`, frontend servido pelo Vite de desenvolvimento. Thumbnail informou 0 tarefas no `JoinSet` e terminou; duplicatas aguardou `fingerprint_batch`, com observação em `provider_extraction`, e concluiu antes do prazo. Os 12 handles registrados terminaram com `clean=true`; pool informou 20 conexões ociosas de 20 e fechamento retornou em 63 ms. Servidor registrou uma tentativa de cancelamento de segmento e limpeza de 3 sessões lineares sem falhas relatadas. |
| 2026-11-10               | Primeira indexação após zerar dados da aplicação                                  | macOS, `tauri dev` com backend `release`; logs e contexto fornecidos pelo usuário                                                         | `BatchCreate` registrou `database is locked` durante o scan. No encerramento, thumbnails, cores e duplicatas concluíram; 11 handles, `clean=true`. Pool informou 6 conexões, 5 ociosas, e retorno do fechamento em 17 ms.                                                                                                                                                                                                                                                                      |
| 2026-11-10               | Reinício, processamento e reprodução                                              | macOS, `tauri dev` com backend `release`; logs e contexto fornecidos pelo usuário                                                         | Boot scan falhou com `database is locked`. No encerramento, workers concluíram, mas `streaming_server` excedeu 5 s após iniciar drenagem HTTP e foi abortado; `clean=false` e fechamento explícito do pool omitido. Duas tentativas de cancelamento de segmento e uma sessão linear sem falhas de limpeza relatadas.                                                                                                                                                                           |
| 2026-11-10               | Nova execução com processamento e reprodução                                      | macOS, `tauri dev` com backend `release`; logs e contexto fornecidos pelo usuário                                                         | Boot scan concluiu sem erros relatados; thumbnails e duplicatas terminaram antes do prazo; 12 handles, `clean=true`. Uma tentativa de cancelamento de segmento e duas sessões lineares sem falhas relatadas. Pool informou 20 conexões, 19 ociosas, e retorno do fechamento em 52 ms.                                                                                                                                                                                                          |
| 2026-11-10               | Testes de streaming após tornar esperas lineares canceláveis                      | macOS, execução dos testes Rust de streaming, Clippy incluindo testes, verificação de formatação Rust e verificação de whitespace do diff | 7 testes aprovados, incluindo arquivo pronto, timeout, cancelamento e teste HTTP local que valida resposta 503 e término do servidor. Clippy e verificações de formatação aprovados. Não substitui teste do handler real com Tauri, player e FFmpeg.                                                                                                                                                                                                                                           |
| 2026-11-10               | Dados zerados, primeira indexação após alteração do streaming                     | macOS, `tauri dev` com backend `release`; logs fornecidos pelo usuário                                                                    | `BatchCreate` falhou com SQLite 517. Encerramento: 11 handles, `clean=true`; thumbnail e cores concluíram e fingerprint em `provider_extraction` concluiu; servidor terminou com 0 segmentos e sessões lineares registrados. Pool: 11 conexões, todas ociosas; fechamento retornou em 47 ms.                                                                                                                                                                                                   |
| 2026-11-10               | Segunda execução após alteração do streaming                                      | macOS, `tauri dev` com backend `release`; logs fornecidos pelo usuário                                                                    | Boot scan falhou com SQLite 5. Encerramento: 12 handles, `clean=true`; servidor terminou após duas tentativas de cancelamento de segmento, uma delas com comando não bem-sucedido, e limpeza de uma sessão linear sem falhas relatadas. Diagnóstico observou um handler `segment` ativo antes da parada. Pool: 9 conexões, 8 ociosas; fechamento retornou em 58 ms.                                                                                                                            |
| 2026-11-10               | Terceira execução após alteração do streaming                                     | macOS, `tauri dev` com backend `release`; logs fornecidos pelo usuário                                                                    | `Final BatchCreate` falhou com SQLite 517, seguido de scan que reportou `Errors: 0`. Encerramento: 12 handles, `clean=true`; servidor terminou após tentativa de cancelamento de um segmento e limpeza de duas sessões lineares sem falhas relatadas. Pool: 20 conexões, todas ociosas; fechamento retornou em 234 ms.                                                                                                                                                                         |
| 2026-11-10               | Regressão de snapshot SQLite e recuperação de scan                                | macOS, execução da suíte Rust da biblioteca, testes direcionados e Clippy incluindo testes                                                | Suíte: 282 aprovados e 2 ignorados antes do último ajuste no progresso; testes direcionados e clippy aprovados após o ajuste. Testes reproduziram SQLite 517 com WAL e transação diferida, verificaram lote concorrente com outro escritor e falhas injetadas de lote cheio e final seguidas de novo scan. Execução real no aplicativo continua pendente.                                                                                                                                      |
| Atualizado em 2026-11-18 | Matriz desktop                                                                    | macOS, Windows e Linux                                                                                                                    | Windows dev e evidências macOS preservados; sessão Linux concluída. Bundle Windows atualizado e smoke macOS final permanecem em R5/R7. Estado vigente nas seções 10.2 e 14.                                                                                                                                                                                                                                                                                                                    |

Na amostra fornecida, o intervalo entre o log do pedido e o log de conclusão
da drenagem registrada foi de aproximadamente 3,5 ms pela titlebar, 78,8 ms
pelo Dock e 3,3 ms por Cmd+Q. Esses tempos **não medem** duração da janela,
apresentação, término do processo ou desempenho sob carga. Não houve mensagens
de erro, timeout ou aviso na saída compartilhada. O `clean=true` refere-se aos
handles registrados, não comprova a finalização dos subprocessos, watchers
nativos ou operações não rastreadas. A versão do macOS e as condições exatas
de carga não foram informadas; o build empacotado no macOS foi validado apenas por relato, sem logs naquela rodada; a cobertura posterior consta da matriz consolidada.

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
verificação de whitespace do diff passaram.
Os dois testes ignorados incluem o teste do pool com conexão adquirida, cuja
limitação permanece aberta. A validação desktop parcial é descrita a seguir.

### Três execuções posteriores no macOS

O usuário executou `tauri dev -- --release`, com logs de segmentos em DEBUG,
e informou que a primeira execução usou pasta de aplicação limpa e que a
indexação e a árvore de pastas voltaram a funcionar.

| Execução       | Scan                                                                               | Encerramento observado                                                                                   |
| -------------- | ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| 1 — banco novo | Scan manual: 787 descobertos, nenhum inalterado, nenhum erro reportado, em 2,63 s. | `clean=true`; 11 tarefas registradas drenadas; aproximadamente 1,23 s entre a solicitação e a conclusão. |
| 2              | Boot scan: 14 descobertos, 773 inalterados, nenhum erro reportado, em 0,75 s.      | `clean=true`; 12 tarefas registradas drenadas; aproximadamente 2,22 s até a conclusão.                   |
| 3              | Boot scan: 14 descobertos, 773 inalterados, nenhum erro reportado, em 0,87 s.      | `clean=true`; 12 tarefas registradas drenadas; aproximadamente 1,37 s até a conclusão.                   |

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

A validação posterior manteve a pasta da aplicação e repetiu o scan sem
alterar os assets para verificar aliases Unicode e estabilidade de CREATE_ASSET.
Os resultados efetivos estão registrados a seguir; não há roteiro de terminal
pendente nesta subseção.

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

**Testes automatizados:** Clippy em release incluindo testes, com avisos tratados como erros
passou; os nove testes direcionados de streaming em release passaram no
macOS. Eles incluem cancelamento e espera de filho real, falha genuína,
requisição descartada, supersessão e recusa de admissão após shutdown.
verificação de formatação Rust e verificação de whitespace do diff passaram. O linker voltou a avisar
sobre a versão mínima de macOS do objeto `zstd`; não houve falha no teste.

A validação nativa dessa alteração foi realizada nas rodadas macOS abaixo,
seguida das evidências Windows e Linux posteriores. A presença dos eventos
de cancelamento/espera depende de haver processo ativo naquele instante.

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

**Validação no macOS:** verificação de formatação Rust, verificação de whitespace do diff e Clippy
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

| Entrada         | Tempo entre pedido e conclusão do backend | Sessões lineares | Limpeza linear | Pool antes do fechamento           |
| --------------- | ----------------------------------------: | ---------------: | -------------: | ---------------------------------- |
| Barra de título |                                    859 ms |                2 |         110 ms | 11/11 ociosas                      |
| Cmd+Q           |                                  1.312 ms |                3 |         433 ms | 19/19 ociosas após espera de 12 ms |
| Cmd+Q           |                                  4.206 ms |                6 |         545 ms | 9/9 ociosas                        |
| Dock            |                                  1.953 ms |                9 |       1.875 ms | 19/19 ociosas                      |

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
verificação de formatação Rust e verificação de whitespace do diff passaram. O linker emitiu o aviso
conhecido de objeto zstd compilado para macOS 13.1 em alvo 10.12; não houve
falha de compilação.

### Três novas execuções e supervisão do scan manual (12 de novembro)

Na ordem cronológica dos logs, os encerramentos foram pelo Dock, Cmd+Q e
botão da barra de título. Os três scans concluíram com 787 arquivos
inalterados, 14 aliases identificados e nenhuma descoberta, poda ou erro.
Todos os encerramentos registraram `clean=true`, sem `PoolClosed` ou
`database is locked`. O callback nativo do macOS apareceu nas duas
primeiras execuções, conforme esperado para Dock e Cmd+Q.

| Entrada         | Pedido até conclusão do backend | Limpeza linear                | Pool antes do fechamento |
| --------------- | ------------------------------: | ----------------------------- | ------------------------ |
| Dock            |                          496 ms | 4 sessões, 439 ms, sem falhas | 20/20 ociosas            |
| Cmd+Q           |                          318 ms | nenhuma sessão                | 20/20 ociosas            |
| Barra de título |                          130 ms | 2 sessões, 105 ms, sem falhas | 6/6 ociosas              |

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
verificação de formatação Rust e verificação de whitespace do diff passaram. Os novos testes verificam
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
`-D warnings`, verificação de formatação Rust e verificação de whitespace do diff passaram. O teste
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
verificação de formatação Rust e verificação de whitespace do diff passaram. Os testes cobrem a ordem
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
verificação de formatação Rust e verificação de whitespace do diff passaram. O teste novo verifica a
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
`lifecycle::shutdown_budget_tests`, verificação de formatação Rust e verificação de whitespace do diff
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

| Log      | Pedido até conclusão do backend | Tarefas registradas | Pool: conexões ociosas / total | Bridge: eventos / lag |
| -------- | ------------------------------: | ------------------: | -----------------------------: | --------------------: |
| `172148` |                           26 ms |                  10 |                          7 / 7 |                 0 / 0 |
| `175724` |                        2.968 ms |                  12 |                          7 / 7 |                 4 / 0 |
| `175939` |                          907 ms |                  11 |                          8 / 8 |                 1 / 0 |
| `180112` |                        1.174 ms |                  12 |                        20 / 20 |                 7 / 0 |
| `180334` |                        2.034 ms |                  11 |                        20 / 20 |                 5 / 0 |

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

| Log      | Pedido até conclusão do backend | Resultado     | Evidência relevante                                                                                       |
| -------- | ------------------------------: | ------------- | --------------------------------------------------------------------------------------------------------- |
| `211058` |                           36 ms | `clean=true`  | 10 tarefas registradas drenadas; pool fechado com 5 de 5 conexões ociosas.                                |
| `211107` |                           45 ms | `clean=true`  | 10 tarefas registradas drenadas; pool fechado com 7 de 7 conexões ociosas.                                |
| `211135` |                           70 ms | `clean=true`  | Scan em segundo plano parou após o pedido; 11 tarefas drenadas; pool fechado com 5 de 5 conexões ociosas. |
| `211219` |                        5.015 ms | `clean=false` | Worker de thumbnails excedeu o prazo e foi abortado; fechamento explícito do pool foi omitido.            |

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

| Log      |    Tempo | Resultado     | Evidência                                                                                                                                                                                              |
| -------- | -------: | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `232729` |    44 ms | `clean=true`  | Nenhum thumbnail pendente na drenagem; pool 5/5 ocioso e fechado.                                                                                                                                      |
| `234708` |    46 ms | `clean=true`  | Scan manual iniciado e encerrado cooperativamente; nenhum thumbnail pendente na drenagem; pool 6/6 ocioso e fechado. Não exercita o cancelamento de uma extração ativa.                                |
| `234816` | 5.013 ms | `clean=false` | Dois thumbnails em publicação ou reconciliação; worker de thumbnails, scan de inicialização, verificador de ferramentas e worker de duplicatas excederam o prazo. Pool não foi fechado explicitamente. |
| `235144` | 5.008 ms | `clean=false` | Dois thumbnails em extração de metadados; worker de thumbnails, scan de inicialização e servidor de streaming excederam o prazo. Pool não foi fechado explicitamente.                                  |

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

Na suíte focada de streaming, os 17 testes passaram no Windows, incluindo filhos de
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

| Log      | Evidência                                                                                                                                                                                                          | Limite da conclusão                                                                                                                                                                         |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `062259` | Segmentos de vídeo foram gerados e servidos do cache. Um segmento levou 15.531 ms; houve pedidos repetidos de playlist. Servidor de streaming drenou; boot scan e thumbnails excederam o prazo; `clean=false`.     | Produzir MPEG-TS não comprova reprodução pelo WebView. Os pedidos repetidos não identificam sua causa no frontend.                                                                          |
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

#### Cenários manuais preparados naquela rodada (histórico)

A comparação preparou vídeo e áudio locais e seus originais em rede, com
reprodução, seek e fechamento pelo X. Os resultados recebidos estão nas
subseções seguintes. Os comandos de terminal foram retirados desta revisão;
o relatório mantém apenas cenários e análises.

#### Verificação automatizada desta continuação

- execução dos testes Rust de streaming:
  17 testes aprovados no Windows.
- Testes de reprodução de vídeo, gerenciamento HLS e utilitários de streaming:
  9 testes aprovados na versão final, incluindo retorno de fonte nativa para HLS.
- verificação de tipos TypeScript: aprovado.
- build do frontend com Vite: aprovado na versão final; 2.753 módulos transformados.
- ESLint dos cinco arquivos TypeScript/TSX alterados: aprovado, sem warnings.
- Prettier dos cinco arquivos TypeScript/TSX alterados: aprovado.
- verificação de formatação Rust e verificação de whitespace do diff:
  aprovados após os ajustes de formatação.

O teste que falhou ao recolocar intencionalmente o retry antigo foi uma
verificação da regressão, não uma falha restante da versão final. Nenhum
teste automatizado substitui as três execuções do aplicativo descritas acima.

### Quatro cenários de rede e disco local: HLS e apresentação (13 de novembro)

Os quatro arquivos recebidos têm o mesmo prefixo `mundam-streaming-video-local`,
mas o cenário abaixo segue o relato do usuário e os caminhos nos logs, não o
rótulo do arquivo. O compartilhamento está em um MacBook conectado por Wi-Fi.

| Sufixo do log | Cenário relatado                                                             | Resultado do backend                                                                                                                         |
| ------------- | ---------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `120851`      | Adição de pasta em rede, processamento e reprodução de vídeos.               | Scan: 787 descobertos em 92,61 s. Shutdown: 5.018 ms, `clean=false`; thumbnails e duplicatas excederam o prazo.                              |
| `121941`      | Áudios em rede; alguns tocaram após nova tentativa.                          | Shutdown: 5.005 ms, `clean=false`; thumbnails excederam o prazo.                                                                             |
| `122433`      | Banco/pastas da aplicação recriados pelo usuário; biblioteca local e vídeos. | Scan: 783 descobertos em 2,01 s. Shutdown: 1.826 ms, `clean=true`; pool 6/6 ocioso e fechado. HLS continuou sem reproduzir segundo o relato. |
| `123117`      | Áudios locais reproduziram; mídia continuou audível durante o fechamento.    | Scan de boot: 783 inalterados em 1,55 s. Shutdown: 2.643 ms, `clean=true`; pool 15/15 ocioso e fechado.                                      |

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

#### Verificação e cenários posteriores (histórico)

Os sete arquivos de teste focados do frontend passaram: 19 testes, incluindo
seleção de engine por plataforma, passagem pelo gerenciador mesmo com suporte
nativo anunciado, preview com probe lento/falho, pausa de mídia e descarte dos
listeners. Typecheck, ESLint dos arquivos alterados e build Vite foram aprovados.
Os dois testes novos da playlist também passaram em compilação isolada do
módulo de playlist com o compilador Rust.

Na compilação completa do aplicativo, a suíte Rust de streaming passou com 19 testes, e
a suíte Rust de encerramento passou com
23 testes. As suítes têm testes em comum; as contagens não representam 42
testes distintos. Nenhum teste foi ignorado nessas duas execuções.

A validação real posterior contemplou vídeo local, áudio local e rede,
com reprodução, seek, qualidade e fechamento durante mídia ativa, conforme
os resultados da subseção seguinte.

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

| Sufixo do log | Cenário informado                                         | Pedido até conclusão do backend | Resultado e pool SQLite                                       |
| ------------- | --------------------------------------------------------- | ------------------------------: | ------------------------------------------------------------- |
| `134821`      | Vídeos locais; fechamento com MPG linear tocando.         |                        1.818 ms | `clean=true`; 19/19 conexões ociosas; pool fechado em 117 ms. |
| `141611`      | Áudios locais; fechamento com WMA tocando.                |                           90 ms | `clean=true`; 17/17 conexões ociosas; pool fechado em 79 ms.  |
| `144512`      | Biblioteca em rede por cabo, após recriação pelo usuário. |                        2.116 ms | `clean=true`; 7/7 conexões ociosas; pool fechado em 61 ms.    |

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
em rede. Isso não fecha a matriz de bundle, inicialização parcial ou plataformas
restantes. Acessibilidade foi dispensada como requisito de conclusão na revisão
seguinte. Nenhuma alteração de código foi necessária nesta
rodada; foi atualizada a documentação. Não foram solicitados novos testes de
suporte a formatos, thumbnails ou desempenho de rede.

## 14. Pendências gerais após a conclusão Linux

**Estado consolidado em 19 de novembro:** a sessão Linux está concluída e
aceita pelo usuário (R6). A documentação foi reconciliada com as rodadas de
17–18 de novembro e com o checkout `a1af219`, acrescido da implementação R1
das seções 24–25, validada na seção 26, e de R2, implementado/auditado na seção 27
e validado na seção 28. R3 foi implementado e verificado automaticamente na
seção 29, com integração Windows validada na seção 30. R4 foi implementado e
verificado automaticamente na seção 31, com integração Windows validada na
seção 32. R1–R4 estão concluídos nos escopos registrados. Essa etapa não deriva
dos erros intermitentes não atribuídos da máquina Linux.
A conclusão funcional da sessão Linux não é uma afirmação de drenagem universal.

### Implementação e auditoria: estado dos itens

| ID  | Trabalho restante                                                                                          | Evidência e critério de conclusão                                                                                                                                                                                                                                                                                                                                 |
| --- | ---------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | **Concluído nos cenários dirigidos.**                                                                      | Implementação e correção do registro duplicado validadas em duas execuções Windows com backend release (seção 26): callback nativo liberado, loops aguardados, library_watcher concluído antes do bridge, SQLite fechado explicitamente e clean=true. Não equivale ao aceite dos demais donos ou do bundle final.                                                 |
| R2  | **Concluído nos caminhos e cenários registrados.** | Implementação/auditoria e 34 testes aprovados na seção 27; dois encerramentos Windows com backend release, HLS e linear, auxiliares HTTP concluídos, IPC ocioso limpo, bridge após produtores e SQLite fechado com clean=true (seção 28). Filhos IPC ativos cobertos automaticamente, incluindo FFmpeg real. Limites nativos em R4; admissão geral de R3 validada na seção 30. |
| R3  | **Concluído nos caminhos e cenários registrados.** | 98 testes Rust e 24 de frontend aprovados (seção 29); duas saídas Windows clean=true, IPC drenado antes do bridge/pool e recusa real de get_processing_snapshot após fechamento do banco (seção 30). Três scans manuais registrados e aguardados, já concluídos antes do X; concorrência ativa e transação retida cobertas automaticamente. |
| R4  | **Concluído nos caminhos e cenários registrados.** | 113 testes Rust, Clippy e política de boot/I/O/recuperação na seção 31. Três saídas Windows clean=true, scan/workers ativos aguardados, nova raiz retomada e uma chamada IPC ativa drenada antes do bridge/pool (seção 32). Backend já pronto ao fechar pela splash; fases iniciais, deadline, abort e expiração de leases cobertos automaticamente. |

### Validação e encerramento restantes

| ID  | Estado           | Trabalho restante ou decisão                                                                                                                                                                                                                                                                                                     |
| --- | ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R5  | **Concluído nos cenários registrados** | Quatro encerramentos iniciais instalados (seção 34) e reteste do pacote CORS corrigido (seções 35–36). Usuário confirmou funcionamento correto; mídia com avanço de quadros, sessão linear ativa limpa, IPC e workers drenados antes do pool, saída zero e clean=true em 1.469 ms. Avisos/falhas de segmentos preservados, sem reabrir suporte a formatos ou exigir instalação MSI separada. |
| R6  | **Concluído**    | Linux testado em desenvolvimento com backend release, binário direto, Debian instalado e AppImage. Preferências persistentes, mídia e encerramentos sob processamento foram validados. Aceite do usuário em 18/11 com observações intermitentes transferidas para notas futuras. Não há reteste Linux obrigatório nesta revisão. |
| R7  | Pendente         | Smoke macOS do código compartilhado final, preservando o aceite anterior. Cobrir mídia/pausa/saída e os caminhos alterados desde a validação anterior, sem repetir suporte a todos os formatos.                                                                                                                                  |
| R8  | Decisão pendente | Definir coleta ou dispensa das medições gerais ainda previstas: série fria/quente, consumo e comparação com baseline. Tempos amostrais já constam do relatório. A ausência da série Linux não reabre a sessão aceita; não se registra como coletada uma série inexistente.                                                       |
| R9  | Parcial | R1–R5 concluídos nos escopos registrados; relatório, tabelas, inventário e checklist atualizados. Restam R7/R8, verificações finais de integridade/recuperação e aceite geral do responsável, distinto dos aceites Windows/Linux já registrados. |

### Fora da lista de pendências

- Acessibilidade: dispensada como requisito de conclusão.
- Correções Linux de mídia HTTP/classificação HLS, manutenção de banco,
  resiliência do cliente de apresentação/IPC e preferências de vídeo/gráfica:
  implementadas e validadas nos cenários registrados.
- Corrupção de heap, timeouts intermitentes sob pressão, WebProcess remanescente,
  foco inicial e avisos de runtime sem causa atribuída ao produto: somente
  notas futuras na seção 23, por decisão do usuário; sem bloqueio Linux.
- Formatos/thumbnails conhecidos, MP2, otimização de rede e prompts sem impacto
  comprovado no fechamento: fora do escopo. A ampliação Linux autorizada já foi
  concluída, sem obrigação de uma auditoria universal de formatos.
- Outros hosts Linux, instalação limpa, updater, auditoria de licenças,
  compatibilidade com distribuições antigas e regeneração Debian com a última
  preferência gráfica: limites do que foi testado, sem nova tarefa imposta
  para encerrar esta sessão.
- Logout/terminação forçada: sem garantia de interceptação universal; somente
  reconsiderar se houver requisito específico futuro.

### Aceite

**Linux:** concluído e aceito pelo usuário em 18 de novembro de 2026, nos
cenários e condições descritos, incluindo as compatibilidades apropriadas ao
host. Os testes adicionais feitos pelo usuário fora da coleta foram relatados,
sem logs novos analisados nesta revisão. Sua avaliação de provável problema
no computador/Ubuntu fundamenta a exclusão de escopo, não um diagnóstico
forense de causa comprovada.

**Geral da implementação:** permanece em R9, pelos itens de implementação e
validação explicitamente mantidos acima. Não há aprovação pendente da sessão
Linux. Eventuais regressões dirigidas após futuras alterações compartilhadas
serão definidas com essas alterações e não equivalem a reabrir esta sessão.

## 15. Validação Linux: encerramento ocioso (17 de novembro)

**Estado na revisão de 18/11:** Rodadas históricas de desenvolvimento; sessão Linux concluída em 18/11. A cobertura final está nas seções 10.2 e 23.

As duas execuções recebidas usaram o commit `146eb8b`, em Ubuntu 26.04.1
LTS, kernel `7.0.0-43-generic`, x86_64, GNOME (`ubuntu:GNOME`) e sessão
Wayland. O modo foi `tauri dev --release`: backend otimizado e frontend
servido pelo Vite, sem bundle. A sessão não comprova qual backend gráfico
o WebView utiliza. A única alteração local registrada era este relatório.

O usuário confirmou a sequência visual splash → principal → apresentação de
finalização, com comportamento considerado esperado. Os dois arquivos têm
rótulo `ocioso-x`; os logs não distinguem a origem nativa do fechamento.
Na confirmação posterior, o usuário identificou o primeiro fechamento como X
da titlebar e o segundo como Alt+F4; os dois caminhos estão validados para
o cenário ocioso observado, com origem atribuída pelo relato.

| Log                            | Pedido até conclusão do backend | Pedido até handshake | Tarefas | Pool ocioso / total; fechamento | Resultado                               |
| ------------------------------ | ------------------------------: | -------------------: | ------: | ------------------------------- | --------------------------------------- |
| `ocioso-x-20261117-090803.log` |                        181,8 ms |             729,4 ms |      10 | 6/6; 180 ms                     | `clean=true`; comando Tauri retornou 0. |
| `ocioso-x-20261117-092345.log` |                        208,4 ms |             937,6 ms |      10 | 7/7; 205 ms                     | `clean=true`; comando Tauri retornou 0. |

Cada execução registrou um pedido e uma conclusão, sem timeout, abort,
`PoolClosed` ou bloqueio SQLite. O servidor HTTP parou, os proprietários de
processos registrados terminaram e os handlers do protocolo de assets
drenaram antes do fechamento do pool. O bridge terminou sem lag e sem eventos
recebidos durante a drenagem. Os handshakes foram pontuais (`timely=true`),
seguidos do registro de ocultação das janelas anteriores. A confirmação
visual vem do relato do usuário; o handshake não mede pixels visíveis.
Os tempos da tabela não medem duração da janela nem instante de saída final.

As bibliotecas tinham zero raízes observadas. Não havia tarefas de thumbnail
em andamento, elementos de mídia a pausar, sessões lineares, sessões HLS
rastreadas ou processos a cancelar no shutdown. As listagens antes e depois
não encontraram processos com os nomes exatos `mundam`, `ffmpeg` e `ffprobe`.
Isso reforça o resultado ocioso, mas não constitui inventário universal de
filhos, processos com outros nomes ou recursos nativos. Não foram exercitados
watcher com raiz ativa, reprodução, indexação, recuperação de trabalho pendente
ou corrida de admissão.

### Avisos e limites da rodada

Ambos os logs contêm aviso do clipboard: o protocolo Wayland de data control
exigido pelo arboard não está disponível no compositor, e o runtime informa
fallback para o protocolo X11 de clipboard. O aviso ocorreu no startup e não
foi acompanhado de falha de encerramento nestas amostras. O funcionamento do
clipboard não foi testado; o fallback não comprova uso de X11 pelo WebView.

O Cargo informou dependência `urlencoding` não utilizada. O primeiro build
levou 13 min 47 s, e o segundo 25,44 s; esses valores incluem compilação e
não representam latência de startup ou shutdown. Nenhum desses avisos
justifica mudança no fechamento com base nos resultados recebidos.

**Resultado:** validação inicial positiva do fluxo visual e do encerramento
ocioso em desenvolvimento neste ambiente Linux. Os casos de carga, entradas
complementares e bundle foram executados nas rodadas seguintes; R6 foi
concluído em 18/11. R1–R4 permanecem apenas como lacunas compartilhadas da seção 14.

Na preparação, os cinco testes frontend focados de shutdown passaram em três
arquivos. A suíte Rust filtrada por `shutdown` foi iniciada, mas seu resultado
final não foi recuperado; ela não está registrada como aprovada. Esta rodada
alterou apenas documentação, sem correção de código.

### Processamento e tentativas de mídia (17 de novembro)

O usuário seguiu os cenários de processamento, vídeo local e áudio com
Alt+F4. Informou que nenhum vídeo ou áudio reproduziu; as mensagens incluíam
“failed to load media” e “HLS is not supported in this browser”. Assim,
os testes exercitaram tentativas de reprodução e fechamento sob processamento,
mas não validaram pausa audível ou drenagem de reprodução efetivamente ativa.

| Log                                 | Cenário / resultado                                                       | Pedido até conclusão do backend | Evidência                                                                                                                                                                             |
| ----------------------------------- | ------------------------------------------------------------------------- | ------------------------------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `processamento-20261117-093303.log` | X durante processamento; `clean=true`, saída do comando 0.                |                        846,7 ms | Scan manual parou após cancelamento; dois thumbnails em admissão/lease foram adiados. 11 tarefas concluíram; pool 8/8 ocioso, fechamento em 616 ms. Handshake pontual aos 1.362,3 ms. |
| `video-local-20261117-093519.log`   | Tentativas de vídeo, fechamento pelo X; `clean=true`, saída do comando 0. |                      1.311,4 ms | Dois thumbnails ativos em publicação/reconciliação concluíram. 11 tarefas; pool 8/8 ocioso após espera de 16 ms, fechamento em 519 ms. Não há handshake registrado.                   |
| `audio-alt-f4-20261117-094530.log`  | Execução interrompida externamente; usuário relatou código 9.             |                   Não aplicável | Log termina sem pedido/conclusão de shutdown ou código final do comando. Não é evidência de execução do coordenador.                                                                  |
| `audio-alt-f4-20261117-095151.log`  | Nova tentativa de áudio, Alt+F4; `clean=false`, saída do comando 0.       |                      5.003,0 ms | Thumbnail excedeu o orçamento na fase `publication`; handle abortado. Pool explicitamente não fechado devido à drenagem incompleta.                                                   |

Nos três fechamentos coordenados, o servidor e seus proprietários registrados
terminaram, sem sessões lineares nem processos a cancelar; os protocolos
fecharam com `active=0` e drenaram. As listagens finais não mostraram processos
com os nomes observados. O frontend registrou pausa de zero, dois e dois
elementos, respectivamente; existência de elementos não comprova reprodução.
Os bridges terminaram sem lag; receberam zero, três e um eventos na drenagem.
Não houve erro `PoolClosed` ou bloqueio SQLite nestas amostras. Os tempos
não medem duração visual ou saída final do processo.

O scan do vídeo reconheceu 400 registros e descobriu outros 387, totalizando
787, sem poda ou erros. Os dois boots de áudio encontraram os 787 inalterados,
sem poda ou erros. É evidência de persistência/reabertura do catálogo observado,
não de conclusão de todo processamento pendente ou recuperação universal.

#### Interrupção externa e pressão de memória

Consulta ao journal de `systemd-oomd` encontrou, às 09:50:03 locais, término
de um scope `ptyxis-spawn` por pressão de memória do grupo do usuário superior
a 50% por mais de 20 s (78,43% no registro). O scope candidato tinha 3,7 GB
em uso. Horário e tipo de scope são compatíveis com a interrupção do primeiro
áudio relatada pelo usuário. Não há identificação individual do Mundam no
registro consultado, nem prova de qual processo causou a pressão. A ausência
de registro no journal do kernel não exclui uma decisão do oomd em userspace.
Não é possível atribuir a falha a vazamento específico a partir destes dados.

#### Dependências de reprodução verificadas

A inspeção local encontrou WebKitGTK 2.52.6 e GStreamer 1.28.2, com os
pacotes `gstreamer1.0-libav`, `gstreamer1.0-plugins-bad` e
`gstreamer1.0-plugins-ugly` não instalados. Não foram encontrados os elementos
`avdec_h264`, `avdec_aac`, `h264parse` ou `tsdemux`; `hlsdemux2` e `qtdemux`
estavam disponíveis. WebKitGTK utiliza GStreamer para mídia, conforme sua
[documentação oficial](https://docs.webkit.org/Ports/WebKitGTK%20and%20WPE%20WebKit/Multimedia.html).

Essa ausência fornece uma explicação concreta plausível para parte das falhas,
inclusive fluxos HLS com H.264/AAC. Não comprova que todos os erros tenham a
mesma causa. No código, a recusa de HLS ocorre quando não há suporte nativo
anunciado nem suporte detectado pelo hls.js; a mensagem não distingue sozinha
codecs ausentes de outras limitações do runtime. O FFmpeg utilizado no backend
não substitui os plugins do GStreamer utilizados pelo WebView. Nenhum pacote
foi instalado nesta análise e a reprodução após instalação não foi validada.

#### Resultado degradado no segundo áudio (nota histórica)

O thumbnail ativo usava `WINDOWS_MEDIA_VIDEO_PROVIDER`, asset
`298cd052-cee7-4ea2-9136-2a34f823106a`. Permaneceu em `publication` durante
os cinco segundos. No código atual, essa fase abrange verificação de tentativa,
publicação do thumbnail e commit correspondente; o diagnóstico não identifica
qual suboperação reteve a tarefa. Não há evidência para atribuí-la à ausência
de codecs no WebView ou à pressão de memória da execução anterior. Cores e
duplicatas terminaram antes do prazo, e o bridge drenou após o abort do worker.
O retorno 0 do comando não converte o resultado `clean=false` em aceite limpo.
O resultado degradado desta amostra fica preservado como nota futura, sem
pendência R2/R4 criada por sua causa não identificada, conforme aceite de 18/11.
As lacunas concretas de ownership são enumeradas separadamente na seção 14.

### Mídia após disponibilizar codecs e nova lacuna de ownership (17 de novembro)

Nos logs `video-codecs-20261117-101610.log` e
`audio-codecs-20261117-102330.log`, `avdec_h264`, `avdec_aac`, `h264parse` e
`tsdemux` estavam disponíveis. O usuário relatou erro de formato no MP4,
vídeo HLS com tela preta e falha de mídia nos áudios nativos como MP3.
O áudio HLS passou a avançar e carregar waveforms. Não houve confirmação
acústica: o usuário informou ausência de som também no YouTube pelo navegador.
Isso caracteriza uma limitação do teste de saída de áudio do sistema;
não demonstra que todo erro de reprodução do Mundam tenha essa origem.
Waveform e avanço visual não comprovam sozinhos som audível ou imagem decodificada.

| Log                                | Pedido até conclusão do backend | Resultado de encerramento                                                                                   |
| ---------------------------------- | ------------------------------: | ----------------------------------------------------------------------------------------------------------- |
| `video-codecs-20261117-101610.log` |                      4.261,2 ms | Tarefas registradas limpas, mas pool SQLite excedeu 3 s; resultado final `clean=false`. Comando retornou 0. |
| `audio-codecs-20261117-102330.log` |                        541,7 ms | `clean=true`; 11 tarefas concluíram, pool 18/18 ocioso e fechado em 540 ms. Comando retornou 0.             |

No vídeo, um processo de segmento recebeu cancelamento por `Shutdown`, foi
aguardado e o servidor parou. A admissão de assets fechou sem handlers ativos;
os handlers drenaram. Duplicatas concluiu sua extração em andamento; nenhum
worker excedeu o orçamento. O snapshot anterior ao fechamento do pool era
12/12 ocioso, mas `close` excedeu o limite. Após a conclusão degradada,
`bootstrap::database` registrou `Library cleanup remains pending` com tentativa
de adquirir conexão em pool fechado. Isso confirma trabalho de manutenção
fora da drenagem; não prova que esse trabalho causou o timeout do pool.

No áudio, uma sessão linear cujo processo já havia saído foi limpa sem falhas,
em 1 ms. O servidor e os proprietários registrados terminaram. Nos dois
encerramentos foram pausados dois elementos de mídia; a telemetria chegou
aproximadamente 17,4 ms e 38,9 ms após o pedido. Não há handshake de apresentação
registrado nestes logs. As listagens finais não mostram processos com os nomes
observados; não constituem inventário universal de recursos.

#### Reprodução: evidência e delimitação

Os dois logs mostram uso de MediaSource HLS e manifestos processados pelo
hls.js. No vídeo há respostas 503 para segmentos antes do shutdown, próximas
a cancelamentos por `Superseded`, além de `bufferStalledError` e
`bufferSeekOverHole`. Portanto, há falhas de carregamento antes do fechamento;
não é correto atribuir todas as respostas 503 à drenagem. Segmentos também
foram gerados com sucesso e chegaram a existir elementos com `readyState=4`;
isso não comprova imagem apresentada, conforme a tela preta relatada.
O registro atual não determina a causa completa dos erros nativos nem da
imagem preta. Não foi aplicada mudança de renderer, driver ou configuração
gráfica nessa rodada. A indisponibilidade acústica foi superada após o ajuste
do dispositivo de saída relatado na subseção seguinte.

#### Correção compartilhada de manutenção do banco

A inspeção de `bootstrap/database.rs` confirmou que a rotina periódica de
`retry_cleanup` tinha handle descartado e loop sem cancelamento; a recuperação
de sagas também era destacada. Ambas passaram à admissão atômica com
`LifecycleRegistry::spawn_unique`, sob os nomes `library_cleanup` e
`saga_recovery`. A limpeza observa o cancelamento antes de iniciar uma rodada,
aguarda uma rodada já iniciada e não agenda outra após seu retorno.
A recuperação admitida é aguardada até concluir, sujeita ao orçamento existente.
Assim, o fechamento explícito do pool depende também desses handles; se não
drenarem, o resultado permanece degradado. Isso elimina esses dois donos
específicos do conjunto não supervisionado, sem encerrar R2/R3 integralmente.
Não comprova a correção de toda causa possível do timeout do pool, do timeout
anterior em `publication` ou da pressão de memória.

Foram adicionadas regressões para impedir limpeza após cancelamento prévio e
para verificar que o shutdown aguarda uma limpeza em andamento antes de retornar,
sem executar uma segunda rodada. A formatação Rust e verificação de whitespace do diff passaram.

**Validação automatizada recebida em 17 de novembro:** o log
`mundam-database-tasks-tests.log` confirma compilação em perfil de testes
(`unoptimized + debuginfo`) em 5 min 44 s e os dois testes aprovados, sem falhas
ou ignorados; 326 testes ficaram fora do filtro. O teste de cancelamento prévio
confirma ausência de nova limpeza; o teste com lifecycle confirma espera pela
rodada em andamento e ausência de uma segunda execução após cancelamento.
Os testes usam uma operação controlada, sem executar a limpeza real de arquivos
ou a recuperação completa de sagas. Não comprovam a resolução do timeout do
pool em runtime. Os logs de mídia acima precedem a alteração; a validação
desktop posterior da manutenção está registrada na subseção seguinte.

### Validação runtime da manutenção e confirmação acústica (17 de novembro)

Foi analisado `manutencao-20261117-141654.log`, localizado na pasta de logs
Linux. O cabeçalho registra `146eb8b` com as alterações locais de manutenção
e testes descritas acima, em `tauri dev --release`. O backend permaneceu
ativo por aproximadamente 19 min 27 s entre o registro da manutenção e o
pedido de saída, ultrapassando o intervalo de 60 s da rotina periódica.
O log não conta as rodadas efetivamente executadas.

O usuário resolveu a ausência de som selecionando o dispositivo de saída
correto. Confirmou som e waveforms para AAC, OGG, MIDI, AC3, AIF, APE, M4R,
M4A, RA e WMA. MP3, FLAC e WAV continuaram falhando com “Failed to load media”.
Nos vídeos 3GP, 3G2, F4V, M2TS, MKV e AVI, o áudio e o seek funcionaram via
HLS, mas a imagem permaneceu preta. FLV, HEVC e SWF no caminho linear tiveram
comportamento semelhante. MP4 e MOV falharam com “error loading video format”.
A resolução do dispositivo de som elimina a indisponibilidade acústica desta
rodada; não resolve nem explica por si os erros nativos ou a imagem preta.

O usuário deixou `sample_1280x720_surfing_with_audio.f4v` tocar até o fim,
relatou mudança para 1,5x e 2x após cerca de 1 min 30 s e alteração de qualidade.
Isso confirma interação e reprodução audível observadas, sem comprovar imagem
correta. O log contém manifestos HLS processados e também stalls, saltos sobre
lacunas de buffer e erros 503 anteriores ao shutdown; não caracteriza todos
os formatos como plenamente suportados. As falhas de reprodução permanecem
registradas separadamente da validação de fechamento.

**Encerramento por Alt+F4, conforme relato:** um pedido às 17:50:39.886 UTC e
conclusão do backend às 17:50:40.784 UTC, aproximadamente 897,8 ms, com
`clean=true`. As 13 tarefas registradas concluíram, incluindo `library_cleanup`
e `saga_recovery`. A limpeza periódica confirmou parada aproximadamente
9,8 ms após o pedido e foi aguardada antes do pool. A recuperação não encontrou
sagas pendentes neste boot; o cenário de recuperação efetiva não foi exercitado.

O servidor terminou após limpar três sessões lineares, duas com processos
ativos e uma com processo já encerrado. A limpeza levou 156 ms, com 146 ms
acumulados esperando processos e 8 ms removendo diretórios; nenhuma falha de
processo ou diretório foi reportada. Os proprietários de processo registrados
terminaram, os handlers de assets drenaram e o pool fechou em 735 ms após
snapshot 9/9 ocioso. Não houve timeout, abort ou nova tentativa de manutenção
sobre pool fechado. O comando retornou 0, e a listagem final não mostrou
processos com os nomes observados.

A telemetria registrou pausa de um elemento cerca de 177,1 ms após o pedido.
Um erro de fragmento com status 0 foi registrado após a parada do servidor;
essa proximidade é compatível com uma tentativa do player durante a saída,
sem demonstrar sua origem individual. Não há handshake de apresentação
registrado nesta amostra; os tempos não medem visibilidade ou saída final.

**Resultado:** evidência positiva da correção dos dois donos de manutenção
neste cenário Linux, sem repetição do timeout do pool. Uma saída limpa não
elimina os outros caminhos de admissão não auditados, o timeout anterior de
thumbnail em `publication`, os casos restantes de boot/janelas ou a validação
de bundle. Nenhuma mudança adicional de código foi feita nesta rodada.

### Splash e pedidos repetidos no Linux (17 de novembro)

O usuário confirmou execução dos passos de fechamento durante a splash e de
Alt+F4 repetido. Ambos os logs registram o checkout `146eb8b` com as alterações
locais de manutenção, em `tauri dev --release`.

| Log                                     | Pedido até conclusão do backend | Tarefas / pool                                                       | Resultado                         |
| --------------------------------------- | ------------------------------: | -------------------------------------------------------------------- | --------------------------------- |
| `boot-interrompido-20261117-145428.log` |                        396,0 ms | 13 tarefas; pool 7/7 ocioso após espera de 10 ms, fechado em 381 ms. | `clean=true`; comando retornou 0. |
| `pedidos-repetidos-20261117-145720.log` |                        184,8 ms | 13 tarefas; pool 12/12 ocioso, fechado em 181 ms.                    | `clean=true`; comando retornou 0. |

Nos dois, manutenção e recuperação foram aguardadas, o servidor parou, os
handlers de assets drenaram e o bridge terminou sem lag. Não havia sessões
lineares, processos a cancelar ou thumbnails pendentes no instante da saída.
As listagens finais não mostraram processos com os nomes observados. Não houve
timeout, abort ou acesso da manutenção ao pool fechado.

**Momento do primeiro pedido:** `BackendReady` ocorreu às 17:55:24.619 UTC,
o scan terminou às 17:55:25.131 e o fechamento foi solicitado às
17:55:27.624. Conforme o procedimento confirmado pelo usuário, a ação ocorreu
na splash; o backend já estava pronto, enquanto o frontend ainda carregava.
Esse caso cobre fechamento na apresentação de startup, mas não cancelamento
antes do banco, durante a aquisição de serviços ou com scan ativo. Não há
handshake de shutdown registrado nem evento que comprove visualmente a ausência
de reapresentação da principal após o pedido.

Após a conclusão limpa do backend, às 14:55:29–30 locais, o Vite/esbuild
registrou erros `write EPIPE` e serviço parado para módulos ainda em
transformação. A sequência é compatível com interrupção do servidor de
desenvolvimento durante carregamento do frontend; a causa exata não foi
isolada. Não houve falha de drenagem Rust ou código final diferente de zero.
Essa observação de desenvolvimento não valida o comportamento do bundle.

**Pedidos repetidos:** há um único registro de pedido, uma drenagem e uma
conclusão. A confirmação de apresentação ocorreu aos 828,1 ms após o pedido,
com `timely=true`, e a pausa de mídia informou zero elementos. O resultado é
compatível com a idempotência esperada sob Alt+F4 repetido, conforme o relato;
o log não conta quantos eventos nativos adicionais chegaram ao coordenador.
O usuário confirmou posteriormente que apareceu apenas uma tela de fechamento
no segundo cenário, acrescentando a evidência visual de apresentação única.

**Resultado:** os dois fluxos observados encerraram normalmente. O caso de
splash com backend pronto e a execução de pedidos repetidos acrescentam
cobertura nativa Linux. As falhas injetadas de apresentação/IPC foram validadas
na seção 16; marcos anteriores do boot continuam delimitados por R4, sem
bloqueio da sessão Linux aceita. Nenhuma correção de código foi necessária
nesta rodada.

### Principal minimizada e saída pelo dock (17 de novembro)

No teste `minimizada-20261117-150040.log`, em `tauri dev --release`, o usuário
minimizou a principal e selecionou Sair no menu de contexto do ícone do dock.
Confirmou que a apresentação de finalização apareceu rapidamente e o aplicativo
terminou. A origem do pedido e a visibilidade são atribuídas ao relato; o log
confirma a execução do coordenador.

Houve um único pedido às 18:01:41.354 UTC e conclusão do backend às
18:01:41.495, aproximadamente 141,1 ms, com `clean=true`. As 13 tarefas
registradas concluíram, incluindo manutenção e recuperação; servidor,
proprietários de processos registrados, handlers de assets e bridge drenaram.
O pool tinha 10/10 conexões ociosas e fechou em 138 ms. A confirmação da
apresentação chegou aos 761,8 ms após o pedido, com `timely=true` e registro
de ocultação das janelas anteriores. Esses tempos não medem duração visível.

Não havia thumbnails pendentes, sessões lineares, processos a cancelar ou
mídia a pausar. Não houve timeout, abort, erro de pool fechado ou lag do bridge.
O comando retornou 0; a listagem final não mostrou processos com os nomes
observados. O boot reconheceu 787 arquivos inalterados, sem descobertas, poda
ou erros. Permaneceram os avisos já conhecidos de clipboard e dependência
`urlencoding` não utilizada, sem falha de encerramento associada nesta amostra.

**Resultado:** fechamento com principal minimizada e saída pelo dock validado
neste ambiente Linux em desenvolvimento, com confirmação visual de apresentação.
O teste não simula falha de criação da janela/IPC e não cobre bundle ou toda
sessão gráfica Linux. Nenhuma alteração de código foi necessária nesta rodada.

## 16. Mídia nativa Linux e resiliência da apresentação/IPC (17 de novembro)

**Estado na revisão de 18/11:** Correções de mídia e resiliência implementadas e validadas nas rodadas abaixo; não há pendência atual desses testes.

**Ampliação autorizada de escopo:** o usuário incluiu explicitamente a correção
de mídia nativa e das falhas de apresentação/IPC antes do bundle Linux.
Essa decisão prevalece sobre a exclusão anterior de suporte a formatos para
os caminhos Linux investigados nesta seção. Não representa aceite de reprodução
nem encerramento das lacunas de ownership R1–R4.

### Transporte de mídia nativa

A inspeção identificou que `getVideoUrl` e `getAudioUrl` retornavam
`asset://localhost/{id}` para mídia nativa no Linux. HLS utiliza HTTP local.
Há uma [ocorrência no projeto Tauri](https://github.com/tauri-apps/tauri/issues/8654)
com reprodução local por protocolo customizado falhando no WebKitGTK e mídia
HTTP funcionando. É evidência externa compatível, não uma reprodução isolada
neste host nem prova suficiente da causa de todos os arquivos.

A seleção de URL agora encaminha mídia nativa Linux para a rota existente
`http://127.0.0.1:9876/stream/{id}`, com token de sessão codificado na query.
Essa rota mantém resolução por ID, validação do caminho/raízes autorizadas,
autenticação e suporte a Range do servidor existente. A mudança preserva a
decodificação nativa e não força transcodificação. macOS/Windows mantêm os
protocolos anteriores, e as rotas HLS/linear não foram alteradas. O token é
obtido pelo IPC de startup existente. Não foi criado servidor ou acesso por
caminho arbitrário adicional.

Regressões verificam URL HTTP autenticada para áudio/vídeo nativos Linux,
preservação de HLS e do transporte anterior nas outras plataformas. A correção
foi confirmada depois no WebView real para MP4/MOV/MP3/WAV/FLAC, conforme a
subseção de validação de mídia nativa abaixo.

### Apresentação e IPC: lacuna de recuperação

Os testes nativos anteriores de apresentação foram positivos; falha de janela
ou IPC ainda não tinha sido exercitada. A inspeção do cliente encontrou dois
limites: uma consulta IPC sem resposta mantinha `queryPending` indefinidamente,
impedindo novas consultas, e a confirmação de apresentação era tentada apenas
uma vez. O backend já possui limite de apresentação independente da limpeza.

O cliente agora limita a espera de cada IPC a um segundo, permitindo novas
consultas após uma resposta ausente. Uma confirmação que falhe recebe uma única
nova tentativa após 100 ms, também limitada. Respostas nativas tardias não são
canceladas pelo timeout frontend; confirmações repetidas continuam idempotentes
no backend. Isso não altera os prazos de 1.500 ms/600 ms nem adia a limpeza.
Uma confirmação tardia segue a política existente e não reinicia o tempo mínimo.

Foram adicionados testes com IPC sem resposta, falha transitória de confirmação,
esgotamento limitado das tentativas e falha injetada. Para permitir validação
nativa controlada, o frontend de desenvolvimento ganhou injeção explícita de
IPC indisponível ou ausência de montagem da apresentação, protegida por
`import.meta.env.DEV`. A build de produção ignora essas falhas mesmo se a variável
de ambiente estiver presente; há teste dessa proteção. A injeção não simula uma
falha real de criação da janela nativa, e não substitui auditoria de todas as
operações IPC ou do fallback nativo. Nenhuma falha foi ativada nesta análise.

### Diagnóstico da imagem preta

A inspeção do host identificou NVIDIA GeForce GT 530 (GF108). A presença desse
hardware não demonstra defeito do driver ou do WebKit. A implementação de
WebKitGTK/GStreamer oferece um controle de diagnóstico do sink GL
([código-fonte WebKit](https://github.com/WebKit/WebKit/blob/main/Source/WebCore/platform/graphics/gstreamer/MediaPlayerPrivateGStreamer.cpp));
o nome correspondente foi encontrado na biblioteca instalada. Não foi aplicada
configuração permanente de desativação de GPU/sink, atualização de driver ou
mudança de renderer.

A telemetria dos players passou a registrar estado de mídia, dimensões e
contagem de frames quando disponível, com amostragem de avanço no máximo a
cada cinco segundos em DEBUG. Não serializa URL, token, mensagem bruta do
runtime ou conteúdo dos frames. Os listeners são liberados na troca da fonte
e no descarte do player. O teste verifica dados de dimensão, ausência de URL/token
e remoção dos listeners. Esses contadores não comprovam que os pixels foram
apresentados corretamente; seu objetivo é localizar o estágio da falha.

A validação desktop e a comparação do caminho gráfico foram realizadas nas
rodadas seguintes, com logs e observação do usuário. A preparação isolada não
foi tratada como evidência de execução gráfica.

### Verificação automatizada desta alteração

Os sete arquivos focados de frontend passaram com 26 testes, incluindo seleção
de transporte, ciclo do player, dados de diagnóstico, observação/recuperação de
IPC, pausa de mídia e apresentação de shutdown. A execução final usou um worker.
Na primeira execução paralela, o teste de apresentação excedeu 5 s; passou em
1,25 s na execução sequencial sem alterar seu limite ou sua implementação.
Isso registra sensibilidade ao ambiente de execução, sem comprovar sua causa.
Após ajustar o mock de telemetria do teste de playback, seus três testes passaram
novamente, sem tentar usar IPC nativo no jsdom. Essas três verificações se
sobrepõem aos 26 testes e não representam casos adicionais.

Typecheck, ESLint dos arquivos de implementação e testes alterados, Prettier e
verificação de whitespace do diff passaram. A build Vite final passou em 50,63 s, inclusive
com a variável de falha IPC presente no ambiente de produção. A inspeção da
entrada de shutdown gerada não encontrou os marcadores de injeção de falha.
Nenhum teste Rust foi necessário nesta rodada, que não alterou o backend.
Esses resultados não substituem a reprodução nativa, o teste comparativo do
sink gráfico e as falhas injetadas no aplicativo desktop, cujos resultados
posteriores estão registrados abaixo e encerram esses testes.

### Quatro cenários: transporte, sink GL e falhas injetadas (17 de novembro)

Foram recebidos os logs `midia-http-20261117-153804.log`,
`video-sem-gl-20261117-160910.log`, `ipc-indisponivel-20261117-162016.log` e
`sem-apresentacao-20261117-162314.log`. Todos registram desenvolvimento com
backend release sobre `71f10f3` e as alterações locais da seção 16.

| Cenário                                     | Pedido até conclusão do backend | Pool ocioso / total; fechamento | Resultado                         |
| ------------------------------------------- | ------------------------------: | ------------------------------- | --------------------------------- |
| Mídia HTTP, X com MP4 aberto                |                      1.623,6 ms | 13/13; 1.600 ms                 | `clean=true`; comando retornou 0. |
| Sink GL desativado, Alt+F4 com M2TS tocando |                        395,0 ms | 13/13; 295 ms                   | `clean=true`; comando retornou 0. |
| IPC de shutdown indisponível                |                        148,6 ms | 8/8; 147 ms                     | `clean=true`; comando retornou 0. |
| Apresentação sem montagem                   |                        175,6 ms | 9/9; 168 ms                     | `clean=true`; comando retornou 0. |

Nas quatro execuções, as 13 tarefas registradas concluíram sem timeout/abort,
os handlers de assets drenaram e o servidor e seus proprietários registrados
terminaram. No segundo cenário foi limpa uma sessão linear cujo processo já
havia saído, sem falhas de processo/diretório, em 10 ms. A confirmação da
apresentação chegou aos 1.350,1 ms, com `timely=true`. No primeiro não há
handshake registrado. As listagens finais não mostraram os processos de nomes
monitorados; isso não constitui inventário universal de recursos nativos.

#### Classificação indevida de URLs nativas como HLS

No primeiro cenário, o usuário relatou loading persistente em MP3/WAV/FLAC,
com waveform disponível, e MP4/MOV identificados como HLS sem reproduzir.
O log mostra `manifestParsingError`, fatal, com HTTP 200. A inspeção encontrou
uma interação não coberta pela alteração anterior: `isHlsUrl` retornava true
para qualquer URL que contivesse o endereço do servidor HLS. Assim, a rota
nativa `/stream/{id}` também acionava o hls.js, que tentava interpretar os
bytes do arquivo como manifesto. A URL HTTP estava corretamente selecionada,
mas era entregue ao engine incorreto. Os testes anteriores validavam a URL
sem verificar sua classificação no consumidor.

`isHlsUrl` agora verifica a extensão `.m3u8` do recurso, removendo query e
fragmento da comparação. Rotas `/stream`, `/probe`, `/health` e URLs blob não
são HLS apenas por compartilhar servidor. Foram acrescentadas regressões de
classificação da URL produzida para mídia nativa Linux, playlists com query,
rotas lineares e token contendo `.m3u8`. Dois testes do hook real de áudio/vídeo
verificam que a fonte HTTP nativa não cria gerenciador HLS e que `loadeddata`
retira o loading. A reprodução real após essa correção foi validada na
subseção seguinte de mídia nativa.

O log do segundo cenário também contém erro de parsing de manifesto durante
as tentativas de MOV. Não foi encontrado erro novo de FFmpeg/servidor que
permita atribuir todos os MOV a uma falha de codec. A classificação corrigida
cobre MOV encaminhado por `/stream`; MOV com codec realmente não nativo pode
continuar selecionando transcodificação conforme o probe, como antes.

#### Imagem com sink GL desativado

Com `WEBKIT_GST_DISABLE_GL_SINK=1`, o usuário confirmou imagem, som e seek para
3GP, 3G2, F4V, M2TS, MKV, AVI, FLV, HEVC, SWF e MJPEG. O log registra eventos
`loadeddata`/`playing`, dimensões válidas e avanço de contagem de frames,
inclusive 1280×720. Esta é evidência positiva do caminho alternativo do sink
neste host, em conjunto com a observação visual. Não prova qual componente
do caminho GL falha nem justifica desativação global em todos os hosts Linux.
Nenhuma configuração persistente de driver ou de GPU foi alterada. A política de compatibilidade explícita foi posteriormente implementada e
validada pelas preferências das seções 17 e 20.

#### IPC e apresentação indisponíveis

Os cabeçalhos dos dois últimos logs confirmam as falhas de desenvolvimento
solicitadas, com sink GL normal. O usuário confirmou comportamento esperado.
Não houve handshake de apresentação; a drenagem permaneceu independente do
frontend, com retorno ao prompt e saída 0. Esses cenários validam saída autônoma
quando os comandos/eventos da apresentação são bloqueados ou quando o conteúdo
não monta. Não simulam falha na criação do WebView, IPC geral da aplicação,
crash nativo ou prazo de saída imposto pelo sistema operacional.

**Verificação automatizada da correção de classificação:** 30 testes passaram
nos arquivos de URLs/HLS, gerenciador e cliente de shutdown; outros dois testes
do hook nativo passaram, totalizando 32 nesta execução focada. A regressão de
URL verifica também o classificador real usado pelos consumidores, cobrindo
a interação ausente nos testes anteriores. Typecheck final, ESLint dos arquivos
alterados, formatação e verificação de whitespace do diff passaram. A build Vite passou em
1 min 3 s, com 2.756 módulos transformados. Não houve alteração Rust nesta
correção. O comportamento com arquivos reais foi validado pelo usuário na
rodada seguinte; a etapa automatizada isolada não havia dado esse aceite.

### Validação real de mídia nativa após corrigir o classificador (17 de novembro)

No log `nativos-v2-20261117-163429.log`, a execução continuou em
`tauri dev --release`, sobre `71f10f3` com as alterações locais da seção 16.
O usuário confirmou execução do procedimento anterior, que manteve
`WEBKIT_GST_DISABLE_GL_SINK=1` e desativou as injeções de falha. O cabeçalho
deste log não imprime essas variáveis; a condição gráfica é atribuída ao
procedimento confirmado, e não à telemetria do WebView.

**Reprodução confirmada pelo usuário:** MP3, WAV e FLAC tocaram, carregaram
waveforms e permitiram seek. MP4 e MOV foram identificados como nativos pelo
frontend, reproduziram imagem e som e permitiram seek e mudança de velocidade.
F4V e M2TS mantiveram a estratégia HLS e continuaram reproduzindo normalmente.
O log registra `loadeddata` e `playing` com transporte HTTP para mídia nativa,
sem erro de mídia nesses eventos, além de dimensões e frames para vídeo.
Não houve novo `manifestParsingError` nesta amostra. Esses resultados validam
funcionalmente o encaminhamento HTTP nativo e a correção de `isHlsUrl` para os
arquivos observados, mantendo HLS para os demais caminhos testados.

**Fechamento:** o usuário fechou pelo X da titlebar com M2TS tocando. Houve um
único pedido às 19:40:50.499 UTC e conclusão do backend às 19:40:51.109,
aproximadamente 609,9 ms, com `clean=true`. As 13 tarefas registradas concluíram,
os proprietários registrados terminaram, o servidor parou e os protocolos
fecharam com `active=0` e drenaram. Não havia sessões lineares ou processos a
cancelar naquele instante, portanto a reprodução ativa não exercitou kill de
filho ativo nesta amostra. O pool tinha 14/14 conexões ociosas e fechou em
608 ms. A telemetria informou pausa de dois elementos cerca de 68,6 ms após
o pedido. Não há handshake de apresentação registrado.

O comando retornou 0 e a listagem final não mostrou processos dos nomes
monitorados. Não houve timeout, abort ou erro de pool fechado. Após a conclusão
limpa do backend, o WebKit registrou erro interno em
`WebLoaderStrategy::internallyFailedLoadTimerFired()`. Esse erro de runtime
permanece como observação: o log não determina sua causa nem demonstra perda
de dados, falha na drenagem registrada ou crash persistente. Não foi suprimido
nem considerado comprovadamente resolvido; o retorno 0 e a reprodução normal
não garantem ausência de defeito no runtime. A ocorrência deve ser distinguida
dos erros anteriores de parsing de manifesto, ausentes nesta rodada.

**Resultado:** validação de mídia nativa e regressão HLS positiva nos arquivos
observados, no ambiente com o sink GL desativado. As falhas injetadas da seção
anterior também possuem validação nativa positiva. A política persistente de
compatibilidade gráfica e a validação de bundle foram concluídas nas seções
17–20; não se atribui suporte universal a todo MP4/MOV ou a todo host Linux. Nenhuma alteração
de código ou novo teste automatizado foi necessária nesta rodada de análise.

### Inspeção independente do driver gráfico (17 de novembro)

A pedido do usuário, foi iniciada a separação entre falha de aplicação e
problema da pilha gráfica do host. `lspci -k` identificou a GeForce GT 530
(GF108) usando `nouveau`. O journal do kernel deste boot contém falhas de
carregamento de `nouveau/nvc1_fuc084` e `nouveau/nvc1_fuc084d`, seguidas de
`msvld: unable to load firmware data` e `init failed, -19`, inclusive às
12:26 e 13:58 locais. Às 12:26:26 há também `gr: TRAP` no contexto de
`gst-plugin-scan`, com `GPC0/PROP trap` e `RT_HEIGHT_OVERRUN`. Há avisos
repetidos de trabalho de hotplug do nouveau consumindo CPU.

Esses registros demonstram erros de driver/firmware no ambiente, além do
comportamento específico do Mundam. Não comprovam defeito físico da placa,
nem estabelecem que todo erro WebKit ou a imagem preta tenha exatamente essa
causa. Não foi atualizado driver, instalado firmware ou alterada configuração
gráfica nesta inspeção. `glxinfo` não estava disponível no ambiente consultado;
`gst-play-1.0` estava. O acesso a dispositivos gráficos pelo sandbox do agente
não deve ser usado para inferir presença/permissões na sessão desktop do usuário.

Os testes independentes de identificação do renderizador, padrão OpenGL e
reprodução GStreamer fora do Mundam foram realizados na rodada seguinte.
A comparação serve para distinguir renderização, decodificação e integração
WebKit; passar em um deles não valida universalmente os demais caminhos.

### Resultados dos testes gráficos independentes (17 de novembro)

Foram analisados `ambiente.log`, `padrao-opengl.log`, `arquivo-opengl.log`,
`arquivo-ximagesink.log` e `kernel.log`, em `/tmp/mundam-gpu/`.
O `glxinfo -B` informa renderizador NVC1, Mesa 26.0.8, `direct rendering: Yes`
e `Accelerated: yes`, com OpenGL 4.3 e aproximadamente 2 GB de memória de vídeo.
Não se trata de llvmpipe/softpipe nesse teste GLX. Isso confirma aceleração no
caminho consultado, sem demonstrar que todos os clientes Wayland/EGL/WebKit
utilizem ou apresentem buffers da mesma forma.

O padrão colorido OpenGL foi observado sem piscadas ou tela preta pelo usuário;
o pipeline encerrou por EOS após 10,07 s. O arquivo F4V foi reproduzido com boa
imagem e som tanto por `glimagesink` quanto por `ximagesink`, conforme relato.
Os logs de reprodução chegaram ao término da lista. Há aviso de qtdemux sobre
um segmento que excede levemente a duração declarada do arquivo; no teste GL
há também `gst_egl_image_from_dmabuf: eglCreateImage failed: EGL_BAD_ALLOC`.
Apesar desse aviso, a reprodução observada funcionou. O aviso não deve ser
omitido nem interpretado sozinho como falta de VRAM ou causa comprovada da
imagem preta no WebKit.

O kernel mantém as falhas de firmware e o trap já descritos; são registros
anteriores aos novos testes, não prova de novo trap durante cada reprodução.
O usuário relatou ainda glitches ocasionais ao abrir ChatGPT ou VSCode.
Não houve reprodução controlada desses glitches nesta rodada, portanto sua
relação com o driver, compositor e buffers do WebKit permanece indeterminada.

**Conclusão delimitada:** a GPU e o driver conseguem renderizar OpenGL e
reproduzir o arquivo testado fora do Mundam. Não há fundamento para declarar
placa inoperante ou defeito físico a partir destes dados. Tampouco a pilha
gráfica está comprovadamente livre de problemas: existem erros de
firmware/driver e um aviso de importação DMA-BUF/EGL fora da aplicação.
A falha do vídeo no caminho GL do WebKit continua sendo uma incompatibilidade
na integração WebKit/GStreamer/Mesa/nouveau/compositor a isolar; não foi atribuída tecnicamente a defeito físico ou lógica de playback.
As opções de compatibilidade foram validadas depois; a causa interna da
pilha gráfica fica como nota futura, sem bloquear o aceite Linux.
O workaround de sink do WebKit possui evidência funcional
neste host, sem comprovar a causa específica. Nenhum driver foi trocado e
nenhuma configuração gráfica persistente foi alterada nesta análise.

## 17. Preferência persistente de compatibilidade de vídeo Linux (17 de novembro)

**Estado na revisão de 18/11:** Preferência de vídeo implementada e validada após reinício; registros de erros intermitentes são notas futuras, sem bloqueio de aceite.

O usuário aprovou oferecer um modo de compatibilidade e indicou a
[orientação oficial do Tauri para gráficos Linux](https://v2.tauri.app/develop/debug/linux-graphics/).
A página relaciona incompatibilidades WebKitGTK/driver e recomenda aplicar
workarounds antes de criar o WebView, evitando impor perda de desempenho a
hosts não afetados. Os ajustes de DMABUF, sincronização e composição citados
ali não são equivalentes ao ajuste específico do sink de vídeo validado neste
host. Nenhuma configuração de kernel ou driver foi alterada.

### Implementação

Foi adicionada em Settings → Performance uma seleção de vídeo visível apenas
no Linux, com Normal como padrão e Compatibility como opção explícita. O texto
explica seu uso para imagem preta/glitches, possível redução de desempenho e
necessidade de reiniciar. O painel carrega a preferência ao abrir, sem depender da passagem pelo painel
General, e mantém a seleção desabilitada até concluir essa leitura. Uma falha
de carregamento mantém o controle desabilitado e informa como tentar novamente.
O salvamento não afirma que o renderer já mudou na sessão atual; falha de
persistência mantém a preferência anterior e informa o erro sem expor detalhes
internos.

A chave booleana `linux_video_compatibility` é persistida no mapa `extra` do
`settings.json` existente. O serviço valida o tipo tanto no comando de chave
individual quanto na atualização completa das preferências. Arquivos antigos
ou chave ausente mantêm modo normal. Não há alteração do schema SQLite.

No início de `run`, antes do builder, do GTK/WebView e dos workers, o bootstrap
Linux lê essa preferência do mesmo diretório de dados local/identificador
usado pelo Tauri. Foi declarada dependência Linux direta de `dirs` 6, já
presente transitivamente no lockfile, para manter a resolução de diretório
alinhada com o runtime. Com a preferência ativada e sem override externo,
aplica `WEBKIT_GST_DISABLE_GL_SINK=1`; no modo normal deixa o ambiente intacto.
Uma variável definida explicitamente pelo lançador/terminal tem precedência,
inclusive valor 0, e essa origem é registrada no log. Esse bootstrap de vídeo não altera DMABUF ou composição. O ajuste independente
de DMABUF foi adicionado posteriormente pelo bootstrap gráfico da seção 20;
`WEBKIT_DISABLE_COMPOSITING_MODE` não é aplicado por essas preferências.

Leitura ausente mantém normal; falha de leitura/parse é registrada sem sobrescrever
preferências ou ativar workaround por suposição. A criação de contexto e a
instalação do builder reutilizam uma única expansão do contexto Tauri. A mudança
vale para desenvolvimento e binário empacotado, sem depender de script npm ou
variável fornecida manualmente no uso normal. macOS e Windows não executam
esse bootstrap específico.

### Cobertura de regressão preparada

Foram adicionados testes de persistência real em arquivo através do serviço
existente e leitura antecipada, chave ausente, valor inválido, configuração
incompleta e retorno ao modo normal. Testes em processos filhos verificam a
aplicação no startup e a preservação dos overrides 0/1, sem alterar o ambiente
do processo de teste principal. Testes frontend cobrem salvamento antes de
confirmar o modo, falha de gravação, indicação de reinício e visibilidade
apenas no Linux, além da integração com o store existente.

A aplicação da preferência salva sem variável externa foi validada nas
rodadas de desktop abaixo. A evidência anterior por comando foi distinguida
dessa confirmação de persistência.

### Validação automatizada da preferência persistente

Cinco testes Rust passaram, sem falhas ou ignorados, com 328 testes fora do
filtro. Incluem persistência real via `SettingsService`/adaptador JSON, leitura
antecipada, ausência da chave, valores inválidos, retorno ao normal e uma matriz
de quatro combinações de preferência/override em processos filhos. Um dos
cinco testes é a entrada do subprocesso, exercitada pela matriz; as execuções
filhas não são contadas como testes adicionais. Esses processos não criam GUI.

A execução final usou um job Cargo e override temporário de debug apenas para
o pacote Mundam no perfil de testes, sem modificar os perfis do repositório.
A compilação levou 17 min 17 s e os testes concluíram em 0,09 s. Uma tentativa
anterior não produziu resultado final recuperável; o journal registrou ação
do `systemd-oomd` às 17:29:04, contemporânea à perda das sessões. Essa tentativa
não está registrada como aprovada. Não se atribui automaticamente a pressão
a um único processo nem ao novo recurso da aplicação.

Seis testes do painel e quatro do store passaram, totalizando dez testes
frontend nesta validação. Verificam também leitura independente do painel,
bloqueio do controle diante de falha de carregamento e confirmação de modo
apenas após persistência. As tentativas iniciais com workers fork expiraram
antes de executar os casos; o painel passou com uma thread de worker, sem
ampliar os timeouts dos testes. A aprovação final corresponde a esse ambiente
de execução e não converte tentativas sem execução em resultados positivos.

Typecheck final, ESLint dos arquivos alterados, Prettier, formatação Rust e
verificação de whitespace do diff passaram. A build Vite final passou em 33,23 s.
A validação manual após reinício e os testes de bundle foram concluídos
nas rodadas posteriores. Lacunas compartilhadas vigentes constam da seção 14.

### Preferência salva validada no desktop e ressalvas de encerramento (17 de novembro)

Foram analisados `salvar-compatibilidade-20261117-180255.log` e
`compatibilidade-salva-20261117-182414.log`. A primeira execução registrou no
startup `compatibility_enabled=false source="settings"`; após os testes, o
usuário selecionou Compatibility, aguardou a confirmação e fechou. Na segunda,
o startup registrou `compatibility_enabled=true source="settings"`, comprovando
leitura e aplicação da preferência salva, sem depender do override manual.
O usuário confirmou que a opção persistiu no painel.

No modo normal, MP4/M2TS tocaram com imagem preta e MP3 tocou normalmente.
Após reabrir em Compatibility, os mesmos vídeos apresentaram imagem, som e seek
normais, e o áudio permaneceu funcional. A comparação confirma o efeito da
preferência persistente neste host; não exige impor o ajuste aos demais hosts.
A primeira compilação release levou 14 min 36 s; a segunda, 31,18 s. Esses
valores não medem latência de boot ou finalização.

| Execução                                        | Pedido até conclusão do backend | Drenagem registrada / pool                                            | Resultado                                                                          |
| ----------------------------------------------- | ------------------------------: | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Salvar Compatibility, sessão iniciada em Normal |                        298,4 ms | 13 tarefas limpas; pool 16/16 ocioso e fechado em 294 ms.             | Backend `clean=true`; comando Tauri retornou 0, com erro nativo posterior.         |
| Reabrir com Compatibility salvo                 |                      3.296,1 ms | 13 tarefas limpas; pool 8/8 ocioso antes de `close`, que excedeu 3 s. | Backend `clean=false`; comando Tauri retornou 0, com erro posterior do WebProcess. |

Em ambas, manutenção, servidor e proprietários registrados terminaram, e os
handlers de assets drenaram antes do pool. Não houve timeout de worker ou
processo de mídia registrado. Na primeira, houve confirmação pontual de
apresentação aos 1.263,1 ms após o pedido. Na segunda não há handshake
registrado. As listagens finais não mostraram os processos dos nomes monitorados,
mas não abrangem todos os nomes de subprocessos do WebKit.

#### Pressão de memória e erros nativos: limites da conclusão

O usuário relatou alertas de baixa memória simultâneos em outras aplicações
no segundo fechamento. É evidência de pressão sistêmica percebida, compatível
com atrasos na saída, mas não identifica o consumidor nem comprova que esse
fator causou o timeout do SQLite. O journal de `systemd-oomd` consultado após
18:20 não continha entradas; a consulta ao journal geral no intervalo também
não recuperou eventos explicativos. A ausência desses registros não invalida
os alertas observados e não prova ausência de pressão ou de falha nativa.

No primeiro log, depois da conclusão limpa e do handshake, apareceu
`double free or corruption (out)`. Não é correto tratá-lo como mensagem genérica
de pouca RAM ou ignorá-lo porque o comando Tauri retornou 0. O log não identifica
qual processo emitiu a mensagem nem fornece stack trace para localizar a
liberação inválida. Portanto, não comprova corrupção do catálogo SQLite,
mas impede afirmar que todo o término nativo ocorreu sem erro.

No segundo, depois da conclusão degradada, o WebKit registrou às 18:32:18
que o WebProcess não saiu como esperado após fechar a conexão com o processo
UI. O comando terminou às 18:33:42 locais, cerca de 2 min 13 s após o pedido
inicial, conforme o timestamp de segundos do cabeçalho final. Isso distingue
conclusão do coordenador de saída efetiva do comando; não foi uma execução
completa de aproximadamente 3,3 s. O log não determina a causa dessa demora.

**Resultado:** preferência persistente e efeito funcional no vídeo validados.
O segundo encerramento foi degradado e o primeiro registrou erro nativo
posterior ao backend limpo. São resultados históricos dessas amostras; o aceite
Linux posterior foi dado com as observações transferidas para notas futuras. As repetições com execução isolada e código de saída direto estão na
subseção seguinte. As ocorrências sem causa atribuída ficaram como notas
futuras no encerramento da sessão, sem novo reteste exigido. Não houve mudança
de código nesta rodada de análise.

### Binário direto: ocioso e M2TS, sem wrapper Tauri (17 de novembro)

O usuário confirmou execução dos procedimentos de binário direto, com Vite
separado e sem nova compilação, nos logs `direto-ocioso-20261117-184538.log`
e `direto-video-20261117-184811.log`. O primeiro fechou sem abrir mídia;
o segundo fechou após reprodução de M2TS. Ambos os startups registraram
`compatibility_enabled=true source="settings"`, mantendo o modo persistente
sem override manual.

| Execução         | Pedido até conclusão do backend | Pool ocioso / total; fechamento  | Handshake após pedido | Saída do binário |
| ---------------- | ------------------------------: | -------------------------------- | --------------------: | ---------------- |
| Direto ocioso    |                        283,5 ms | 16/16; 275 ms                    |   1.096,4 ms, pontual | 0                |
| Direto com vídeo |                        372,2 ms | 9/9 após espera de 52 ms; 318 ms |   1.067,4 ms, pontual | 0                |

Cada execução teve um único pedido e uma conclusão `clean=true`, com 13 tarefas
registradas concluídas. Manutenção, servidor, proprietários registrados,
handlers de assets e bridge terminaram antes do fechamento do pool. Não havia
sessões lineares para limpar. A pausa de mídia informou zero elementos no
ocioso e dois no vídeo, aproximadamente 6,2 ms e 63,8 ms após o pedido.
A confirmação da apresentação foi seguida do registro de ocultação das janelas
anteriores, sem erro reportado.

Não houve timeout/abort, pool fechado indevidamente, `double free or corruption`,
erro interno de WebKit ou erro de saída do WebProcess nestas amostras. O código
0 foi capturado do binário, independentemente do wrapper npm/Tauri. As listagens
finais incluíram nomes WebKit, além de Mundam/FFmpeg/FFprobe, sem correspondências
presentes. Os timestamps finais foram 18:46:51 e 18:49:24 locais, sem a demora
de mais de dois minutos observada no teste anterior; a resolução desses
registros não permite medir precisamente a duração total até o retorno.

Os snapshots finais indicaram cerca de 2 GB de RAM em uso, 5,2 GB disponíveis
e 1,3 GB de swap ocupada. São fotografias antes/depois, não monitoramento de pico
ou evidência suficiente para identificar o consumidor da pressão anterior.
O aviso conhecido de fallback do clipboard permaneceu sem falha de fechamento
associada nestes testes.

**Resultado:** execução direta ociosa e com mídia encerrou limpa, com modo
persistente aplicado e código real de saída 0. Os erros nativos e o timeout do
pool anteriores não foram reproduzidos. Permanecem no histórico como ocorrências
intermitentes sem causa comprovada; estes resultados não demonstram que tenham
sido causados exclusivamente por pouca RAM, nem que foram definitivamente
corrigidos. O binário usado ainda é de desenvolvimento com Vite, não um bundle.
Nenhuma alteração de código foi necessária nesta rodada de análise.

## 18. Início dos bundles Linux (17 de novembro)

**Estado na revisão de 18/11:** Debian gerado, instalado e exercitado; a ausência de teste nos parágrafos de preparação é histórica e foi superada pelas execuções abaixo.

O usuário autorizou iniciar os testes de bundle após as duas saídas diretas
limpas. O primeiro formato selecionado foi `.deb` amd64 para o Ubuntu 26.04.1
atual. Na preparação inicial o AppImage ainda não havia sido gerado; foi
validado depois, nas seções 19–20. Esta rodada descreve um candidato,
não aceite final de R1–R9 nem comprovação de compatibilidade com outros hosts.
A [documentação Debian do Tauri](https://v2.tauri.app/distribute/debian/)
explica que a base de compilação pode elevar o requisito de glibc; não foi
produzida uma build em distribuição mais antiga nesta rodada.

### Preparação e recursos conferidos

A configuração Linux já declarava FFmpeg/FFprobe, Assimp e PDFium; a configuração
principal declara SoundFonts. A CLI instalada confirmou merge automático da
configuração por plataforma. Os recursos Linux foram encontrados no staging;
FFmpeg, FFprobe, CLI Assimp e PDFium são ELF x86_64. FFmpeg e Assimp usam
bibliotecas compartilhadas do sistema; não são executáveis estáticos autônomos.
O `ldd` do PDFium não indicou biblioteca ausente no host atual. Essa inspeção
não equivale a validar uma instalação em máquina limpa.

Foram declaradas dependências Debian explícitas para `ffmpeg` e os plugins
GStreamer base/good/bad/ugly/libav usados no ambiente de reprodução validado.
`assimp-utils` foi declarado como recomendado, mantendo o tratamento opcional
do suporte 3D. O objetivo é não depender apenas da presença dos executáveis
copiados quando eles ainda exigem bibliotecas externas. O control gerado preservou também as dependências padrão
`libwebkit2gtk-4.1-0` e `libgtk-3-0`.

A build foi iniciada com um job Cargo, resolução offline das dependências Rust
já presentes e sem overrides de sink GL ou falhas de shutdown no ambiente.
O frontend de produção compilou em 32,94 s, incluindo as entradas de splash e
shutdown. A compilação Rust de produção terminou em 14 min 14 s e o empacotamento
Debian foi concluído com código 0. Não há instalação ou execução empacotada
aprovada neste ponto.

### Identificação do candidato

O snapshot inicial registra o commit base e as alterações locais em
`/tmp/mundam-bundle-linux/provenance.json`, com SHA-256 do patch e dos arquivos
novos. O patch e uma cópia dos arquivos novos foram guardados nessa mesma
pasta. Isso permite distinguir este candidato de builds anteriores com a mesma
versão 0.1.0. O log de geração foi preservado em `build-deb.log` nessa pasta.
Nenhum pacote foi instalado pela preparação.


### Inspeção do Debian gerado

O artefato `src-tauri/target/release/bundle/deb/Mundam_0.1.0_amd64.deb`
tem 40.657.496 bytes e SHA-256
`743e295c5acd0a80501b333689392164a855b59a23618be66827ce7b61a9c4de`.
O manifesto de proveniência foi complementado com o hash do pacote e do
executável extraído. O conteúdo foi extraído sem instalação para inspeção.

Foram confirmados `/usr/bin/mundam`, o desktop entry com `Exec=mundam`, ícones,
SoundFont e recursos Linux em `/usr/lib/Mundam/resources`. Os executáveis
empacotados mantiveram permissão de execução. No host atual, o `ldd` do Mundam,
FFmpeg, FFprobe, Assimp e PDFium não apontou bibliotecas ausentes. As consultas
de versão dos três utilitários extraídos retornaram código 0. O arquivo
`libassimp.so` de 14 bytes é texto, não uma biblioteca ELF; a biblioteca com
SONAME `libassimp.so.6` está presente e a resolução no host atual funciona.
Isso não comprova isolamento das bibliotecas do sistema ou instalação limpa.
Os resultados detalhados ficaram em `inspecao-deb.log` na pasta de evidências.

O bundler emitiu aviso de ausência do símbolo `__TAURI_BUNDLE_TYPE`, que pode
prejudicar a identificação do pacote pelo updater. O pacote foi gerado, mas
atualização automática não foi validada. Não foi encontrada integração do
plugin updater no código desta rodada. O aviso permanece registrado e não
foi tratado como falha de execução já comprovada.

**Resultado desta etapa:** geração e inspeção estrutural do `.deb` concluídas;
instalação, mídia e fechamento foram testados nas rodadas abaixo, e o
AppImage foi gerado e validado posteriormente. Não há preparação pendente.


### Instalação e execução do Debian no host atual

Evidências recebidas: `instalacao-deb.log`,
`deb-ocioso-20261117-191638.log` e `deb-midia-20261117-191852.log`,
em `/tmp/mundam-bundle-linux/`. O apt instalou e configurou `mundam` 0.1.0,
processando os gatilhos de ícones, menus e desktop. Nenhuma dependência adicional
precisou ser instalada nesta máquina já preparada; isso não valida resolução
em ambiente limpo.

O usuário confirmou abertura e fechamento normais no cenário ocioso. No cenário
de mídia, a preferência Compatibility permaneceu salva; MP3 teve waveform,
reprodução e seek, e MP4/M2TS tiveram imagem, áudio, seek e alteração de
velocidade. O fechamento ocorreu com M2TS tocando e a tela de finalização
terminou, devolvendo o terminal. Os dois boots registraram
`compatibility_enabled=true source="settings"`.

| Evidência                       | Ocioso                 | Mídia                  |
| ------------------------------- | ---------------------- | ---------------------- |
| Tarefas registradas encerradas  | 13, `clean=true`       | 13, `clean=true`       |
| Pool SQLite antes do fechamento | 7 conexões, 7 ociosas  | 9 conexões, 9 ociosas  |
| Espera por conexões ociosas     | 0 ms                   | 0 ms                   |
| Fechamento do pool              | 138 ms                 | 205 ms                 |
| Backend: pedido até conclusão   | aproximadamente 141 ms | aproximadamente 211 ms |
| Apresentação reconhecida        | `timely=true`          | `timely=true`          |
| Código do executável principal  | 0                      | 0                      |

O fallback conhecido do clipboard Wayland para X11 permaneceu nos dois logs.
Não houve timeout do pool ou falha das tarefas registradas. Entretanto, a
listagem imediatamente após o retorno ainda mostrou WebKit: no ocioso,
PID 117509 com RSS 0; na mídia, PIDs 117866 (RSS 0) e 117875 (RSS 538.684 KiB).
Todos tinham PPID 80068. Sem estado do processo, linha de comando e observação
posterior, essa fotografia não distingue processos transitórios, zombies ou
processos que continuaram ativos, nem prova a origem de todos eles.

Após essa listagem, o log de mídia contém `free(): corrupted unsorted chunks`.
A mensagem não identifica PID e apareceu depois de `MUNDAM_EXIT_CODE=0`;
portanto, o código do processo principal não comprova saída limpa de toda a
árvore WebKit. A ocorrência permanece sem causa atribuída, relacionada no
histórico aos erros intermitentes de alocador já observados, sem comprovação
de que seja a mesma falha ou consequência de pouca memória. Os snapshots finais
mostram 5,2 GiB disponíveis no ocioso e 5,0 GiB na mídia, sem medir picos.

O HlsManager registrou zero sessões rastreadas no fechamento de mídia, apesar
da reprodução HLS observada. O teste confirma reprodução, mas não demonstra
cancelamento de transcodificação em andamento nem remoção de seus diretórios.

**Resultado:** instalação, preferência persistente e reprodução do `.deb`
validadas nestes cenários no host atual. O backend e o processo principal
encerraram com sucesso. A observação posterior dos subprocessos foi acrescentada
na repetição abaixo. A mensagem de corrupção ficou como nota futura, sem
bloqueio do aceite Linux, e o AppImage foi validado nas seções seguintes.


### Repetição com mídia e observação posterior dos subprocessos

Evidência: `/tmp/mundam-bundle-linux/deb-saida-midia-20261117-192634.log`.
A captura inicial não mostrou Mundam, WebKit, FFmpeg ou FFprobe no filtro usado.
O modo Compatibility foi novamente lido das configurações. Foram registrados
reprodução HTTP de áudio e vídeo, vídeo com dimensões 1280×720, reprodução HLS
e avanço de posição HLS para aproximadamente 44,22 s. O log não comprova por
si só percepção de imagem/áudio ou alteração de velocidade nesta repetição.

O pedido de fechamento ocorreu às 19:28:30.061 locais e o backend concluiu às
19:28:30.238: aproximadamente 177 ms. As 13 tarefas registradas encerraram com
`clean=true`; o pool tinha 8 conexões, todas ociosas, e fechou em 153 ms, sem
espera por conexões ociosas. A apresentação foi reconhecida com `timely=true`
e o executável principal retornou `MUNDAM_EXIT_CODE=0`.

Diferentemente da amostra anterior, havia um processo de segmento em andamento:
`process_cancellations=1`. O log registrou sua terminação aguardada com
`reason=Shutdown`, status SIGKILL, seguida do cancelamento da requisição do
segmento 23 e da conclusão dos owners registrados. Esse SIGKILL foi registrado
no fluxo explícito de cancelamento; não é evidência de OOM nesta execução.
O aviso HLS `fragLoadError`, HTTP 503, `fatal=false`, ocorreu depois do pedido
de fechamento e corresponde ao cancelamento do segmento. O handler retorna
503 para esse caso. Não foi tratado como falha de reprodução anterior ao
fechamento. O HlsManager continuou com zero sessões rastreadas, mas essa métrica
não abrange o processo de segmento que teve cancelamento e espera comprovados.
A remoção de diretórios não foi comprovada pela captura.

Na primeira fotografia após o retorno, às 19:28:31, o WebKitWebProcess PID
118712 ainda estava presente, estado `SLl+`, RSS 559.132 KiB, PPID 80068.
Não era zombie nessa fotografia. Já às 19:28:32 não apareceu nenhum processo
no filtro, e a ausência se manteve às 19:28:36 e 19:28:46. Assim, nesta amostra,
o WebProcess saiu entre as duas primeiras fotografias; não houve processo
remanescente persistente durante os 15 segundos observados. A RAM disponível
passou de 4,9 GiB na primeira fotografia para 5,4 GiB na última.

Não apareceu `free(): corrupted unsorted chunks` nem outro erro de alocador
nesta repetição. Isso demonstra não reprodução, sem corrigir ou explicar a
ocorrência anterior. Nenhuma alteração de código foi feita para este teste.

**Resultado:** repetição do fechamento com mídia aprovada nesta amostra,
incluindo cancelamento aguardado de um processo de segmento e saída posterior
do WebProcess. A suspeita de subprocesso persistentemente ativo não se
confirmou nesta captura. O erro de alocador ficou somente como nota futura;
não há promessa de estabilidade universal ou validação de máquina limpa.
O AppImage foi gerado e validado nas seções 19–20.


## 19. Preparação do AppImage Linux (17 de novembro)

**Estado na revisão de 18/11:** AppImage gerado e testado; glitches iniciais foram contornados pela preferência gráfica implementada na seção 20. Sessão aceita.

Após a repetição do Debian, o usuário autorizou prosseguir para AppImage e
reservou a investigação do erro intermitente para depois dessa validação.
Problemas de disco ou memória são hipóteses do usuário, sem diagnóstico
confirmado pelas evidências atuais.

Foi habilitado `bundle.linux.appimage.bundleMediaFramework` na configuração
Linux, conforme a [documentação oficial do Tauri](https://v2.tauri.app/distribute/appimage/)
para reprodução multimídia. Os recursos já declarados de FFmpeg/FFprobe,
Assimp, PDFium e SoundFonts foram mantidos. O candidato é amd64, compilado
no Ubuntu 26.04.1; não demonstra compatibilidade com distribuições mais antigas.

O frontend de produção compilou em 47,45 s. A build foi iniciada com um job
Cargo e sem overrides de sink GL ou falhas de shutdown. O snapshot de origem,
patch das alterações locais, arquivos novos e manifesto de hashes foram salvos
em `/tmp/mundam-appimage-linux/`, junto ao log `build-appimage.log`.
A compilação Rust terminou em 18 min 21 s. Nenhum teste gráfico do
AppImage foi realizado nesta preparação.


### Geração e inspeção do candidato AppImage

A primeira tentativa concluiu o executável, mas falhou no empacotamento com
`Read-only file system (os error 30)`. O empacotamento foi retomado usando
cache em `/tmp/mundam-appimage-linux/cache`, sem recompilação Rust. A tentativa
restrita de download falhou por resolução DNS; com acesso de rede autorizado,
as ferramentas oficiais foram baixadas e o empacotamento concluiu com código 0.
O log dessa retomada é `bundle-appimage.log` na pasta de evidências.

O arquivo `src-tauri/target/release/bundle/appimage/Mundam_0.1.0_amd64.AppImage`
tem 225.819.128 bytes (aproximadamente 215,4 MiB), SHA-256
`e5f976c1906bb2c5110c0d12ada6d0830952608f31b2bdaf2d83b7eaec480c24`.
O manifesto de proveniência foi complementado com o hash do artefato e do
executável interno. A extração do arquivo final para inspeção retornou código
0, com conteúdo de aproximadamente 635 MiB. O ambiente restrito emitiu avisos
`Failed to create stream fd: Operation not permitted`; não houve abertura da
interface ou teste de montagem FUSE nessa inspeção.

Foram confirmados launcher, executável, FFmpeg, FFprobe, Assimp, PDFium,
SoundFont e scanner GStreamer dentro do arquivo extraído. Foram encontrados
273 plugins GStreamer, incluindo playback, libav, isomp4, audioparsers e
ximagesink. Com o diretório interno de bibliotecas no caminho de resolução,
o `ldd` do Mundam, FFmpeg, FFprobe, Assimp, PDFium e scanner não mostrou
bibliotecas ausentes no host atual. As consultas de versão dos utilitários
FFmpeg, FFprobe e Assimp retornaram código 0. Os resultados foram preservados
em `inspecao-appimage.log`; isso não prova execução em máquina limpa ou ausência
de dependências do host.

O AppRun gerado carrega hooks GTK e GStreamer. O hook GTK seleciona
`GDK_BACKEND=x11` e o hook GStreamer aponta os plugins e scanner para o AppDir.
Esse comportamento do launcher difere da execução direta do Debian no ambiente
Wayland e foi preservado, sem override adicional durante a preparação.

O aviso de símbolo `__TAURI_BUNDLE_TYPE` ausente voltou a ocorrer. O plugin
GStreamer também emitiu avisos de copyright não localizado ao consultar cópias
de bibliotecas dentro do AppDir, além de avisos do launcher já existente.
Esses avisos não impediram a geração; não se considera validado o updater nem
uma auditoria de completude das licenças por esse resultado.

**Resultado:** geração e inspeção estrutural do AppImage concluídas. Abertura
pela imagem, reprodução e preferência persistente foram testados depois,
conforme as rodadas abaixo. O erro intermitente ficou como nota futura,
sem investigação exigida para a conclusão.


### Primeiras execuções: apresentação visual bloqueada

Evidências: `appimage-ocioso-20261117-201319.log` e
`appimage-midia-20261117-201644.log`, em `/tmp/mundam-appimage-linux/`.
Ambas registraram o SHA-256 do candidato descrito acima. O usuário relatou
corrupção visual em toda a interface desde a abertura, tornando splash,
janela principal e finalização ilegíveis. A segunda tentativa apresentou o
mesmo problema e não permitiu testar mídia; o nome do log não representa
reprodução efetivamente validada.

Nos dois boots, `compatibility_enabled=true source="settings"` confirmou
aplicação da preferência de vídeo. O backend chegou a BackendReady e concluiu
o scan de 787 arquivos inalterados, sem erros. Não há evento de reprodução
nessas capturas. A preferência existente atua no sink GL do vídeo; sua
aplicação não comprova correção da composição gráfica de toda a interface.

As 13 tarefas registradas terminaram com `clean=true` nos dois testes. Os pools
SQLite tinham 8 conexões ociosas de 8 e fecharam em 446 ms e 220 ms,
respectivamente, sem espera por conexões ociosas. O backend concluiu em cerca
de 448 ms e 221 ms desde o pedido. As duas imagens retornaram
`APPIMAGE_EXIT_CODE=0`, mas não há registro de reconhecimento da apresentação
`timely=true` nessas execuções, nem confirmação visual possível pelo usuário.

No ocioso, WebKitWebProcess PID 134280 estava presente na primeira fotografia,
às 20:15:20, estado `SLl+`, RSS 280.732 KiB. Não apareceu nas fotografias de
20:15:22, 20:15:26 e 20:15:36. Na segunda tentativa não houve correspondências
no filtro de processos em nenhuma das quatro fotografias finais. Não houve
mensagem de corrupção de alocador nesses logs; isso não encerra a investigação
da ocorrência intermitente anterior.

A inspeção do launcher confirmou `GDK_BACKEND=x11` definido incondicionalmente
pelo hook GTK do AppImage, além das bibliotecas gráficas empacotadas. Essas são
diferenças relevantes em relação à execução Debian validada, sem demonstrar
qual delas causa os glitches. Não há erro explícito de DMABUF, GL ou falha de
hardware registrado nessas capturas. A
[documentação de gráficos Linux do Tauri](https://v2.tauri.app/develop/debug/linux-graphics/)
lista o renderer DMABUF e a composição acelerada entre caminhos relacionados
a problemas visuais; essa referência sustenta uma hipótese diagnóstica, não
uma causa comprovada neste host. Nenhuma mudança de renderer foi incorporada
ao produto nesta análise.

**Resultado histórico:** o candidato inicial apresentou falha visual nessas
duas execuções e bloqueou mídia. O candidato final com preferência gráfica
foi validado na seção 20; esse bloqueio de apresentação foi superado. O fechamento do backend e o retorno
do processo tiveram sucesso, sem comprovação de tela de finalização correta.
Não se atribuiu a falha a disco, RAM ou GPU física pelas evidências atuais.


### AppImage com renderer DMABUF desativado por ambiente

Evidência: `/tmp/mundam-appimage-linux/appimage-sem-dmabuf-20261117-202044.log`.
A execução foi identificada pelo marcador
`DIAGNOSTICO: WEBKIT_DISABLE_DMABUF_RENDERER=1`. O usuário confirmou splash,
janela principal e finalização legíveis, ausência de glitches ao redimensionar
e funcionamento dos testes de áudio/vídeo solicitados (MP3, MP4 e M2TS,
reprodução, seek e velocidade). Também relatou melhora percebida de fluidez
e de artefatos na apresentação/transparência da janela; não houve medição de
desempenho ou isolamento específico da transparência. Fechou com Alt+F4.
A preferência existente do sink de vídeo permaneceu aplicada:
`compatibility_enabled=true source="settings"`.

O log confirma reprodução HTTP de áudio e vídeo, dimensões de vídeo 1280×720,
manifestos HLS e reprodução com posições avançadas após seek. Houve dois avisos
HLS `bufferSeekOverHole` e um `bufferStalledError`, todos `fatal=false` e sem
código de erro do elemento de mídia. Foram seguidos de eventos `playing`;
não se classificou a sessão como isenta de avisos, nem se inferiu falha fatal
a partir deles. O relato do usuário não indicou prejuízo perceptível.

O pedido de fechamento foi registrado às 20:23:45.943 locais. As 13 tarefas
registradas concluíram com `clean=true`. O SQLite tinha 18 conexões, todas
ociosas, e fechou em 2654 ms, com `idle_wait_ms=0`. O backend concluiu às
20:23:48.710, aproximadamente 2768 ms após o pedido, e a imagem retornou
`APPIMAGE_EXIT_CODE=0`. Não houve timeout, mas o fechamento do pool foi
consideravelmente mais lento que nas amostras anteriores. Não há registro de
reconhecimento da apresentação `timely=true` nesta captura; a confirmação da
tela de finalização é o relato visual do usuário.

O primeiro timestamp após o retorno foi 20:23:55, cerca de nove segundos após
o pedido e seis segundos após a conclusão do backend. Às 20:23:55 e 20:23:56,
os processos WebKitNetworkProcess PID 135662 e WebKitWebProcess PID 135672
estavam presentes em estado `DLl+`. Não apareceram às 20:24:00 e 20:24:10.
Portanto, houve saída posterior dos subprocessos no intervalo observado,
mas com demora maior que a repetição do Debian. O estado D é uma observação
de espera ininterruptível; a captura não identifica a operação ou comprova
falha de disco, RAM ou outro componente. Não houve erro de alocador neste log.

**Resultado:** apresentação, redimensionamento, mídia e fechamento visual do
AppImage aprovados nesta amostra, condicionados ao override externo de DMABUF
e à preferência de vídeo já salva. A mudança fornece evidência de um contorno
para o caminho gráfico afetado neste ambiente, sem provar defeito físico de
GPU, memória ou disco. O candidato inicial sem esse override apresentou falhas visuais. A opção
persistente de DMABUF foi incorporada e validada na seção 20; não houve
incorporação durante este teste diagnóstico isolado. A lentidão e o erro de alocador anterior ficam apenas como notas futuras
na seção 23, sem investigação exigida ou bloqueio de aceite.


## 20. Preferência persistente de compatibilidade gráfica Linux

**Estado na revisão de 18/11:** Implementação, testes Rust/frontend e preferência persistida após reinício concluídos; não há build ou validação manual pendente desta seção.

Foi implementada uma opção independente de compatibilidade gráfica em
Settings → Performance → Interface rendering. O modo Normal é o padrão;
Compatibility grava `extra.linux_graphics_compatibility` como booleano no
settings.json existente e informa que a alteração exige reinício. A opção de
compatibilidade de vídeo permanece independente. Os controles são exclusivos
do Linux, aguardam leitura inicial e exibem confirmação apenas após gravação;
falhas de leitura ou gravação não são apresentadas como sucesso.

O novo bootstrap `linux_graphics` lê a preferência no thread inicial, antes de
criar o Builder Tauri, GTK, WebViews ou workers. Quando habilitada, define
`WEBKIT_DISABLE_DMABUF_RENDERER=1`; ausente/desabilitada preserva o caminho
normal. Uma variável explícita no ambiente tem prioridade, inclusive valor 0.
A política de sink GStreamer não é modificada por esse bootstrap. As duas
formas de gravação do serviço (setting individual e objeto completo) validam
o tipo booleano. Foram adicionados schema, estado e acesso pelo hook de
settings, sem alteração manual das preferências do usuário nesta preparação.

A rodada de frontend passou 17 testes (6 de interface gráfica, 6 de interface
de vídeo e 5 do store), incluindo persistência, falhas e visibilidade por
plataforma. A checagem de tipos passou. O lint inicialmente apontou excesso de
complexidade em updateSettings; a gravação da nova preferência foi extraída
para um helper e o lint dos arquivos envolvidos passou após esse ajuste.
Uma repetição isolada do teste de store após o helper atingiu timeout de
inicialização do worker Vitest, sem executar testes, portanto não representa
uma falha de assert nem um resultado aprovado dessa repetição. A execução
posterior com pool de threads e um worker passou os 5 testes do store, já
com o helper final.

Foram adicionados testes Rust de default, leitura das preferências gravadas
pelo serviço, rejeição de valores inválidos e aplicação em subprocessos com
prioridade do ambiente. As sessões de compilação dos testes foram interrompidas
sem registrar resultado final, inclusive após retomada; essa validação Rust
foi concluída na retomada descrita abaixo. Não se atribuiu a interrupção a
uma falha de hardware comprovada. O AppImage anterior não continha a nova
opção; o pacote atualizado e a persistência foram validados nas rodadas seguintes.


### Retomada dos testes Rust e novo AppImage

Evidências recebidas em `/tmp/mundam-graphics-persist/`: `testes-rust.log`,
`build-appimage.log` e `sha256.log`. A compilação dos testes terminou em
12 min 56 s. Os 10 testes selecionados passaram (5 de graphics e 5 de vídeo),
sem falhas, em 0,14 s, com 328 outros testes filtrados. Os casos de aplicação
em subprocessos exercitaram a matriz de preferências e overrides de ambiente;
a entrada auxiliar de teste também aparece nessa contagem. Essa rodada conclui
a verificação Rust que havia ficado pendente nas sessões interrompidas.

O frontend de produção compilou em 38,58 s; a compilação release terminou em
14 min 02 s, seguida da geração de um AppImage. O aviso conhecido de ausência
do símbolo `__TAURI_BUNDLE_TYPE` permaneceu; não houve falha de geração
registrada. O artefato mantém o nome e versão 0.1.0, mas é um candidato novo,
contendo a preferência gráfica persistente.

O arquivo `src-tauri/target/release/bundle/appimage/Mundam_0.1.0_amd64.AppImage`
tem 225.851.896 bytes e SHA-256
`f8ae4cecbaeafd08c7a35611c9e24fc89107946f6fa35b6cd70b395ce0dff047`.
O hash foi recalculado nesta análise e coincide com `sha256.log`; distingue
esse pacote do candidato anterior `e5f976c…`. As capturas anteriores de mídia
e interface não são consideradas teste gráfico deste novo artefato.

**Resultado:** testes Rust selecionados e geração do novo AppImage concluídos.
A gravação e a aplicação da preferência gráfica sem override externo foram
validadas pelo usuário na subseção seguinte.


### Validação da preferência gráfica salva no AppImage

Evidências: `/tmp/mundam-graphics-persist/salvar-20261117-220500.log` e
`/tmp/mundam-graphics-persist/persistida-20261117-220711.log`. O usuário
confirmou execução dos passos propostos: primeira abertura com override
externo, gravação de Graphics mode → Compatibility e fechamento pelo X;
segunda abertura sem override externo, recuperação das duas preferências,
interface legível e sem glitches, reprodução de MP3/MP4/M2TS com waveform,
seek e velocidade, e fechamento com Alt+F4 com M2TS tocando.

Na primeira execução, o bootstrap gráfico registrou
`Linux graphics renderer override preserved source="environment"`, enquanto
o modo de vídeo já vinha das configurações. Na segunda, os dois bootstraps
registraram `compatibility_enabled=true source="settings"`. Isso confirma
que o ajuste gráfico passou a ser aplicado pela preferência persistida,
sem depender do override temporário para essa segunda abertura. O relato
visual confirma que splash, janela principal e finalização funcionaram.

O log da segunda execução contém reprodução HTTP de áudio/vídeo e HLS por
MediaSource, com posição HLS avançando para 53,74 e 61,48 s. Um evento registrou
24 frames totais e zero frames descartados; esse snapshot não é uma medição
de fluidez da sessão inteira. Waveform e velocidade foram confirmados pelo
usuário, sem inferir sua validação apenas desses eventos de reprodução.

| Evidência                       | Gravação                | Preferência persistida  |
| ------------------------------- | ----------------------- | ----------------------- |
| Tarefas registradas encerradas  | 13, `clean=true`        | 13, `clean=true`        |
| Pool SQLite                     | 20 conexões, 20 ociosas | 20 conexões, 20 ociosas |
| Espera por conexões ociosas     | 0 ms                    | 0 ms                    |
| Fechamento do pool              | 293 ms                  | 498 ms                  |
| Pedido até conclusão do backend | aproximadamente 295 ms  | aproximadamente 501 ms  |
| Apresentação reconhecida        | `timely=true`           | `timely=true`           |
| Código da imagem                | 0                       | 0                       |

Nenhum processo Mundam, WebKit, FFmpeg ou FFprobe apareceu no filtro das quatro
fotografias após cada retorno, incluindo a primeira e a última, cerca de
15 segundos depois. Não houve erro de alocador, timeout do pool ou aviso HLS
nesses dois logs. O aviso conhecido de fallback do clipboard Wayland para X11
permaneceu. Os snapshots finais indicaram cerca de 5,5–5,6 GiB de RAM disponível;
não representam monitoramento de pico. O fechamento com mídia registrou zero
cancelamentos de processos de segmento, portanto essa amostra não substitui
o teste anterior que efetivamente aguardou um processo de segmento ativo.

**Resultado:** preferência gráfica persistente, apresentação, mídia e
fechamento do novo AppImage aprovados nestes cenários no host atual, com
compatibilidade gráfica e de vídeo habilitadas. A falha visual do candidato
anterior foi contornada pela opção persistente neste ambiente. Não se estende
o aceite a outras máquinas, ao modo Normal neste host ou a toda a matriz de
shutdown. O erro intermitente de corrupção de memória anterior não foi
reproduzido nem explicado; fica somente como nota futura, sem pendência ou
atribuição técnica confirmada a hardware. O Debian anteriormente testado não contém a nova
preferência gráfica e não foi regenerado nesta rodada.


## 21. Investigação inicial do ambiente: pressão de memória e armazenamento

**Estado na revisão de 18/11:** Diagnósticos históricos do ambiente e notas futuras, sem pendência ou investigação obrigatória da aplicação.

O usuário relatou alertas frequentes de consumo de memória também no terminal,
Arquivos e outras aplicações, suspeitando do systemd-oomd e do HD antigo com
histórico de problemas. Informou que os testes Windows nesta mesma máquina
usaram um SSD mais novo. Essa comparação altera simultaneamente sistema,
armazenamento, drivers e WebView; não isola a causa dos erros Linux.

A consulta de leitura ao journal de 17 de novembro, entre 08:00 e 00:00 do dia
seguinte, encontrou 18 registros de encerramento pelo systemd-oomd. As cópias
foram preservadas em `/tmp/mundam-diagnostico-host/oomd.log` e `kernel.log`.
Há encerramentos de scopes Chromium, VS Code e ptyxis-spawn, além do D-Bus
da sessão e GNOME Shell. Entre os registros relevantes:

| Horário local | Alvo registrado | Pressão informada no user@1000.service |
| ------------- | --------------- | -------------------------------------- |
| 09:50:03      | ptyxis-spawn    | 78,43%                                 |
| 20:38:25      | Chromium        | 58,31%                                 |
| 20:38:40      | VS Code         | 78,42%                                 |
| 20:54:36      | Chromium        | 84,32%                                 |
| 20:54:52      | GNOME Shell     | 67,08%                                 |

Os registros indicam pressão acima de 50% por mais de 20 s com atividade de
reclaim. Esses percentuais são métricas de pressão, não percentual de RAM
ocupada. A [documentação do systemd-oomd](https://www.freedesktop.org/software/systemd/man/258/systemd-oomd.html)
explica sua atuação por cgroups e PSI. Os eventos da noite são temporalmente
compatíveis com as interrupções de sessão durante as checagens. O registro
de 09:50 é compatível com o horário da execução de áudio interrompida relatada
anteriormente, sem identificação suficiente da árvore do Mundam para atribuir
seu encerramento com certeza. A confirmação de ação do oomd não identifica
sozinha o consumidor responsável pela pressão ou erro de configuração do daemon.

O lsblk identificou `/dev/sda`, modelo ST1000LM024 HN-M101MBB, 931,5 GiB e
ROTA=1, com a raiz em sda2. O smartctl não estava disponível para consulta.
Na captura atual, havia 5,5 GiB de RAM disponível e 2,1 GiB de swap ocupada;
PSI memória avg10/avg60 era zero, enquanto PSI I/O avg300 indicava some=4,30%
e full=3,77%. São snapshots sem correlação suficiente com as falhas históricas.
O filtro dos registros de kernel disponíveis não encontrou erros de I/O,
ATA, ext4, OOM do kernel ou machine check com os padrões consultados; isso não
é teste de saúde do disco ou RAM nem exclui falhas intermitentes.
A consulta oomctl não acessou o system bus dentro do ambiente restrito.

A mensagem `free(): corrupted unsorted chunks` corresponde a uma verificação
de integridade das estruturas do heap no
[código do alocador glibc](https://raw.githubusercontent.com/bminor/glibc/master/malloc/malloc.c).
É distinta de um encerramento por pressão de memória. Nenhum dos 18 registros
de kill consultados ocorreu no minuto do erro do Debian, às 19:22. Não há
correlação demonstrada entre o erro do alocador e esses kills ou falha de HD.

**Resultado:** ação recorrente do systemd-oomd no ambiente confirmada;
instabilidade de sessões possui evidência externa ao Mundam. Saúde do HD e
integridade física da RAM permanecem sem diagnóstico. A causa da pressão e
a ocorrência intermitente de corrupção de heap não tiveram causa determinada.
Permanecem apenas notas futuras, sem defeito confirmado ou pendência de aceite.


### SMART, política oomd e amostra de utilização recebidos

Evidências: `smart-sda.log`, `oomctl.log` e `estado-host.log`, na pasta
`/tmp/mundam-diagnostico-host/`, coletadas às 22:39 locais de 17 de novembro.

O oomctl confirmou modo ativo (`Dry Run: no`), monitoramento de
`user@1000.service` por pressão com limite de 50% e duração de 20 s. O default
global exibido era 60%, mas o limite efetivo desse cgroup era 50%. Nenhum
cgroup apareceu na lista de monitoramento por swap; o limite global de swap
exibido como 90% não comprova que os kills anteriores tenham ocorrido por esse
critério. Os registros anteriores identificam explicitamente pressão e reclaim.
Esses percentuais não são ocupação de RAM. Não foi demonstrado erro de
configuração do daemon; sua ação é coerente com os limites exibidos.

No snapshot havia cerca de 5,5 GiB de RAM disponível, 2 GiB de swap ocupada e
PSI de memória zerado nas três médias. Havia dois arquivos de swap, de 4 e 8 GiB.
As amostras de um segundo do vmstat mostraram swap-out zero e swap-in pontual,
sem thrashing sustentado nessa janela. A primeira linha do vmstat é uma média
desde o boot e não foi interpretada como atividade daquele segundo. Houve
I/O wait pontual de até 10% nas linhas seguintes, com PSI I/O avg60 some=7,23%
e full=7,01%. Esses dados não diagnosticam falha de HD. A CPU de sistema ficou
em cerca de 25–26% na maior parte da amostra, apesar do baixo uso de aplicações;
a origem não foi identificada. A lista de processos não continha Mundam entre
os maiores consumidores apresentados; ela não identifica os responsáveis
pelos picos anteriores nem prova ausência de vazamento em outras execuções.

O SMART identificou HD de 5400 rpm e 34.642 horas ligado. O resultado global
foi PASSED, com zero setores realocados, pendentes e offline incorrigíveis nos
atributos atuais, temperatura de 32 °C e nenhum self-test registrado. Em
contrapartida, há histórico de 63.099 erros de dispositivo, com os oito últimos
registros mostrando erros UNC de leitura às 19.938 horas de vida. São
registros antigos, aproximadamente 14.704 horas de funcionamento anteriores
à leitura atual, e não erros demonstrados na sessão de testes. O total não
representa quantidade de setores defeituosos distintos.

O atributo de CRC UDMA registra 59 ocorrências e Load_Cycle_Count registra
2.278.977 (normalizado 1, threshold 0). São contadores históricos; não houve
segunda leitura que demonstrasse crescimento ou diagnóstico do componente
responsável pelos erros de comunicação. Valores brutos específicos do
fabricante de outros atributos não foram tratados como contagens universais
de falhas. PASSED não equivale a uma varredura de superfície bem-sucedida nem
anula o histórico de erros UNC. Nenhum autoteste adicional ou reparo foi
executado nesta análise.

**Resultado:** pressão histórica de memória com ação do oomd confirmada;
sem pressão de memória significativa durante a amostra atual. O HD tem
histórico comprovado de leituras incorrigíveis e uso prolongado, mas falha
ativa durante os testes não foi demonstrada. O ambiente é um fator de
confusão relevante para desempenho e interrupções; não há evidência suficiente
para atribuir a ele a corrupção de heap ou excluir problemas de software.
A comparação Windows/SSD não isola armazenamento de diferenças de sistema
e bibliotecas. A validação funcional já obtida nos cenários limpos permanece
registrada, sem converter o erro intermitente em defeito de hardware confirmado.


## 22. Preparação de reprodução com biblioteca nova e mídia concorrente

**Estado na revisão de 18/11:** Tentativas de reprodução concluídas; resultados degradados ficam como observações históricas, sem bloquear a sessão Linux aceita.

O usuário solicitou reproduzir o erro intermitente durante nova indexação,
gerando thumbnails, cores e fingerprints enquanto reproduz vídeos. A hipótese
de origem em FFmpeg/FFprobe foi mantida como hipótese; as capturas anteriores
não identificam o PID emissor da mensagem de corrupção de heap.

Foi preparado um coletor externo, sem alteração do produto, em
`/tmp/mundam-diagnostico-host/reproduzir-indexacao.py`. Ele cria perfis únicos
sob `src-tauri/target/linux-repro/`, com XDG_DATA_HOME, XDG_CACHE_HOME,
XDG_CONFIG_HOME e XDG_STATE_HOME isolados. Não apaga o banco/cache existentes
nem arquivos de assets. Os dois overrides gráficos já validados são aplicados
apenas ao ambiente da execução; o restante das configurações parte dos defaults
no perfil novo. Não há aumento artificial de concorrência ou desativação do
oomd. O caminho do banco foi conferido no bootstrap do produto como
app_local_data_dir()/mundam.db.

A coleta preparada registra hash do AppImage, perfil, código de retorno,
stdout/stderr, snapshots de processos e /proc a cada dois segundos, journal
de oomd/kernel/crashes no intervalo e metadados dos relatórios de /var/crash
antes/depois. O host usa Apport como core_pattern e não disponibilizava
coredumpctl; não foi alterada a configuração de core dumps. O coletor não usa
ptrace/strace para não alterar nesta rodada o sandbox WebKit nem adicionar
sobrecarga de tracing às operações. Processos muito curtos podem escapar dos
snapshots; não se promete atribuição de todo stderr sem um crash identificado.

Foram preparados dois cenários com perfis independentes, um de indexação sem
player e outro de indexação com mídia, para separar reprodução concorrente de
processamento de biblioteca. A preparação não executou os cenários; os resultados efetivos estão nas
subseções seguintes. A sintaxe Python e a entrada de ajuda foram
verificadas; não houve abertura de interface ou novo resultado funcional.
Os logs são arquivados separadamente do banco e dos caches, preservando o
perfil para eventual investigação posterior.


### Primeira tentativa do coletor: interação bloqueada antes da indexação

Perfil: `src-tauri/target/linux-repro/20261117-231754-982728-A-indexacao`.
O usuário relatou impossibilidade de focar a janela ou clicar para utilizar
a aplicação. O log atingiu BackendReady às 23:18:02 locais, com zero roots
e fingerprints=0, e não registrou início de indexação. Essa execução não
reproduziu a carga de thumbnails/cores/fingerprints solicitada.

O manifesto confirmou banco criado dentro do perfil isolado, AppImage com
hash `f8ae4cec…`, retorno 0 e ausência de alteração dos metadados dos relatórios
em /var/crash. O backend encerrou as 12 tarefas registradas com `clean=true`
e fechou o SQLite em 225 ms, sem boot_scan porque não havia roots. Não houve
mensagem de corrupção de heap no log do aplicativo, nem mensagem explícita
de falha de foco. Esses resultados não validam a interação visual relatada
como bloqueada e não diagnosticam a causa.

A revisão 1 do coletor isolava quatro variáveis XDG. Foi preparada a revisão
2 com somente XDG_DATA_HOME isolado, suficiente para o banco e caches locais
baseados em app_local_data_dir do Mundam. XDG_CONFIG_HOME, XDG_CACHE_HOME e
XDG_STATE_HOME passam a ser herdados do terminal, reduzindo mudanças no
ambiente de desktop. A revisão mantém os overrides gráficos e a coleta de
logs; o perfil anterior foi preservado. A modificação não é uma correção
comprovada da falta de foco: a repetição posterior voltou a expor a ocorrência,
hoje mantida apenas como nota futura.
A sintaxe do script revisado foi verificada sem abrir nova instância.


### Coletor revisão 2: A com trabalho pendente, B bloqueado por foco

Evidências recebidas: `20261117-232711-233315-A-indexacao-logs.tar.gz` e
`20261117-232921-469230-B-indexacao-midia-logs.tar.gz`, na pasta de diagnóstico.
Ambos identificam o AppImage `f8ae4cec…` e revisão 2 com somente XDG_DATA_HOME
isolado, overrides gráficos aplicados por ambiente e banco criado no perfil.

No A, o usuário conseguiu indexar e fechar com processos pendentes. O scan
registrou 787 assets novos, zero erros e conclusão em 2,07 s. O pedido de
fechamento ocorreu às 23:27:59.133 locais, cerca de 4,5 s após o início do scan.
A enumeração havia acabado, mas não o processamento derivado: o thumbnail worker
registrou duas tarefas pendentes e o worker de duplicatas aguardou um
fingerprint_batch até 23:27:59.472. Um handler de asset estava ativo na admissão
e foi drenado. As 13 tarefas terminaram com clean=true, SQLite fechou em 4 ms
(7 conexões ociosas de 7), e o backend concluiu em aproximadamente 345 ms.
A apresentação teve timely=true e o launcher retornou 0.
Não houve mensagem de corrupção de heap, novo relatório em /var/crash ou evento
oomd no intervalo. A captura de processos não flagrou FFmpeg/FFprobe; por sua
resolução de dois segundos, não exclui utilitários curtos entre snapshots.
O log não comprova trabalho de extração de cores em voo no instante do fechamento.

No B, o usuário voltou a não conseguir focar/interagir. O log ficou com zero
roots, sem scan de biblioteca nem reprodução: não é execução válida do cenário
concorrente solicitado. O backend encerrou 12 tarefas com clean=true, fechou
SQLite em 361 ms e concluiu cerca de 365 ms após o pedido, com timely=true e
retorno 0. Também não houve evento oomd, novo relatório Apport ou corrupção de
heap. A menor RAM disponível amostrada foi aproximadamente 4216 MiB no A e
4274 MiB no B; não são medições contínuas de pico. Os últimos snapshots não
mostraram Mundam, WebKit, FFmpeg ou FFprobe no filtro. Os logs de kernel
contêm os avisos conhecidos de firmware nouveau/msvld e permissões de
fusermount3 em ambos os cenários, sem distinção que explique por si só a falha
de foco exclusiva da segunda tentativa.

**Resultado da rodada:** A validou fechamento com processamento pendente, sem
reproduzir o erro intermitente. B ficou bloqueado antes da carga nessa tentativa,
mas foi executado nas rodadas seguintes; foco fica somente como nota futura.
O sucesso de A não demonstra que a mudança de
isolamento resolveu o foco: sua recorrência com a mesma revisão invalida essa
conclusão. Foi preparada revisão 3 do coletor, acrescentando captura de
propriedades X11 das janelas Mundam e do foco ativo a cada aproximadamente dez
segundos, sem comandos para alterar foco ou comportamento do aplicativo.
A revisão foi verificada sintaticamente e depois executada nos cenários abaixo.


### Cenário B revisão 3: reprodução concorrente com pressão e fechamento degradado

Evidência: `20261117-234120-320327-B-indexacao-midia-logs.tar.gz`, em
`/tmp/mundam-diagnostico-host/`. Perfil novo, coletor revisão 3 e mesmo AppImage
`f8ae4cec…`. O usuário conseguiu interagir, indexar e reproduzir áudio/vídeo,
confirmando imagem, som, waveform, seek e velocidade. Relatou travamentos,
pausas no processamento e recuperação gradual, interpretados como interferência
do oomd. A falha anterior de foco não ocorreu nesta amostra; a janela principal
foi capturada visível, aceitando foco e com _NET_WM_STATE_FOCUSED. Isso não
explica a intermitência das tentativas bloqueadas.

O scan inicial descobriu 787 assets novos sem erros em 2,48 s, às 23:41:55–58
locais. Um scan posterior concluiu 787 inalterados, sem erros, em 11,12 s.
Houve extração e reprodução efetivas, mas também erros de previews CorelDRAW,
Penpot e JPEG embutido, retries de cores por ausência de pixels opacos e falhas
de decode de fingerprints (incluindo AVIFS/HEIC/RIF), além de fontes ausentes.
Esses diagnósticos de formatos não foram classificados como corrupção de heap
ou automaticamente atribuídos a hardware. Houve aquisição SQL lenta de até
6,05 s e consulta de preview de 3,32 s, compatíveis com a degradação observada,
sem identificação suficiente da causa individual de cada espera.

O journal de oomd no intervalo veio sem entradas, e o journal de crashes
filtrado também não trouxe ocorrências. Os metadados de /var/crash não mudaram.
Assim, não se confirmou novo kill pelo oomd durante B. O kernel/journald
registrou pressão de memória às 23:42:16, 23:42:23 e 23:51:28–34, além dos
avisos conhecidos de nouveau/msvld e fusermount3. Não houve erro explícito de
I/O de disco nessas entradas. A interpretação do usuário foi distinguida dos
eventos efetivamente registrados, sem negar os travamentos percebidos.

Nas 295 fotografias, PSI memory some avg10 chegou a 63,40% às 23:42:30
(full 61,31%); PSI I/O some avg10 chegou a 74,16% às 23:52:00. Houve swap-in e
swap-out no intervalo. A menor memória disponível amostrada foi aproximadamente
2779 MiB. Esses percentuais medem espera/pressão, não fração de RAM ocupada.
O Mundam chegou a RSS de aproximadamente 1802 MiB, WebKitWebProcess a 612 MiB,
e um FFmpeg a 624 MiB; são máximos individuais em instantes diferentes, não
somáveis como pico simultâneo. Houve FFprobe/FFmpeg e processos em estado D,
confirmando atividade e espera ininterruptível, sem comprovar defeito do HD.
O contador oom_kill do kernel não comprova nem exclui ação de oomd userspace.

O HLS registrou cancelamentos por Superseded com terminação de segmento
aguardada e SIGKILL, acompanhados de fragLoadError 503 não fatal. Esses
SIGKILL têm motivo explícito de substituição de operação e não foram atribuídos
a oomd. Houve também bufferStalledError e bufferSeekOverHole não fatais.
A sessão não foi considerada isenta de interrupções apenas porque a mídia
voltou a reproduzir.

No fechamento, às 23:52:05.894, havia duas tarefas de thumbnail pendentes
(extração/publicação), fingerprint_batch e um segmento em execução. O segmento
foi cancelado com reason=Shutdown e terminação aguardada. As 13 tarefas
registradas terminaram com clean=true às 23:52:06.252. O pool SQLite tinha
13 conexões ociosas de 13 e idle_wait_ms=0, mas não fechou em 3 s: o backend
concluiu com clean=false às 23:52:09.253, aproximadamente 3,36 s após o pedido.
Não houve reconhecimento timely=true da apresentação no log dessa amostra.

O launcher retornou 0 às 23:52:18, cerca de 12 s após o pedido. Isso não
significa encerramento limpo de toda a árvore: às 23:52:27.807 o WebKitWebProcess
PID 171808 emitiu `WebProcess didn't exit as expected after the UI process
connection was closed`. Ele ainda aparecia na última fotografia, às 23:52:33,
estado ILl+, aproximadamente 453788 KiB de RSS. O processo de rede deixou de
aparecer antes; não se demonstrou saída do WebProcess no intervalo coletado.

**Resultado:** cenário concorrente executado, com reprodução confirmada pelo
usuário e forte pressão de memória/I/O observada. A corrupção de heap não foi
reproduzida. Esta amostra registrou fechamento degradado por timeout SQLite e
WebProcess remanescente com erro, apesar de retorno principal 0. É uma
observação histórica, sem pendência vigente de aceite Linux.
Não há novo kill oomd confirmado nesse intervalo. Pressão de ambiente e carga
da aplicação são fatores correlacionados, sem causa única demonstrada;
o desfecho fica preservado como nota futura, sem tarefa ou bloqueio de aceite
Linux, por decisão do usuário em 18/11. Não se registra causa física comprovada
nem se abre defeito da aplicação sem atribuição demonstrada.


### B com redução de concorrência após a tela de welcome (18 de novembro)

Evidência: `20261118-000029-776536-B-indexacao-midia-logs.tar.gz`, na pasta de
diagnóstico. O usuário esclareceu que o primeiro uso mostra uma tela de welcome
sem acesso a Settings. Assim, iniciou a indexação e somente depois reduziu o
limite, confirmou uma operação de processamento por vez e fechou com Alt+F4
com M2TS tocando. O roteiro anterior pressupunha incorretamente acesso às
configurações antes da indexação; a execução foi considerada de configuração
mista no tempo, não limitada a uma operação desde o início.

O perfil local preservado contém processing_concurrency=1, thumbnail_threads=1,
indexer_concurrency_limit=200 e auto_detect_duplicates=true. Essa leitura
confirma o valor final, sem fornecer o instante exato de mudança. O limite
compartilhado de workers não deve ser interpretado como limite global de um
único processo do sistema, inclusive para reprodução/transcodificação.

O scan descobriu 787 assets sem erros em 4,00 s. A sessão durou aproximadamente
3 min 36 s até o retorno, contra aproximadamente 10 min 58 s na execução B
anterior. São cargas acumuladas e durações diferentes; a comparação não isola
apenas a concorrência nem estabelece causalidade por duas amostras.

O fechamento às 00:04:04.303 locais tinha uma tarefa de thumbnail pendente,
trabalho de fingerprint em content_hash e segmento HLS ativo. O segmento 11
foi cancelado com reason=Shutdown e terminação aguardada (SIGKILL). As 13
tarefas registradas terminaram com clean=true. O pool tinha 7 conexões, todas
ociosas, idle_wait_ms=10, e fechou em 824 ms. O backend concluiu com clean=true
às 00:04:05.185, aproximadamente 883 ms após o pedido. O manifesto registra
retorno 0 às 00:04:05 e fim de coleta às 00:04:20. Não há reconhecimento
`timely=true` da apresentação nessa captura; não foi inferido a partir do
código de saída. Os últimos quatro snapshots não mostraram Mundam, WebKit,
FFmpeg ou FFprobe no filtro. Não houve erro de saída de WebProcess nesta rodada.

Houve fragLoadError 503 não fatal após substituição de segmentos e 503/status 0
no segmento cancelado durante shutdown. Esses eventos foram mantidos como
diagnósticos do cancelamento, sem classificá-los como novos crashes. Nenhuma
mensagem de corrupção de heap apareceu, nenhum metadado de /var/crash mudou
e não houve entrada de oomd ou crash no journal filtrado do intervalo.
Os avisos conhecidos de nouveau/msvld e fusermount3 permaneceram.

Nas 111 fotografias, a menor RAM disponível foi aproximadamente 3907 MiB,
o maior RSS do Mundam aproximadamente 890 MiB, de um FFmpeg 151 MiB e do
WebKitWebProcess 439 MiB. Os picos individuais não são simultâneos. PSI memory
some avg10 máximo foi 14,43%, contra 63,40% na execução anterior. O máximo de
PSI I/O some avg10 ainda foi 72,06%, durante a abertura às 00:00:45; esse pico
não ocorreu necessariamente durante o mesmo estágio de processamento do
máximo anterior. Menor pressão de memória e fechamento limpo nesta amostra
são compatíveis com benefício da carga reduzida, sem demonstração de que
somente esse ajuste resolveu o timeout ou excluiu defeito de software.

**Resultado:** fechamento com mídia e trabalho pendente aprovado nesta
repetição, após reduzir a concorrência durante a sessão. Não reproduziu o
timeout do SQLite, o WebProcess remanescente nem a corrupção de heap. A amostra
anterior com clean=false permanece como resultado histórico sem causa
definitiva e foi transferida para notas futuras, sem pendência de aceite. O fluxo
de welcome limita a preparação manual de testes com perfil novo; não foi feita
alteração no produto ou coletor nesta análise.


## 23. Conclusão da sessão Linux e notas para investigação futura (18 de novembro)

**Decisão do usuário:** sessão Linux concluída. Foram aceitos os testes em dev
com backend release, execução direta, Debian e AppImage, incluindo mídia e
fechamento sob processamento. O usuário também relatou testes adicionais fora
dos logs desta sessão e considera as ocorrências intermitentes relacionadas
ao computador ou à instalação do Ubuntu; não as considera problemas da aplicação.
Esta revisão incorpora essa decisão de escopo e aceite sem declarar uma causa
forense que os registros não provaram.

### Evidências que encerram a sessão

- Saídas por X/Alt+F4, splash, pedidos repetidos com uma apresentação, principal
  minimizada e saída pelo dock em desenvolvimento, conforme seção 15.
- Correção de mídia nativa HTTP/classificação HLS e regressão funcional de áudio,
  vídeo, waveform, seek e velocidade; IPC/apresentação indisponíveis com saída
  autônoma nos testes injetados, conforme seção 16.
- Compatibilidade de vídeo persistente e execução direta sem wrapper, seção 17.
- Debian instalado e testado ocioso/com mídia, com repetição e observação posterior
  dos processos, seção 18; não foi regenerado com a última opção gráfica e isso
  não é requisito adicional para a sessão aceita.
- AppImage final `f8ae4cecbaeafd08c7a35611c9e24fc89107946f6fa35b6cd70b395ce0dff047`,
  preferência gráfica salva e recuperada sem override externo, seção 20.
- Biblioteca nova com 787 assets, fechamento com thumbnails/fingerprints e
  segmento HLS ativos, inclusive repetição com concorrência reduzida e
  clean=true/saída 0, seção 22. A configuração foi alterada depois do welcome,
  portanto não se afirma uma carga limitada desde o primeiro instante.

### Notas futuras — sem pendência ou bloqueio de aceite

| Observação preservada                                                                  | Classificação vigente                                                                                                                                                                     |
| -------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| free(): corrupted unsorted chunks e double free or corruption (out)                    | Mensagens intermitentes sem PID/stack suficiente para atribuir ao Mundam, FFmpeg ou FFprobe. Não reproduzidas nas repetições finais; somente nota futura.                                 |
| Timeouts SQLite/worker, clean=false, demora e WebProcess remanescente sob carga        | Resultados degradados históricos preservados. Não se declara que foram clean=true, mas também não geram pendência de correção ou reteste da sessão Linux sem causa atribuída ao produto.  |
| Foco/interação bloqueados em algumas aberturas com perfil novo                         | Não ocorreram em todas as tentativas nem impediram as execuções finais. Sem causa isolada; nota futura, sem defeito confirmado.                                                           |
| Alertas de memória, pausas, pressão PSI/I/O e ação do oomd                             | Há kills comprovados em outros horários, mas não em todos os intervalos relatados. Contexto do ambiente, sem atribuição automática de cada travamento ao daemon ou a vazamento do Mundam. |
| HD antigo, SMART com erros UNC históricos e CPU de sistema elevada                     | Diagnóstico parcial do host; sem falha atual demonstrada no instante de cada ocorrência. Investigação/manutenção do computador fora das pendências da aplicação.                          |
| nouveau/firmware, clipboard, fusermount, erros internos WebKit e avisos HLS não fatais | Diagnósticos preservados. A compatibilidade gráfica/vídeo funcional foi oferecida e validada; avisos sem impacto atribuído ao produto não reabrem o aceite.                               |
| Aviso __TAURI_BUNDLE_TYPE e metadados de licenças do empacotador                       | Não impediram geração ou execução. Updater/auditoria de distribuição não foram objeto deste aceite e não se tornam tarefas novas desta implementação.                                     |
| Formatos/previews/fontes específicos e resultados de extração conhecidos               | Fora do trabalho atual, conforme delimitação de escopo, sem reabrir a campanha de suporte a formatos.                                                                                     |

Essas notas só voltarão a trabalho ativo por nova decisão do usuário ou
reprodução com evidência suficiente para atribuir um problema ao produto.
Não há obrigação de continuar investigando hardware/Ubuntu ou de provar sua
culpa para concluir a sessão. Os limites arquiteturais concretos de R1–R4
são independentes dessas ocorrências e permanecem listados como trabalho geral.

**Pendências ao fim desta revisão Linux:** R1, R2, R3, R4, R5, R7, R8 e R9.
R6 encerrado. Esta revisão Linux modificou apenas documentação; nenhum teste
adicional, alteração de código ou nova build foi realizado nela. Posteriormente,
R1 foi implementado e concluído nos cenários da seção 26; o estado vigente dos
itens está na seção 14.

## 24. R1: propriedade e encerramento explícito do watcher (18 de novembro)

**Escopo:** implementação compartilhada de R1. O aceite Linux de R6 permanece
concluído. Esta etapa não altera suporte a formatos, thumbnails, políticas de
rede, apresentação de encerramento nem admissão geral de IPC (R3).

### Implementação e política

- `bootstrap/library.rs` admite um único dono `library_watcher` no lifecycle
  antes de observar qualquer raiz. O mesmo token governa raízes do boot,
  indexação manual e reconciliação. Se o dono não for admitido, o token é
  cancelado e nenhuma observação é iniciada.
- `sensor.rs` serializa registro, remoção e encerramento. Após cancelamento,
  recusa novas raízes e reconciliação. Retém os handles da operação nativa e do
  pipeline; cancelar o pedido ou a espera de shutdown não destaca esses donos.
- `native.rs` executa registro, remoção e destruição em `spawn_blocking` retido.
  O `Drop` de notify 8.2.0 no Windows/Linux solicita parada, mas não oferece
  join público. Por isso, além de destruir o watcher, a operação aguarda uma
  confirmação enviada na destruição do callback, incluindo suas referências
  em requisições nativas pendentes. Isso comprova liberação da origem/callback,
  sem afirmar um join do último epílogo da thread interna da dependência.
- `runtime.rs` mantém debouncer e publicador como futures unidas numa única
  tarefa aguardada. O cancelamento fecha as filas, liberando também callbacks
  presos em `blocking_send` por fila cheia. O publicador verifica cancelamento
  antes de publicar; `shutdown` só conclui depois da espera dos dois loops.
- Notificações e renomes ainda voláteis são descartados, com contadores de
  diagnóstico. Não se força leitura nem publicação desses hints ao sair.
  A reconciliação continua a cargo do boot scan existente; não foi adicionada
  fila persistente nem garantia nova de identidade para renomes não consolidados.
  Arquivos originais, transações e leases não são modificados por essa política.
- Uma leitura de filesystem já iniciada pelo debouncer é aguardada. O orçamento
  geral de cinco segundos do lifecycle continua vigente: timeout ou falha de
  dono degrada o encerramento. Não se promete interromper I/O nativo bloqueado;
  esse limite permanece em R4. Falhas de supervisão são preservadas em chamadas
  repetidas de shutdown e comunicadas ao lifecycle.

### Verificação automatizada

Cobertura dirigida: eventos reais antes/depois da parada no Windows, remoção de
raízes adotadas, recusa de admissão e reconciliação tardias, shutdown concorrente
e idempotente, corrida com registro nativo, cancelamento do solicitante/espera
sem perda do handle, filas bruta/de domínio cheias, renome pendente, falhas dos
donos e espera pelo lifecycle.

Testes Rust dirigidos ao módulo do watcher:
**17 aprovados**, nenhum erro ou teste ignorado, incluindo a confirmação de
liberação nativa e integração com o lifecycle. Execução Windows com toolchain
`stable-x86_64-pc-windows-gnu`, perfil de teste não otimizado; não é uma build
release/MSVC nem validação do aplicativo empacotado.

Clippy do código de produção, com avisos tratados como erros:
**aprovado**, sem avisos no código de produção.
Testes Rust dirigidos ao registro lifecycle:
**9 aprovados**, incluindo deadline compartilhado, falhas propagadas, recusa de
registro tardio e bridge mantido até o fim dos produtores. Total dirigido desta
etapa: **26 testes aprovados**. Não foi executada a suíte completa da aplicação.
As verificações de formatação Rust e whitespace do diff passaram.
O Clippy incluindo testes, com avisos tratados como erros,
encontrou três ocorrências preexistentes, confirmadas no HEAD desta sessão:
dois `expect_used` em `bootstrap/database_tasks.rs:75,87` e um `while_let_loop`
em `delivery/streaming/server.rs:1007`. Não apontou ocorrência nos arquivos de
R1. Esses testes não foram modificados nesta etapa; a análise incluindo testes
não é registrada como aprovada.

### Validação dirigida no aplicativo

Primeira rodada executada e analisada na seção 25: funcionamento visual confirmado,
mas liberação nativa não comprovada devido ao timeout. A correção e a revalidação
da rodada v2 estão concluídas nos cenários dirigidos da seção 26.
Os testes cobriram encerramento com uma raiz local ativa, após criação e renomeação
de cópias descartáveis, seguido de reabertura para verificar a reconciliação e
encerramento ocioso. A análise verificou fechamento da admissão, liberação da
observação nativa, conclusão dos loops e de `library_watcher` antes do bridge e
do fechamento explícito do pool, sem timeout e com `clean=true`.
Essa evidência integra o histórico de R5, cuja validação do bundle Windows
continua pendente; o aceite Linux permanece preservado.

**Próxima etapa prevista após concluir R1:** R2, ownership dos loops auxiliares
HTTP e dos caminhos HLS por IPC, respeitando os limites da seção 14.

## 25. R1: validação Windows e correção de registro duplicado (18 de novembro)

**Relato do usuário:** os dois cenários funcionaram como esperado na interface.
A inspeção dos logs, porém, demonstrou que o critério de encerramento de R1
não foi atendido naquela rodada. A liberação do watcher ficou pendente até a
revalidação v2 da seção 26; o aceite Linux e as delimitações de escopo foram preservados.

| Log                                     | Pedido até conclusão do backend | Resultado do watcher e encerramento                                                                                        |
| --------------------------------------- | ------------------------------: | -------------------------------------------------------------------------------------------------------------------------- |
| `mundam-watcher-r1-20261118-030526.log` |                      5.013,3 ms | Raiz adicionada durante a sessão; mesmo caminho registrado duas vezes. Timeout/abort de `library_watcher`, `clean=false`.  |
| `mundam-watcher-r1-20261118-033850.log` |                      5.004,3 ms | Raiz recuperada no boot; mesmo caminho novamente registrado duas vezes. Timeout/abort de `library_watcher`, `clean=false`. |

As duas execuções usaram desenvolvimento com backend release. Não houve
`Watcher native observation released`, conclusão dos loops nem
`Watcher shutdown completed`. O bridge terminou após o timeout, e o fechamento
explícito do pool foi corretamente dispensado devido à drenagem incompleta.
Isso não é evidência de corrupção ou erro SQLite. A segunda abertura confirmou
uma raiz persistida e boot scan com 12 arquivos inalterados e zero erros;
a confirmação visual de funcionamento/reconciliação é atribuída ao usuário.

Os avisos Tao da primeira execução e `Chrome_WidgetWin_0 / Error = 1412` ao fim
das duas ficam registrados como observações de runtime, sem causa estabelecida
para o timeout nem nova frente de suporte a mídia/gráfica.

### Causa e correção

Boot/indexação e reconciliação podiam pedir a observação da mesma raiz usando
snapshots diferentes. A serialização em `WatcherService` não verificava se o
caminho já havia sido registrado. Em notify 8.2.0, o backend Windows executa
`watches.insert(path, ws)` sem parar o registro anterior da mesma chave. Com
isso, seu handle/pedido anterior deixa de constar do conjunto que é encerrado;
uma referência ao callback pode permanecer retida e impedir a confirmação de
liberação aguardada por R1. Os dois registros do mesmo caminho constam em cada
log. Essa sequência no código explica o timeout observado; não foi obtido dump
das threads do processo do usuário.

`watch` agora verifica o conjunto de caminhos registrados sob a mesma trava
que serializa operações, depois de aguardar e incorporar a operação anterior.
Um pedido repetido retorna sem chamar novamente o backend nativo. A comparação
usa o caminho registrado, sem inferir identidade por conversão de caixa ou por
contenção: grafias diferentes continuam com registros próprios, inclusive em
diretórios Windows com sensibilidade a caixa habilitada.

Foram acrescentados logs debug antes da destruição nativa e antes da espera
pelo callback. A regressão de eventos reais agora repete `watch` na mesma raiz;
um teste adicional cobre pedidos simultâneos, repetição de grafias variantes
no Windows e encerramento com confirmação de liberação. A cobertura anterior
da seção 24 não incluía esse registro duplicado.

**Verificação da correção:**

- Testes Rust dirigidos ao módulo do watcher:
  **18 aprovados**, nenhum erro/ignorado, incluindo repetição sequencial,
  pedidos simultâneos, grafias variantes Windows e liberação nativa real.
  Toolchain GNU Windows, perfil de testes não otimizado; não substitui o reteste
  do aplicativo em release. Os nove testes do lifecycle aprovados na seção 24
  não foram repetidos, pois seu código não mudou nesta correção.
- Verificações de formatação e whitespace do diff aprovadas.
- Clippy do código de produção, com avisos tratados como erros:
  **aprovado**, sem avisos no código de produção. Os três achados preexistentes
  do Clippy de testes continuam documentados na seção 24, sem alteração nesta rodada.

Durante a verificação, o Clippy falhou ao escrever `full.rmeta` por falta de
espaço (`os error 112`); C: reportou zero bytes livres. Foi removido somente o
executável GNU de testes gerado nesta rodada (1.120.699.449 bytes), após seus
18 testes terem terminado, liberando cerca de 1,1 GB para repetir a checagem.
Esse limite do ambiente não é classificado como defeito do watcher. A build
release e os dados da aplicação não foram removidos. A repetição do Clippy passou;
restaram aproximadamente 973 MB livres em C:, exigindo atenção ao espaço antes
da próxima compilação release.

### Escopo da revalidação

A correção foi revalidada em duas execuções com a pasta local monitorada e o
banco existente. A primeira cobriu alterações de arquivos descartáveis seguidas
de encerramento; a segunda, reabertura, reconciliação e saída ociosa.
A análise verificou a supressão do registro nativo repetido, a liberação do
callback, a conclusão dos loops e a ordem de drenagem em relação ao bridge e
ao SQLite. Os resultados e a conclusão de R1 estão registrados na seção 26.

## 26. R1: revalidação Windows v2 concluída (18 de novembro)

**Resultado técnico:** R1 concluído nos dois cenários dirigidos, após a proteção
contra registro duplicado da mesma raiz. Os logs mostram que o pedido repetido
foi recusado antes de chegar ao backend nativo em ambas as aberturas.

| Log                                        | Watcher: início até conclusão | Pedido até conclusão do backend | SQLite: conexões ociosas/total; fechamento | Resultado                                                |
| ------------------------------------------ | ----------------------------: | ------------------------------: | ------------------------------------------ | -------------------------------------------------------- |
| `mundam-watcher-r1-v2-20261118-041631.log` |                        2,8 ms |                         48,2 ms | 12/12; 45 ms                               | Watcher e aplicação `clean=true`; 14 tarefas concluídas. |
| `mundam-watcher-r1-v2-20261118-051741.log` |                        2,2 ms |                         27,7 ms | 17/17; 23 ms                               | Watcher e aplicação `clean=true`; 14 tarefas concluídas. |

Os tempos são diferenças entre timestamps dos logs, referentes ao backend;
não medem a duração total da apresentação nem o retorno do terminal. As duas
execuções usaram `tauri dev` com backend release, sem comprovação de bundle
instalado nem medição externa dos processos após a saída.

### Critérios confirmados

- `Watcher root already registered; native registration skipped` em cada boot,
  exercitando o caminho que antes produzia dois registros nativos.
- Pedido de parada, destruição do watcher, confirmação de liberação do callback
  e conclusão dos loops em ambas as saídas. Nenhum timeout/abort de dono nem
  falha de publicação nas duas amostras.
- `library_watcher` concluído antes da drenagem do bridge e do fechamento
  explícito do SQLite. Todas as conexões estavam ociosas antes de fechar o pool.
- Na primeira execução houve atividade da biblioteca antes do pedido de saída
  e um hint pendente descartado (`pending_discarded=1`); filas bruta/de domínio
  vazias e zero falhas de publicação. Na segunda, todos os contadores de
  descarte ficaram em zero. Não se afirma carga sustentada ou fila cheia nessa
  validação manual; esses caminhos têm testes automatizados na seção 25.
- A segunda abertura concluiu o boot scan com 22 arquivos inalterados e zero
  erros, após a primeira partir de 12 e registrar atividade até 22 fingerprints.
  É evidência de persistência/reconciliação nos arquivos exercitados, sem nova
  garantia universal de identidade de renomes ou recuperação transacional.

Os avisos Tao da primeira execução e `Chrome_WidgetWin_0 / Error = 1412` ao fim
das duas persistem como observações de runtime. Não impediram as confirmações
de liberação/drenagem nem geram nova frente de correção sem impacto comprovado.
Não há relato textual adicional do usuário nessa entrega; a conclusão desta
rodada fundamenta-se nos dois logs, sem atribuir novo aceite visual ou geral.

### Estado consolidado

Checklist, plano de execução e inventário foram atualizados. R1 deixa a lista
de trabalho pendente; a regressão do bundle Windows continua em R5, e o smoke
macOS das mudanças compartilhadas continua em R7. R6/Linux permanece concluído,
sem reabertura de campanha. Restam R2, R3, R4, R5, R7, R8 e R9 conforme a seção 14.

**Próxima implementação:** R2, supervisão dos loops auxiliares HTTP e dos
caminhos HLS por IPC. Esta análise alterou somente documentação; não houve
nova alteração de código, build ou execução de testes automatizados, pois os
logs validaram a correção já testada e registrada na seção 25.

## 27. R2: ownership dos auxiliares HTTP e HLS por IPC (18 de novembro)

**Estado:** implementação, verificação automatizada e validação dirigida Windows
concluídas nos escopos das seções 27–28. Alterações locais sobre a base `9207ad1`.
R1 e o aceite Linux permanecem preservados.

### Implementação

Os loops de expiração de processos HTTP e de sessões lineares deixaram de ser
tarefas destacadas. Ambos são futures aguardadas pelo dono `streaming_server`,
junto com o servidor e sua limpeza final. O cancelamento impede novas rodadas;
uma rodada admitida permanece aguardada até terminar, sob o prazo geral do
lifecycle. Saída antecipada do servidor também cancela seus auxiliares. Falha
de bind é registrada como falha do subsistema, sem iniciar os loops.

A limpeza periódica linear usa a mesma rotina de término/espera e remoção de
diretórios da saída final. Seus erros agora chegam ao lifecycle. Uma falha ao
confirmar a parada do escritor impede a remoção do respectivo diretório.
Os logs distinguem sessões expiradas da limpeza por encerramento.

O HLS por IPC passou a ter um único worker, admitido atomicamente no lifecycle
antes da publicação do manager. Pedidos usam fila limitada e resposta separada;
desistir da chamada IPC não transfere nem descarta o dono de uma preparação já
admitida. Cancelamento fecha a admissão, rejeita pedidos pendentes e aguarda
preparação em andamento, filhos e remoção dos arquivos temporários. Pedidos
concorrentes para o mesmo asset compartilham a sessão, sem sobrescrever filhos.

Expiração e saída final deixam de depender de descarte de handles ou limpeza
de diretórios destacada com atraso. Cada filho é consultado e, se necessário,
recebe pedido de término seguido de espera; seus arquivos só são removidos
após confirmação. Recursos cuja limpeza falha permanecem registrados para a
tentativa final, e a falha é preservada no resultado do worker/lifecycle.
O fallback de kill-on-drop continua como proteção, sem servir como evidência
de encerramento limpo.

Os temporários IPC ficam em subdiretórios internos únicos de `streams/ipc`,
separados dos temporários lineares HTTP. O identificador recebido pelo IPC
não compõe o caminho removido; não se remove arquivo original ou diretório
de uma sessão substituta. A auditoria não encontrou consumidor externo do
caminho antigo do manager: a entrada IPC retorna somente sucesso/erro.
Perfis de mídia e seleção de estratégia pelo provider não foram alterados.

### Auditoria dos caminhos alcançáveis

| Caminho                                                            | Dono e tratamento                                                                                                                                                                                               | Limite                                                                                                                                                                                            |
| ------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Manutenção HTTP de processos e sessões lineares                    | Aguardada dentro do dono do servidor, incluindo rodada admitida; erros de limpeza linear reportados ao lifecycle.                                                                                               | Prazo geral existente; não há prova de término se ele expirar.                                                                                                                                    |
| Preparação/FFmpeg do HLS por IPC                                   | Worker único retém preparação, filho e diretório, independentemente do solicitante; término/espera antes de remover arquivos.                                                                                   | Resolução de ferramentas e consulta síncrona de encoders são aguardadas fora do executor assíncrono, mas não são interrompíveis. Deadline durante essa etapa permanece resultado degradado de R4. |
| Segmentos e FFprobe HTTP                                           | Registro e espera pelos donos existentes preservados.                                                                                                                                                           | Corpos HTTP/conexões já entregues não ganham novo rastreamento nesta etapa.                                                                                                                       |
| Runner síncrono usado por extratores FFmpeg/FFprobe                | Corrigido retorno antecipado em erro de espera: tenta término, espera do filho e join dos dois leitores antes de retornar. Em saída normal, ambos os leitores são aguardados antes de propagar erro de leitura. | Falhas nativas de kill/wait ou leitura bloqueada não se tornam interrompíveis; política de falha e recuperação permanece em R4.                                                                   |
| Workers de thumbnails, cores e fingerprints; probes de ferramentas | Donos de workers/JoinSet e espera de operações nativas existentes conferidos. Probes de versão têm limite próprio e tentativa de kill/wait.                                                                     | Abortar a future superior não encerra automaticamente operações bloqueantes ou chamadas nativas; permanece em R4.                                                                                 |
| Preparação linear HTTP e extratores externos específicos           | Chamadas síncronas de encoder e ferramentas de extração permanecem vinculadas à operação que as aguarda.                                                                                                        | Não foi adicionada interrupção universal de ferramentas/FFI nem campanha de formatos; exceder o prazo ou falhar na recuperação continua delimitado em R4.                                         |

A barreira de recursos HLS não fecha a admissão de toda a função IPC: consultas
SQLite anteriores ao pedido e outros comandos continuam no escopo de R3.
Não se registra inventário universal concluído nem drenagem comprovada apenas
pela ausência de sessões num teste ocioso.

### Verificação automatizada

Foram executados testes de cancelamento antes da primeira rodada HTTP e espera
da rodada em andamento; sessão IPC com filho real de teste, concorrência para
o mesmo asset, desistência do solicitante, cancelamento durante preparação,
rejeição da fila/tardia, expiração, preservação do original e falha de limpeza.
Uma fixture sintética também exercita o worker de produção com FFmpeg real,
geração de playlist HLS e remoção dos temporários ao encerrar. O runner síncrono
tem testes com filho próprio para timeout e erro de espera injetado.

| Grupo               | Resultado    | Cobertura pertinente                                                                                                                                                                       |
| ------------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| HLS por IPC         | 8 aprovados  | Sessões com filho próprio, pedidos concorrentes, cancelamento com preparação retida e fila cheia, expiração, falha de cleanup, original preservado e pipeline de produção com FFmpeg real. |
| Streaming HTTP      | 21 aprovados | Agendamento/drenagem dos auxiliares, admissão linear, cancelamento e reaping de probes, fechamento HTTP com requisição admitida e contratos existentes de resposta/playlist.               |
| Runner síncrono     | 3 aprovados  | Fixture própria, timeout real e erro de espera injetado com término/espera e join dos leitores.                                                                                            |
| Manutenção de banco | 2 aprovados  | Recusa de rodada após cancelamento e espera da operação admitida.                                                                                                                          |

**Total dirigido: 34 aprovados**, nenhum erro ou teste ignorado. Execução Windows
com toolchain GNU, em perfil de testes não otimizado. Não foi executada a suíte
completa nem produzida uma build de distribuição. O teste com FFmpeg gerou uma
fixture de áudio de dois segundos, observou a playlist, aguardou a limpeza do
worker de produção e comparou os bytes do original após a saída.

Clippy incluindo produção e testes aprovado, com avisos tratados como erros.
Formatação Rust e whitespace do diff aprovados. Os três achados antigos de lint
da seção 24 foram corrigidos somente nos testes: propagação de falha de timeout
em manutenção de banco e expressão equivalente do loop de conexão HTTP. As
assertivas e resultados desses cenários foram preservados e revalidados.

A reconstrução das dependências GNU encontrou uma falha de permissão no
arquivador da biblioteca nativa ring. A repetição fora do sandbox concluiu a
build e os testes; a tentativa inicial não é contada como falha do produto.

### Critérios da validação dirigida no aplicativo

A validação realizada cobre fechamento com HLS HTTP e sessões lineares em uso,
com atividade de reprodução/seek e processamento em background. A seção 28
confirma os dois auxiliares antes de `streaming_server`, dono IPC ocioso limpo,
bridge após os produtores e fechamento explícito do pool. As sessões IPC ativas
continuam distinguidas como cobertura automatizada, incluindo FFmpeg real.
Não há nova
exigência de acessibilidade, suporte a formatos, otimização de rede ou repetição
da campanha Linux aceita.

## 28. R2: validação Windows com HLS, linear e processamento ativos (18 de novembro)

**Resultado:** R2 concluído nos caminhos auditados e cenários registrados.
O usuário relatou funcionamento conforme esperado durante a utilização. Os dois
logs confirmam conclusão dos auxiliares HTTP e do dono IPC antes do bridge,
drenagem das 14 tarefas registradas e fechamento explícito do SQLite, com
`clean=true`, sem timeout ou abort de tarefa.

| Log | Pedido até conclusão do backend | Pedido até parada do servidor HTTP | SQLite: ociosas/total; fechamento | Streaming no encerramento |
| --- | ---: | ---: | --- | --- |
| `mundam-shutdown-r2-20261118-180534.log` | 1.601,7 ms | 153,2 ms | 14/14; 57 ms | Um processo de segmento recebeu cancelamento Shutdown e teve sua terminação aguardada; nenhuma sessão linear. |
| `mundam-shutdown-r2-20261118-180908.log` | 1.443,4 ms | 236,9 ms | 19/19; 55 ms | Duas sessões lineares: um processo ativo e um já encerrado, sem falha de processo ou diretório. |

As execuções foram de desenvolvimento com backend release; não representam
teste do bundle instalado. Os tempos são diferenças entre timestamps do pedido
e das conclusões indicadas, sem medir a duração total da apresentação ou o
retorno do terminal. Não foi fornecida uma listagem externa de processos após
a saída; a confirmação dos filhos controlados decorre dos logs de espera.

### Evidências de drenagem

- Em ambas as execuções, os auxiliares `linear` e `processes` registraram
  conclusão antes de `Streaming server: maintenance owners finished` e da
  parada do servidor. `streaming_server` terminou antes do bridge e do pool.
- Na primeira, havia um handler de segmento ativo na fotografia intermediária.
  O processo foi aguardado e o servidor concluiu sua drenagem depois. Essa
  observação não acrescenta rastreamento independente de corpos/conexões HTTP.
- Na segunda, a limpeza das duas sessões lineares levou 233 ms: 218 ms de
  espera de processos e 12 ms de diretórios, com contadores de falha em zero.
- O dono IPC registrou `session_count=0`, `remaining_sessions=0` e `clean=true`
  nos dois logs. Isso confirma sua saída ociosa no aplicativo. A cobertura com
  filhos ativos, cancelamento de preparação e FFmpeg real continua atribuída
  aos testes automatizados da seção 27, sem confundir IPC com reprodução HTTP.
- Havia duas tarefas de thumbnail em voo e trabalho de fingerprints nas duas
  saídas. Os logs mostram espera, adiamento de processamento após cancelamento
  e conclusão desses workers antes do bridge. Não ocorreu fechamento do pool
  enquanto esses donos ainda estavam ativos. As duas aberturas também
  registraram boot scan com 803 assets inalterados e zero erros de scan.
- O watcher permaneceu com `clean=true`, preservando a evidência de R1.

### Observações sem bloqueio de R2

O primeiro log contém saídas de segmento com código 1 e `SEGMENT_ERROR` após
substituição de pedidos (`Superseded`) e após cancelamento de encerramento
(`Shutdown`), com eventos `fragLoadError` não fatais no player. A terminação e
a espera desses processos foram registradas. Esses diagnósticos não são
tratados como falha de reaping nem como prova de defeito de codec a partir
dos avisos OpenH264; a análise não afirma ausência de erros nas requisições.

Também há erros de geração de thumbnails, mensagens de fontes SVG e o aviso
`Chrome_WidgetWin_0 / Error = 1412` ao fim das duas execuções. São preservados
como observações, sem nova campanha de formatos, thumbnails ou acessibilidade.
Não impediram a drenagem verificada e não foram corrigidos nesta análise.

### Estado consolidado

Relatório, tabelas, inventário e checklist atualizados: R1 e R2 concluídos nos
escopos registrados. Permanecem R3, R4, R5, R7, R8 e R9 conforme a seção 14;
R6/Linux continua concluído e aceito. A validação do bundle Windows e o smoke
macOS final permanecem distintos desta rodada em desenvolvimento.

**Próxima etapa de implementação:** R3, admissão e drenagem dos comandos IPC
que criam recursos ou acessam SQLite durante a saída. Esta revisão alterou
somente documentação; não houve nova alteração de código, build ou testes
automatizados, pois os logs validaram a implementação já verificada na seção 27.

## 29. R3: admissão e drenagem dos comandos IPC (18 de novembro)

**Estado:** implementação realizada e verificada automaticamente; validação
Windows concluída na seção 30. R3 concluído nos caminhos e cenários registrados.
R1/R2 e o aceite Linux permanecem preservados. R4 não foi iniciado nesta etapa.

### Barreira de admissão e ordem de fechamento

Os 84 comandos de negócio agora recebem um guard extraído pelo Tauri antes
dos demais argumentos. A autorização não depende de um campo enviado pelo
frontend: uma chamada admitida mantém sua contagem até o retorno da função,
inclusive em erro de decodificação ou de negócio. Os retornos existentes
(incluindo booleanos, strings e listas) foram preservados.

O pedido de saída fecha a admissão sob trava antes do cancelamento dos
produtores. Chamadas posteriores retornam o erro estruturado `CANCELLED`, sem
iniciar consultas SQLite, extrações ou alterações de recursos. O cliente IPC
propaga esse cancelamento sem apresentá-lo como toast genérico de falha.
Os sete comandos de controle — quatro de startup, dois de apresentação de
shutdown e telemetria — permanecem fora dessa barreira, sem acesso a SQLite
nesses caminhos. A proteção existente contra conclusão do startup após pedido
de saída foi mantida.

O dono `ipc_commands` é registrado antes do boot e aguarda os guards admitidos.
Assim, o lifecycle mantém o bridge enquanto esses comandos ainda podem
publicar eventos. O fechamento explícito do pool exige também admissão fechada
e contagem zerada. O orçamento compartilhado de cinco segundos foi preservado;
se ele expirar, não se abortam comandos IPC para forçar commit/rollback nem se
fecha explicitamente o pool com chamadas ativas. O resultado continua
degradado, com os nomes/contagens pendentes registrados sem seus argumentos.

### Produtores derivados e acessos alternativos

O scan manual de duplicatas deixou de usar uma tarefa destacada: sua admissão
agora é atômica no lifecycle, com cancelamento herdado do encerramento. O
rebuild síncrono por IPC também usa esse token, retendo seu guard durante a
operação. A reconciliação de progresso e a manutenção/liberação de reservas
de matching são aguardadas pelo respectivo scan/worker. Cancelamento anterior
ao início não acessa o banco.

As exclusões adiadas pelo indexador permanecem num conjunto de tarefas retido.
Ao cancelar o listener, seus timers são interrompidos e suas tarefas aguardadas,
incluindo qualquer consulta/mutação já iniciada; os registros transitórios são
limpos antes da conclusão do dono. Falhas de supervisão degradam o lifecycle.
Essa mudança trata ownership, sem alterar a política de correlação de renames.

O plugin SQL, sem consumidor no frontend, deixou de ser registrado e suas
permissões foram removidas. As transferências de texto usadas na exportação
de tags e importação/exportação de temas passaram a comandos próprios com
admissão, leitura/escrita aguardadas e autorização pelo diálogo nativo.
O plugin filesystem permanece para receber essas autorizações; suas rotas
diretas de leitura/escrita foram retiradas das capabilities. As negativas
globais de acesso aos dados sensíveis do WebView foram preservadas, inclusive
quando existe uma autorização dinâmica para o mesmo caminho.

### Verificações e análise

Os seis testes de admissão passaram. Eles exerceram comandos
reais pelo IPC, argumentos inválidos, retornos distintos, rejeição após
fechamento, disponibilidade do controle de startup e transferências de texto
autorizadas. Um teste manteve uma transação SQLite aberta, iniciou o shutdown,
confirmou que bridge/pool ainda estavam disponíveis, liberou o commit e só
então observou a drenagem e fechou o pool. Uma nova chamada foi rejeitada com
`CANCELLED` mesmo após o banco já estar fechado.

A auditoria automatizada confere todos os comandos registrados contra os
guards de negócio e a lista explícita de exceções de controle. A rodada final
incluiu regressões de exclusão adiada e scans previamente cancelados. A
negativa global do filesystem foi exercitada com arquivo temporário existente,
autorização dinâmica concorrente e tentativas de leitura/escrita: ambas foram
recusadas e o conteúdo foi preservado. Um caminho inicialmente inexistente
produziu erro de leitura e não demonstrava essa propriedade; o teste foi
ajustado para distinguir recusa de acesso de falha de I/O.

| Verificação | Resultado | Evidência |
| --- | --- | --- |
| Features Rust | 73 aprovados | Duplicatas, matching, progresso, indexador, exclusões adiadas, settings, startup/shutdown e regressões HLS existentes, incluindo FFmpeg real. |
| Adaptadores IPC, lifecycle, pool e worker de duplicatas | 25 aprovados | Seis testes de admissão, transação retida, autorização IPC, scans dinâmicos, ordem do bridge, deadline compartilhado e processamento de duplicatas. O teste de negativas foi corrigido e reexecutado com sucesso. |
| Frontend | 24 aprovados | Cancelamento sem toast, erros reais preservados, clientes de startup/shutdown e controles de duplicatas. |
| Verificação estática e build | Aprovados | Clippy sem avisos em biblioteca/testes, formatação Rust, checagem de tipos, lint, build frontend e diff sem erros de whitespace. |

No frontend, 24 testes passaram: cliente de startup, cliente de shutdown,
cancelamento no wrapper IPC e controles de processamento/scan de duplicatas.
O cancelamento preservou sua rejeição sem toast e falhas reais de banco
continuaram apresentadas. Checagem de tipos, lint e build frontend passaram.
Os testes de componentes emitiram o aviso conhecido de computações Solid fora
de uma raiz; ele não impediu a conclusão.

### Validação dirigida e limites

A validação dirigida foi analisada na seção 30: o usuário relatou sucesso dos
dois cenários e os logs confirmaram admissão fechada, comandos drenados antes
do bridge, recusa de chamada tardia e pool fechado com resultado limpo. Os scans
manuais concluíram antes do pedido de saída; não se afirma cancelamento ativo
no aplicativo. As corridas de cancelamento e transação retida permanecem na
cobertura automatizada, sem depender de acertar esse intervalo manualmente.

Não foram incluídos testes de acessibilidade, campanhas de formatos/thumbnails
ou otimização de rede. Boot parcial, I/O nativo não interrompível e recuperação
transacional/leases continuam delimitados em R4. Bundle Windows atualizado,
smoke macOS final, decisão sobre medições e aceite geral continuam em R5/R7–R9.

## 30. R3: validação Windows e recusa de IPC tardio (18 de novembro)

**Resultado:** R3 concluído nos caminhos e cenários registrados. O usuário
informou que ambos os cenários ocorreram como esperado. Foram analisados
integralmente os logs `mundam-shutdown-r3-20261118-193056.log` e
`mundam-shutdown-r3-20261118-201749.log`, com aplicação em desenvolvimento e
backend compilado em release. Não se trata de validação do bundle instalado.

### Cenários e resultados

O primeiro cenário previa navegação/consulta de assets e importação/exportação
de um tema pelos diálogos nativos. Seu funcionamento foi confirmado pelo relato
do usuário; essas transferências não possuem eventos próprios nos logs, que
não são usados como prova independente de seu conteúdo ou persistência.
A execução também contém reprodução de mídia, pausa de um componente no
encerramento e limpeza de uma sessão linear, cujo FFmpeg já havia terminado
com código zero. A limpeza registrou zero falhas de processo e de diretório.

No segundo cenário, foram iniciados três scans manuais de duplicatas, todos
admitidos com nomes únicos no lifecycle e concluídos com sucesso. O último
terminou às `23:19:40.930003Z`, antes do pedido de saída às
`23:19:41.631172Z`: intervalo de aproximadamente 701 ms. Seus três handles
foram aguardados pelo lifecycle antes do bridge. Portanto, essa execução
valida admissão/registro e fechamento após os scans, sem demonstrar
cancelamento de um scan ainda ativo. Essa corrida permanece coberta pelos
testes automatizados de cancelamento, admissão dinâmica e drenagem da seção 29.

| Evidência | Execução 193056 | Execução 201749 |
| --- | --- | --- |
| Pedido de saída, UTC | 23:17:36.450697 | 23:19:41.631172 |
| Comandos IPC no fechamento da admissão | `active=0`, `commands={}` | `active=0`, `commands={}` |
| Drenagem IPC concluída, UTC | 23:17:36.454254 | 23:19:41.632901 |
| Bridge concluído, UTC | 23:17:36.481126 | 23:19:41.634157 |
| Tarefas concluídas normalmente | 15/15; `clean=true` | 18/18; `clean=true`, incluindo os três scans |
| Pool antes do fechamento | 20 conexões; 20 ociosas | 20 conexões; 20 ociosas |
| Fechamento explícito do pool | Concluído; 74 ms | Concluído; 58 ms |
| Pedido até conclusão do backend | 104,6 ms | 61,6 ms |
| Resultado do encerramento | `clean=true` | `clean=true` |

Esses tempos medem pedido até conclusão da limpeza do backend; não incluem
a duração mínima da apresentação nem medem o término de todos os processos
do ambiente de desenvolvimento. Em ambas as execuções a apresentação foi
confirmada dentro do prazo (`timely=true`).

### Recusa tardia e limites da evidência

Na primeira execução, o pool terminou de fechar às `23:17:36.555247Z`.
Às `23:17:36.823910Z`, a barreira recusou `get_processing_snapshot`, com o
registro `IPC command rejected after admission closed`. Isso demonstra no
aplicativo que uma consulta tardia não alcançou o banco já fechado. Não foi
registrado `PoolClosed`, timeout de drenagem, abort forçado, pânico ou
resultado degradado em nenhuma das duas saídas.

Os logs também confirmam drenagem das exclusões adiadas do indexador antes da
conclusão de seu dono, sem informar se havia alguma exclusão pendente naquele
instante. Como ambas as contagens IPC eram zero no pedido de saída, a espera
por uma transação admitida ainda ativa é demonstrada pelo teste determinístico
da seção 29, não por essas execuções manuais.

Erros de thumbnails/extratores Assimp, avisos de SVG/fontes, extração de cores
e decodificação de fingerprint aparecem durante o uso na primeira execução.
Há também avisos de foco do Tao e, ao final de ambos os logs, a mensagem
`Chrome_WidgetWin_0 / Error = 1412`. Não impediram a ordem de drenagem e o
fechamento limpo observados; permanecem fora do escopo desta etapa. O registro
`NativeCommandError` no começo da captura acompanha a saída stderr do launcher,
seguida de inicialização e execução normais; não representa falha do shutdown.

### Estado consolidado e próxima etapa

R1–R3 estão concluídos nos escopos registrados, com relatório, tabelas,
inventário e checklist reconciliados. R6/Linux continua concluído e aceito.
Não é necessário repetir a campanha de formatos, rede ou acessibilidade para
concluir R3. O encerramento geral da implementação permanece condicionado
aos itens explicitamente mantidos na seção 14.

**Próxima etapa de implementação:** R4, política e testes de boot parcial,
I/O nativo não interrompível e recuperação transacional/leases. Permanecem
depois R5, R7, R8 e R9. Esta análise alterou somente documentação; não houve
mudança de código, novo build ou reexecução de testes automatizados.

## 31. R4: boot parcial, I/O nativo e recuperação após saída incompleta (18 de novembro)

**Estado:** implementação realizada e verificada automaticamente; integração
Windows validada na seção 32. R4 concluído nos caminhos e cenários registrados,
com os limites nativos descritos abaixo. R1–R3 e o aceite Linux permanecem preservados.

### Espera limitada e propriedade do startup

O supervisor de startup era aguardado sem prazo próprio. Além disso, retirar
seu handle de um `Option` permitia que uma segunda espera interpretasse a
ausência do handle como conclusão enquanto a primeira ainda aguardava.
Agora o estado de conclusão é retido sob mutex assíncrono. A espera de shutdown
tem prazo de cinco segundos, incluindo eventual disputa pelo mutex; timeout
devolve resultado não limpo sem descartar ou abortar o supervisor. Uma espera
posterior pode observar sua conclusão real, mas o shutdown em andamento mantém
o resultado degradado já recebido e não fecha explicitamente o pool.

O fluxo de composição passou a usar um sequenciador de fases: diretórios,
banco/dependências básicas, mídia, workers/biblioteca e backend pronto. Cada
fase admitida termina antes da checagem de cancelamento seguinte. Uma falha
interrompe o fluxo e os dependentes não são iniciados. Os avanços e a apresentação
continuam protegidos pelo coordenador de shutdown. O probe síncrono de FFmpeg
no boot passou a ser aguardado em `spawn_blocking`, preservando a responsividade
do executor assíncrono durante essa chamada.

O pool só é obtido para finalização depois das esperas de startup/lifecycle e
protocolos. Se qualquer uma falhar, o fechamento explícito é omitido, inclusive
se o supervisor publicar o banco tardiamente. Boot encerrado antes da criação
do banco não exige fechar um pool inexistente. Admissões tardias no lifecycle
continuam recusadas atomicamente, e a admissão IPC de R3 permanece fechada.

### Política para operações nativas e reservas persistentes

O prazo de startup é separado do orçamento compartilhado de cinco segundos
do lifecycle e da graça já existente para o bridge. Cancelamento cooperativo
continua sendo a primeira ação. Abortar uma future após o prazo não comprova
parada de `spawn_blocking`, FFI, chamada de filesystem ou processo externo;
esse resultado permanece degradado. Não se solicita fechamento explícito do
pool nem se declara interrupção física apenas pelo descarte do handle assíncrono.

No caminho normal, thumbnails, cores e fingerprints continuam aguardando
`ProcessingLease::finish`, incluindo liberação SQL. No descarte anormal,
o guard agora solicita abort do heartbeat, retém o handle durante uma espera
de `finish` cancelável e deixa a reserva persistente recuperar por expiração.
Deixou de destacar uma liberação SQL que poderia ocorrer enquanto a operação
nativa original ainda estava ativa. Isso não desfaz uma consulta já enviada ao
SQLite; a saída incompleta mantém a política de não fechar explicitamente o pool.

A reserva de processamento dura 120 segundos desde a última aquisição ou
renovação. No reinício, uma reserva ainda válida não é tomada de outro dono;
após expirar, somente um novo executor obtém um novo UUID. Renovação, liberação,
falha ou publicação com ticket antigo não modificam o sucessor. Temporários
órfãos continuam sujeitos à reconciliação existente; não se remove um original
nem se publica resultado antigo para acelerar o encerramento.

### Recuperação transacional e falhas explícitas

O journal de operações de biblioteca permanece como barreira durável antes
do efeito nativo. Recibos SQLite distinguem commit de compensação no boot;
manifests ambíguos são retidos para reparo, e a falha interrompe a inicialização
antes dos consumidores dependentes. Os testes existentes de saída real de um
subprocesso nos marcos preparado, movido e commitado são parte desta verificação.

A auditoria das sagas legadas encontrou sucesso indevido para tipos cujo replay
não estava implementado. Esses registros agora recebem `FAILED` e diagnóstico;
o subsistema reporta a falha ao lifecycle, que preserva resultado degradado.
Operações independentes ainda são tentadas. Tipos legados como movimentação e
restauração da lixeira sem contexto suficiente não ganham replay especulativo;
exigem análise/reparo do registro em falha. Não foram redesenhadas as operações
de lixeira nem criada uma interface de reparo nesta etapa.

Exclusões suportadas usam filesystem assíncrono, validam flag e caminho absoluto,
aceitam arquivo já removido como replay idempotente e propagam erros reais.
Payload sem contexto ou operação desconhecida não é prova de conclusão do
efeito nativo. Falhas ficam persistidas, sem serem automaticamente reclassificadas
como sucesso numa execução posterior sem pendências.

### Verificações e análise

A rodada final aprovou 113 testes Rust, sem falhas. Cobriu fases de startup,
espera limitada com chamada nativa retida, mutex disputado, falha do supervisor,
ausência de banco, política de fechamento do pool, leases normais/anormais e
reabertura SQLite para thumbnails, cores e fingerprints. Também passaram as
regressões IPC de R3, workers, matching persistente, rollback de fingerprint,
replay e falhas de sagas, conflitos de recuperação e journals de biblioteca.

| Verificação | Resultado | Evidência |
| --- | --- | --- |
| Regressões Rust de boot, shutdown, IPC, workers e recuperação | 111 aprovados; 1 helper ignorado como teste isolado | O teste-pai executou esse helper em três subprocessos, encerrados nos marcos preparado, movido e commitado; recuperação repetida preservou o conteúdo original. |
| Recuperação persistente de matching | 2 aprovados | Reabertura preserva intenção manual, recupera lease expirado com UUID novo e rejeita publicação antiga; fingerprint/intenção de comparação sofrem commit e rollback juntos. |
| Clippy, formatação e diff | Aprovados | Biblioteca/testes sem avisos, Rustfmt consistente e diff sem erros de whitespace. |

O teste de restart verificou `PRAGMA integrity_check`, preservação de alterações
commitadas, rollback da transação abandonada, admissão exclusiva após expiração,
ausência de consumo de retry pelo dono antigo e bytes do original inalterados.
O teste de saga preservou diagnósticos de seis contextos inseguros ou falhos,
continuou a operação independente suportada e confirmou replay idempotente de
exclusão antes/depois do efeito nativo. O teste de native após abort comprovou
resultado degradado enquanto a chamada bloqueante ainda aguardava liberação.

As injeções de falha usam fixtures temporárias e o sequenciador empregado pelo
boot real; não foi adicionada uma chave de falha em produção nem simulado um
travamento do sistema operacional. A fixture nativa é liberada e aguardada ao
fim do teste; demonstrar que ela persiste após abort não deixa trabalho nativo
executando ao término da verificação.

### Validação dirigida e limites

A integração foi validada na seção 32: o usuário relatou sucesso nos três
cenários e os logs confirmaram saída limpa, drenagem sob processamento e
retomada de uma nova raiz na reabertura. No teste pela splash, o backend já
estava pronto; não se afirma interrupção manual de cada fase inicial.
Não houve deadline excedido nem descarte anormal de lease nesses logs. O
resultado degradado, a omissão do fechamento explícito do pool, a expiração e
as operações nativas retidas permanecem na cobertura automatizada. Essa
injeção não simula falhas reais do kernel/FFI nem exige travar ou terminar à
força a biblioteca real.

Essa rodada não reabre o aceite Linux nem inclui acessibilidade, suporte a
formatos/thumbnails ou desempenho de rede. Depois da conclusão de R4 nos
escopos registrados, permanecem bundle Windows atualizado (R5), smoke macOS final (R7), decisão sobre
medições (R8) e aceite geral (R9).

## 32. R4: validação Windows com processamento e reabertura (19 de novembro)

**Resultado:** R4 concluído nos caminhos e cenários registrados. O usuário
informou ter seguido os três cenários e observado funcionamento esperado, sem
problemas visíveis. Foram analisados os logs
`mundam-shutdown-r4-20261119-022247.log`,
`mundam-shutdown-r4-20261119-024119.log` e
`mundam-shutdown-r4-20261119-024250.log`. As execuções usaram desenvolvimento
com backend release, não o bundle instalado. Esta rodada não alterou código.

### Cenários observados e limites

**Saída pela splash:** conforme o relato do usuário, a saída ocorreu durante
a apresentação de startup. No primeiro log, o backend alcançou `BackendReady`
às `05:40:13.545853Z` e o pedido de saída ocorreu às `05:40:18.341160Z`, cerca
de 4,8 s depois. O scan inicial já havia concluído, preservando 803 assets e
sem erros. Essa execução confirma a integração da saída nesse cenário de
apresentação; não demonstra cancelamento das fases iniciais do backend nem
expiração dos cinco segundos de espera do supervisor. Essas condições
permanecem nos testes automatizados da seção 31.

**Processamento ativo:** no segundo log, uma nova raiz foi adicionada e seu
scan ainda estava em execução no pedido de saída. O cancelamento interrompeu
o scan e o dono `manual_scan` foi aguardado. Thumbnails ainda estavam nas fases
de publicação e reconciliação após o cancelamento; thumbnails, cores e
fingerprints terminaram antes do bridge e do pool. O bridge recebeu dois
eventos durante a drenagem (um de thumbnail e um de metadados), sem lag
registrado; isso não é prova de entrega desses eventos ao frontend.

**Reabertura e retomada:** o terceiro log registra duas raízes recuperadas no
startup, incluindo a adicionada na execução anterior. O scan da raiz anterior
preservou os mesmos 803 assets, e o da nova raiz foi retomado pelo boot. Seu
processamento ainda estava ativo no novo pedido de saída e foi cancelado e
aguardado, junto dos workers. O relato do usuário confirma o comportamento
esperado da biblioteca e das preferências; não há nesses logs uma auditoria
independente dos bytes de cada original ou dos valores de todas as preferências.

### Ordem de drenagem e tempos

| Evidência | Execução 022247 | Execução 024119 | Execução 024250 |
| --- | --- | --- | --- |
| Pedido de saída, UTC | 05:40:18.341160 | 05:42:38.807647 | 05:47:44.732888 |
| IPC no fechamento da admissão | `active=0` | `active=0` | `active=1`, `get_processing_snapshot` |
| Tarefas concluídas normalmente | 15/15 | 16/16, incluindo scan dinâmico | 15/15, incluindo boot scan retomado |
| Resultado do lifecycle | `clean=true` | `clean=true` | `clean=true` |
| Pool antes do fechamento | 6 conexões; 6 ociosas | 20 conexões; 20 ociosas | 20 conexões; 20 ociosas |
| Espera por ociosidade / fechamento do pool | 67 ms / 210 ms | 0 ms / 26 ms | 25 ms / 88 ms |
| Pedido até conclusão da limpeza do backend | 294,9 ms | 697,1 ms | 1.014,2 ms |
| Resultado final | `clean=true` | `clean=true` | `clean=true` |

No terceiro encerramento, a consulta IPC já admitida terminou às
`05:47:45.633120Z`, aproximadamente 900,2 ms após o pedido de saída. Seu dono
foi confirmado antes do bridge (`05:47:45.633237Z`) e do fechamento do pool,
iniciado às `05:47:45.658574Z` e concluído às `05:47:45.747008Z`. A execução
acrescenta evidência real da espera de um comando ativo, além da transação
retida nos testes automatizados de R3.

Os tempos acima medem a limpeza do backend, não o fim de todos os processos
do ambiente de desenvolvimento. As duas últimas execuções registram
confirmação de apresentação com `timely=true`. O primeiro log não contém essa
confirmação; a saída sem dependência dela já possui espera limitada e cobertura
automatizada, e o usuário não relatou problema de apresentação ou fechamento.

Não houve timeout de startup/drenagem, abort forçado, pânico, `PoolClosed`,
descarte anormal de lease ou falha de saga reportada ao lifecycle. Nas três
aberturas, a recuperação de sagas informou ausência de operações pendentes;
essas execuções não exercitaram replay de saga interrompida nem expiração de
lease. As evidências correspondentes continuam na seção 31.

### Erros e observações preservadas

O terceiro log contém dois erros SQLite `database is locked` no worker de
thumbnails: às `05:46:36.951937Z`, durante o uso, e às `05:47:45.180089Z`,
durante a drenagem. Também há uma renovação de lease que demorou 17,93 s
antes do fechamento, com uma linha afetada. São erros/latência reais registrados,
sem causa específica demonstrada por esses logs. Não foram tratados como
problemas de formato nem corrigidos nesta análise.

O caminho revisado do worker aguarda `lease.finish` antes de devolver o erro
do processamento. O log confirma seu dono concluído antes do pool, sem
diagnóstico de falha de liberação do lease ou resultado degradado. Isso
sustenta a conclusão sobre a ordem de finalização, mas não comprova sucesso
das duas operações que reportaram erro. `clean=true` não significa ausência
de erros de negócio/transação nem conclusão de toda a fila. A observação fica
registrada para investigação de contenção, sem evidência de falha na drenagem.

Os avisos de foco do Tao, a mensagem `Chrome_WidgetWin_0 / Error = 1412` no fim
dos três logs e a captura inicial `NativeCommandError` do stderr do launcher
também foram preservados. Não impediram a inicialização e os encerramentos
verificados, e não motivaram nova campanha de formatos, rede ou acessibilidade.

### Estado consolidado e próxima etapa

R1–R4 concluídos nos escopos registrados; relatório, tabelas, inventário e
checklist atualizados. O aceite Linux permanece preservado. Interrupção
universal de kernel/FFI, recuperação automática de contexto ambíguo e prova
de término de todo processo externo continuam como limites explícitos da
política, não garantias acrescentadas a partir de uma saída limpa.

**Próxima etapa:** R5, gerar e validar o bundle Windows atualizado com o código
final. Permanecem também smoke macOS (R7), decisão sobre medições (R8) e aceite
geral (R9). Não houve novo build nem reexecução de testes automatizados nesta
análise, pois as alterações foram somente documentais.

## 33. R5: preparação do bundle Windows final (19 de novembro)

**Estado desta preparação:** build de produção MSI/NSIS concluída, payload MSI e fontes NSIS
conferidos. A rodada instalada posterior está analisada na seção 34; os hashes
abaixo identificam o pacote inicial, anterior à correção CORS. Esta etapa verifica
o artefato Windows depois das mudanças R1–R4;
as execuções anteriores em desenvolvimento com backend release não substituem
essa validação. O aceite Linux e o smoke macOS restante permanecem separados.

### Fonte e empacotamento

A build usa o checkout `ecc98340c7417597246bf0058f91a6f9a5c4d574`, com as mudanças
R1–R4 já registradas. Nesta preparação não foi alterado o comportamento do
backend/frontend, a versão `0.1.0` ou o identificador
`com.marcusmaia.mundam`. Os destinos de bundle configurados são MSI e NSIS,
com Rust `1.94.0`, target Windows x64 MSVC e dependências Rust travadas/offline.
O frontend de produção passou pela compilação Vite, incluindo as entradas
principal, splash e shutdown. A build empacotada usa esses assets, sem depender
do servidor de desenvolvimento.

A compilação Vite terminou em 34,12 s; a compilação release Rust informou
18 min 50 s. O CLI concluiu os dois bundles. O PE de `mundam.exe` foi identificado
como x64 e subsistema Windows GUI. Os três artefatos estão sem assinatura
Authenticode nesta build local.

| Artefato, relativo ao repositório | Tamanho em bytes | SHA-256 |
| --- | ---: | --- |
| `src-tauri/target/release/mundam.exe` | 64.418.816 | `F2E41F79577A8B9E0A29277042D8429B79EE827F349799434290150EA092640F` |
| `src-tauri/target/release/bundle/nsis/Mundam_0.1.0_x64-setup.exe` | 82.486.415 | `1B736FB3C48D07466336DE9D078D910C09321CFD38045516ADD09CC0C756EE4F` |
| `src-tauri/target/release/bundle/msi/Mundam_0.1.0_x64_en-US.msi` | 110.637.056 | `4F0C59558F3FFA32B61F50EFB55CB1CB720846AF5FC374AFD7AF676697856149` |

O manifesto inicial foi preservado em
`src-tauri/target/r5-diagnostics/r5-initial-artifacts.json`;
`src-tauri/target/release/bundle/r5-artifacts.json` identifica agora o pacote
corrigido da seção 35, com hashes, versões e inventário. O NSIS está
configurado como `currentUser`, com destino padrão na pasta Mundam do
LocalAppData; o MSI usa `perMachine` e Program Files. O roteiro de validação
foi preparado para o NSIS. Nenhum dos dois foi instalado nesta preparação.

A configuração Windows inclui FFmpeg/FFprobe e suas DLLs compartilhadas,
Assimp com sua DLL, PDFium e o SoundFont. A inspeção do Assimp preparado
confirmou import de `assimp-vc143-mt.dll`, presente junto do executável;
não foi encontrado o import incompatível `assimp-vc145-mt.dll` diagnosticado
na rodada histórica. Isso não substitui execução dos extratores no aplicativo
instalado nem prova os pré-requisitos em outra máquina.

O MSI foi extraído estaticamente com WiX Dark, sem executar sua instalação:
os 17 arquivos extraídos coincidiram por SHA-256 com as fontes indicadas na
definição WiX, incluindo executável, DLL Rust, ferramentas e dados. A definição
NSIS contém 15 recursos, mais o executável principal; todas as fontes existem
e seus hashes foram registrados. O NSIS inclui os mesmos componentes nativos
necessários, sem a DLL Rust adicional presente no MSI; a inspeção de imports
do executável não mostrou dependência dessa DLL Rust.

O CLI Tauri `2.9.6`, com crate Tauri `2.10.2`, emitiu aviso de que não encontrou
`__TAURI_BUNDLE_TYPE` para marcar o executável durante os dois empacotamentos.
O aviso menciona possível impacto no updater; a aplicação não registra plugin
de updater e esse comportamento não foi validado nem corrigido nesta etapa.
A geração dos instaladores terminou com sucesso. A decompilação MSI também
emitiu `DARK1059` sobre referências de controles de UI; a extração e os hashes
foram concluídos, sem tratar essa análise estática como validação da UI do MSI.

### Captura do executável instalado

Foi adicionado `scripts/test-bundle-windows.ps1`, que inicia o executável
informado, redireciona stdout/stderr mesmo sem console e aguarda seu término.
O registro inclui cenário, caminho e SHA-256 do executável, presença/hashes de
recursos instalados, código de saída e snapshots de processos antes/depois.
O filtro de logs do PowerShell é restaurado ao final. Processos preexistentes
são separados dos candidatos novos; os snapshots não verificam ancestralidade
nem, isoladamente, comprovam processos órfãos do Mundam.

O capturador não instala o aplicativo, não remove dados da biblioteca e não
encerra processos. Sua verificação usou um executável GUI temporário que
escreve marcadores em stdout/stderr e termina com código 7. Os dois fluxos,
o código de saída, o inventário e a restauração do ambiente foram conferidos.
O caminho final passou em PowerShell 7 e Windows PowerShell 5.1. A verificação
no 5.1 exigiu reter o handle do processo antes da espera para preservar a
leitura de `ExitCode`; o ajuste foi aplicado e a repetição confirmou o código 7.
Isso valida a captura e não representa teste visual ou abertura do Mundam
instalado. A fixture e seus logs ficaram no diretório de build ignorado pelo Git.

### Validação dirigida prevista

Os cenários do aplicativo instalado cobrem saída pela splash, uso ocioso,
janela auxiliar, processamento/mídia ativos, reabertura, principal minimizada
e pedidos repetidos. A comparação de hashes identificará o executável do
pacote efetivamente usado. O relatório registrará o instalador escolhido e
os resultados observados, distinguindo geração de MSI/NSIS de instalação real
de cada formato.

O encerramento deve manter admissão fechada, espera dos donos, bridge após
produtores e pool fechado apenas após drenagem limpa, ou registrar o resultado
degradado previsto. A etapa não inclui nova campanha de formatos/thumbnails,
otimização de rede, acessibilidade ou instalação limpa em outra máquina.

## 34. R5: rodada instalada e correção do transporte de mídia (19 de novembro)

**Estado da rodada inicial, anterior ao reteste da seção 36:** os quatro encerramentos foram aprovados nos
cenários relatados. R5 permanece pendente de reprodução/seek e saída sob carga
no pacote corrigido, pois a rodada revelou uma regressão de transporte no
aplicativo instalado. Não foi reaberta uma campanha de suporte a formatos,
thumbnails ou desempenho de rede.

### Amostras e identificação

Foram analisados os registros consolidados e os fluxos originais stdout/stderr:

| Cenário | Registro consolidado |
| --- | --- |
| Boot/splash | `mundam-bundle-r5-boot-20261119-053247-0bfd41.log` |
| Janela auxiliar | `mundam-bundle-r5-auxiliary-20261119-053410-82082e.log` |
| Carga e mídia | `mundam-bundle-r5-load-20261119-053513-615266.log` |
| Minimizada | `mundam-bundle-r5-minimized-20261119-053821-5971b1.log` |

As quatro execuções identificam o executável instalado no LocalAppData, com
SHA-256 `F2E41F79577A8B9E0A29277042D8429B79EE827F349799434290150EA092640F`,
igual ao artefato inicial da seção 33. Os seis recursos amostrados estão
presentes e seus hashes coincidem com o manifesto da build: FFmpeg, FFprobe,
Assimp e sua DLL, PDFium e SoundFont. Essa comparação não representa nova
verificação de todos os arquivos instalados nem validação da instalação MSI.

### Encerramento e reabertura

O usuário relatou encerramentos rápidos e corretos nos quatro cenários,
com ressalva de reprodução/carregamento no cenário de carga. Todos os logs
terminam com código de saída zero, lifecycle e encerramento `clean=true`.
O tempo abaixo é a diferença entre o pedido de shutdown e sua conclusão no
backend; não mede instalação, tempo total de abertura nem percepção visual.

| Cenário | Tarefas registradas na drenagem | Pedido até conclusão do backend |
| --- | ---: | ---: |
| Boot/splash | 2 | 2.417 ms |
| Janela auxiliar | 15 | 1.711 ms |
| Carga e mídia | 16 | 1.272 ms |
| Minimizada | 15 | 4.375 ms |

No boot, o pedido ocorreu durante Database. `library_cleanup` e `saga_recovery`
tiveram admissão recusada, e a inicialização parou antes de Media, sem atingir
BackendReady. IPC e bridge foram aguardados e o pool foi fechado explicitamente.
Esta amostra acrescenta cancelamento real de boot parcial à cobertura
automatizada de R4; na rodada anterior o backend já estava pronto.

Nas três execuções seguintes o backend atingiu BackendReady. A sequência de
aberturas é evidência de reabertura funcional. As recuperações registraram
ausência de sagas pendentes, mas não foi feita auditoria de todos os dados ou
preferências persistidos. O comportamento de janelas auxiliares, minimização
e pedidos de saída segue o relato do usuário; os logs não distinguem cada
gesto nem provam visualmente a apresentação da janela de encerramento.

Sob carga, a sessão linear ativa foi encerrada e aguardada, com zero falhas de
processo e de diretório. `running_processes=1` é a contagem dos processos vivos
encontrados para limpeza, não de processos remanescentes após ela. O processamento
em andamento foi aguardado antes do bridge e do pool. No cenário minimizado,
o boot scan terminou após o cancelamento e explica a maior parte da espera;
as consultas tardias de processamento foram recusadas pela barreira IPC.
Não houve resultado degradado nem fechamento do pool antes da drenagem.

Os quatro snapshots finais não listam novos candidatos entre os nomes de
processos monitorados. São amostras pontuais, sem prova de ancestralidade ou
de término universal de processos externos. Os stderr contêm apenas o aviso
conhecido `Chrome_WidgetWin_0 / Error = 1412`; os avisos Tao de foco também
permanecem registrados, sem falha de encerramento demonstrada nesta rodada.

### Falha de reprodução e diagnóstico

O usuário relatou que vídeos antes funcionais nos modos escolhidos pelos
providers não reproduziram no cenário de carga, e que o loader pareceu estático
nesse contexto. Em testes adicionais, esclareceu que o loader funciona ao
executar o programa e associou a aparência estática ao player. Não foi alterado
CSS, animação ou preferência de acessibilidade, nem foi criado requisito
independente de validação do loader.

O log de carga registra HLS com `manifestLoadError`, fatal, e status HTTP zero;
o modo linear chegou a analisar o manifesto, mas falhou em segmentos com
`fragLoadError` e status zero. Há também eventos do transporte nativo HTTP
com metadados, `playing` e nove quadros em aproximadamente 0,1 s. Esse avanço
breve não comprova reprodução sustentada nem invalida a falha observada pelo
usuário. Status zero, isoladamente, não identifica a causa.

Em diagnóstico controlado do mesmo executável instalado, o WebView2 confirmou
a origem `http://tauri.localhost`. Uma consulta ao servidor local falhou com
`MissingAllowOriginHeader`: a lista CORS autorizava `tauri://localhost`,
`https://tauri.localhost` e a origem de desenvolvimento, mas omitia a origem
HTTP efetivamente usada pelo pacote Windows. Isso explica a diferença de
transporte entre desenvolvimento e instalação, sem atribuí-la aos codecs.

Na mesma sessão, a inclusão temporária apenas do cabeçalho de autorização
para essa origem permitiu resposta HTTP 200. Uma amostra MKV via HLS completou
13,35 s com 400 quadros decodificados; após seek para 5 s, o contador avançou
até 680 quadros e o vídeo voltou ao fim. Houve avisos não fatais de buffer,
sem falha fatal. Uma amostra MPG pelo endpoint linear também completou
13,35 s com 400 quadros, sem erros do player de diagnóstico. Uma amostra MP4
nativa decodificou 42 quadros em 1,19 s sem intervenção nos seus cabeçalhos.

Essas verificações usaram elementos temporários e o Hls.js já empacotado;
isolam transporte/decodificação e não substituem o fluxo completo da interface
nem o reteste do novo instalador. Os elementos e a interceptação temporária
foram removidos e o aplicativo foi fechado pelo fluxo normal, com `clean=true`.
Não foram alterados os arquivos originais de mídia para realizar o diagnóstico.

### Implementação e validação automática

`delivery/streaming/server.rs` passou a autorizar explicitamente
`http://tauri.localhost`, preservando as demais origens e sem liberar origem
arbitrária. Providers, seleção nativo/linear/HLS, codecs, loader e política de
shutdown permanecem com o comportamento anterior.

O teste de regressão usa servidor HTTP real em porta efêmera. Verifica GET
e preflight OPTIONS para as quatro origens do aplicativo, cabeçalho Range
e ausência de autorização CORS para uma origem externa. Os sete testes
direcionados do servidor de streaming passaram; Clippy de biblioteca/testes
com avisos tratados como erro e a verificação de formatação Rust passaram.

**Próxima validação:** pacote corrigido, reprodução e seek de amostras antes
funcionais nos providers existentes, seguidos de fechamento durante uso e
processamento. Os quatro encerramentos iniciais permanecem como evidência;
R5 não foi marcado concluído apenas pelo diagnóstico controlado. R6/Linux
continua aceito, e permanecem R7/macOS, decisão de medições R8 e aceite geral R9.

## 35. R5: instaladores com correção CORS (19 de novembro)

**Estado na preparação, anterior ao reteste da seção 36:** executável e instaladores MSI/NSIS corrigidos gerados; validação
instalada de reprodução/seek e fechamento sob carga pendente do usuário.
Os artefatos substituem os arquivos dos mesmos caminhos da seção 33.

A fonte continua baseada em `ecc98340c7417597246bf0058f91a6f9a5c4d574`, com a
correção local da origem CORS e seu teste registrados na seção 34. A versão
`0.1.0`, o identificador e os providers foram preservados. A build de produção
Windows x64 MSVC, com dependências Rust travadas/offline, terminou com código
zero. O frontend Vite também foi compilado, mantendo as entradas principal,
splash e shutdown. Os artefatos locais continuam sem assinatura Authenticode.

| Artefato, relativo ao repositório | Tamanho em bytes | SHA-256 |
| --- | ---: | --- |
| `src-tauri/target/release/mundam.exe` | 64.418.816 | `8B2C225C60953529B5DFD87924FA5FFE483935F884E754F90054E4BBF1BE47E7` |
| `src-tauri/target/release/bundle/nsis/Mundam_0.1.0_x64-setup.exe` | 82.476.266 | `42A7654397BA29E873A7D742A2AF7733E22F62F4BCBDEC2DB3EEC5132A43DADF` |
| `src-tauri/target/release/bundle/msi/Mundam_0.1.0_x64_en-US.msi` | 110.637.056 | `702660C0E6FAB9D6693F9064195F3A72B66E0308FE0121924BCB4EDF54255BD1` |

O MSI corrigido foi novamente extraído sem instalação: os 17 arquivos
coincidiram por SHA-256 com suas fontes, incluindo o executável corrigido.
As 15 fontes de recursos NSIS foram conferidas, e os seis recursos amostrados
pelo capturador mantiveram os hashes da rodada inicial. A decompilação repetiu
os avisos `DARK1059` sobre referências da UI; essa inspeção não valida a
instalação ou a interface do MSI. O manifesto vigente registra também que
a reprodução no pacote corrigido ainda aguarda reteste.

O aceite de R5 depende de confirmar o fluxo normal da interface instalada
com arquivos antes funcionais, seek e saída durante mídia/processamento.
Não se exige repetir toda a campanha inicial por essa alteração restrita
ao transporte. Permanecem as evidências e os limites das seções 33–34.

## 36. R5: reteste instalado aprovado e conclusão Windows (19 de novembro)

**Estado:** R5 concluído nos cenários registrados. O usuário confirmou que
tudo funcionou corretamente após o reteste solicitado de reprodução/seek e
fechamento sob carga no pacote corrigido. Essa confirmação, somada aos quatro
encerramentos iniciais, encerra a pendência Windows de R5; não representa
compatibilidade universal de mídia nem validação visual da instalação MSI.

### Identificação e reprodução

Foram analisados `mundam-bundle-r5-load-20261119-100332-be276f.log` e seus
fluxos originais `-stdout.log` e `-stderr.log`. O executável instalado no
LocalAppData tem SHA-256
`8B2C225C60953529B5DFD87924FA5FFE483935F884E754F90054E4BBF1BE47E7`,
igual ao pacote corrigido da seção 35. Os seis recursos amostrados estão
presentes e seus hashes coincidem com o manifesto.

Há manifestos HLS analisados e reprodução com avanço de tempo e quadros nos
transportes blob e HTTP. Os eventos de vídeo registram até 452 quadros e
84,45 s em blob, e até 223 quadros e 78 s em HTTP, em diferentes aberturas.
Esses máximos não são somados como duração ou quadros de um único arquivo.
O relato de reprodução/seek dos arquivos testados é do usuário; a telemetria
complementa esse relato sem identificar independentemente cada gesto/provider.
Não se repetiram `manifestLoadError` nem `fragLoadError` com status zero.

O registro não está livre de erros de mídia: há quatro `bufferStalledError`,
três `bufferSeekOverHole` e onze `fragLoadError` com HTTP 500, todos não fatais
nos eventos Hls.js. O backend registra saídas FFmpeg com código 1, inclusive
diagnóstico `Invalid stream index 2` e avisos OpenH264; há também terminação
aguardada por `Superseded`. Não se atribuem todas essas falhas a uma única
causa nem se declara que cada segmento foi recuperado. O usuário não relatou
falha funcional no reteste. As ocorrências permanecem como observações de
mídia fora do escopo desta conclusão, sem impacto demonstrado no shutdown.
O loader permaneceu sem alterações.

### Saída sob mídia e processamento

O pedido foi registrado às 13:08:14.975792 UTC e a conclusão do backend às
13:08:16.444464 UTC: aproximadamente **1.469 ms**. O processo principal terminou
com código **0**, e lifecycle/encerramento registraram **`clean=true`**.

- A admissão IPC fechou com uma consulta `get_processing_snapshot` ativa;
  sua drenagem terminou antes do bridge e do pool.
- O frontend registrou pausa de dois elementos de mídia e confirmação da
  apresentação de shutdown, com ocultação das janelas anteriores.
- A sessão linear com processo ativo foi encerrada e aguardada: 170 ms de
  espera de processo, 2 ms de diretório e zero falhas nessas duas limpezas.
  Donos HTTP e manutenção terminaram antes da conclusão do servidor.
- O watcher liberou observação nativa e loops com `clean=true`. Havia thumbnail,
  extração de cores e fingerprint em andamento; seus donos foram aguardados,
  com thumbnail adiada após cancelamento.
- As 15 tarefas registradas concluíram antes do fechamento explícito do pool
  SQLite, cujo close levou 654 ms. Não houve fechamento antecipado do banco
  nem resultado degradado registrado.

Os snapshots após a saída estão vazios, sem novos candidatos entre os nomes
monitorados; permanece o limite de amostragem e ausência de ancestralidade
verificada. O stderr contém o aviso conhecido `Chrome_WidgetWin_0 / Error = 1412`,
sem falha de encerramento associada nesta execução.

### Estado consolidado

Resumo executivo, matriz desktop, inventário, checklist e tabela da seção 14
atualizados. R1–R5 estão concluídos nos escopos registrados; R6/Linux permanece
concluído e aceito. A próxima etapa é **R7, smoke macOS das mudanças
compartilhadas finais**. Permanecem a decisão de medições R8 e as verificações
finais de integridade/recuperação e aceite geral R9. Acessibilidade, suporte
adicional a formatos e desempenho de rede continuam fora dos critérios desta
conclusão. Esta análise alterou somente documentação; não houve nova build
nem reexecução de testes automatizados.
