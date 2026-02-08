Existe um gargalo clássico de aplicações de gerenciamento de mídia (DAM) desktop: tratar arquivos brutos (*raw, mkv, mov 4k, etc.*) como se fossem arquivos web-ready.

É perfeitamente possível fazer transcoding sob demanda (streaming) sem gerar arquivos gigantescos em disco e sem travar a interface. Isso transforma o Mundam de um "conversor de arquivos" para um "servidor de mídia local".

Para resolver isso no ecossistema **Tauri + Rust**, precisamos mudar a arquitetura de *File-Based* (gera arquivo -> lê arquivo) para *Stream-Based* (lê input -> processa -> envia chunks para o WebView).

Aqui estão as três melhores abordagens para realizar isso, da mais simples à mais robusta, focando em economizar disco e memória.

---

### Abordagem 1: Streaming Linear Direto (fMP4 via Pipe)

Nesta abordagem, você cria um servidor HTTP local em Rust (usando `axum`, `actix-web` ou até `tiny_http`) que roda dentro do processo do Tauri. Quando o `<video>` do HTML pede o arquivo, o Rust chama o FFMPEG.

**Como funciona:**

1. O Frontend pede `http://localhost:port/stream?file=video.mkv`.
2. O Rust inicia um `Child Process` do FFMPEG.
3. O FFMPEG lê o arquivo original e converte para **Fragmented MP4 (fMP4)** ou **WebM**, escrevendo o resultado diretamente no `STDOUT` (Standard Output).
4. O Rust pega esse `STDOUT` e "pipeia" (canaliza) diretamente para a resposta HTTP em tempo real.

**Vantagens:**

* **Zero Disco:** Nenhum byte é escrito no HD. Tudo acontece na RAM (buffer).
* **Início Imediato:** O vídeo começa a tocar assim que o FFMPEG processa o primeiro frame.

**Desvantagens:**

* **Seeking (Avançar/Voltar) é difícil:** O formato MP4 padrão (com *moov atom* no início) não funciona aqui. Você precisa usar MP4 fragmentado. Se o usuário clicar no minuto 50:00, o FFMPEG teria que processar do 0 até o 50 para saber o estado, o que causa delay.
* **Sem Cache:** Se o usuário voltar o vídeo, o processo pode ter que recomeçar.

---

### Abordagem 2: HLS "On-the-Fly" (A Recomendada) 🌟

Esta é a técnica usada por gigantes (Netflix, YouTube), mas adaptada para rodar localmente no Mundam. O HLS (HTTP Live Streaming) divide o vídeo em pequenos pedaços (`.ts` de alguns segundos) listados em um arquivo de manifesto (`.m3u8`).

A grande sacada aqui é **não gerar todos os segmentos de uma vez**.

**Como funciona a arquitetura:**

1. **O Manifesto Virtual:**
O Frontend pede o arquivo `.m3u8`. O Rust analisa o vídeo original (usando `ffprobe`) para saber a duração total (ex: 10 min). O Rust gera um texto `.m3u8` dinamicamente na memória dizendo: "Existem 60 pedaços de 10 segundos".
2. **Transcoding Sob Demanda (Chunking):**
O player de vídeo (como Video.js ou Plyr no frontend) decide baixar o "pedaço 30" (minuto 5:00).
* O Player pede: `http://localhost/segment_30.ts`.
* O Rust recebe a requisição e calcula: "O pedaço 30 começa em 300s e dura 10s".
* O Rust invoca o FFMPEG com o comando de **Seek rápido**: `ffmpeg -ss 300 -t 10 -i video_original.mkv ...`.
* O FFMPEG processa *apenas* esses 10 segundos e devolve o binário.
* O Rust entrega ao player.



**Vantagens:**

* **Seeking Instantâneo:** O usuário pode pular para o final de um vídeo de 4 horas e o FFMPEG só processará aqueles 10 segundos específicos.
* **Baixo Uso de CPU:** Só processa o que o usuário assiste. Se ele assistir 1 minuto de um vídeo de 1 hora, você não processou os outros 59 minutos.
* **Baixo Disco:** Você não salva os arquivos `.ts`. Eles são gerados, entregues e descartados da memória.

**Desvantagens:**

* **Complexidade de Implementação:** Exige gerenciar o servidor local e calcular tempos de corte com precisão.
* **Audio Gaps:** Se o corte não for feito nos *keyframes* exatos, pode haver um leve "estalo" no áudio na transição dos blocos (resolvível com flags corretas do FFMPEG).

---

### Abordagem 3: Custom Protocol do Tauri (Scoped)

O Tauri permite registrar protocolos customizados (ex: `mundam://video/id`). Ao invés de levantar um servidor HTTP completo (localhost), você usa a API interna do Tauri.

**Como funciona:**
Você intercepta a requisição do arquivo. O *WebView* do sistema operacional enviará requisições com cabeçalhos de **Range** (ex: `bytes=0-1024`).
Você precisaria criar um "Reader" em Rust que, ao receber um pedido de bytes, traduz isso para uma posição de tempo no vídeo, invoca o FFMPEG e devolve os bytes transcodificados.

**Vantagens:**

* Mais seguro (não abre portas TCP locais).
* Integrado nativamente ao Tauri.

**Desvantagens:**

* **Extremamente difícil:** Mapear *bytes de um arquivo MP4 transcodificado (que tem bitrate variável)* para *timestamp do vídeo original* é uma tarefa hercúlea e propensa a erros. O navegador espera que o tamanho do arquivo seja consistente, mas no transcoding em tempo real, não sabemos o tamanho final exato do arquivo. **Não recomendo essa via para vídeos longos.**

---

### Resumo Comparativo

| Característica | Transcoding Completo (Atual) | Streaming Linear (Pipe) | HLS On-the-Fly (Recomendado) |
| --- | --- | --- | --- |
| **Uso de Disco** | Alto (Arquivo Duplicado) | Zero (RAM) | Zero (RAM) |
| **Tempo de Início** | Lento (Espera terminar) | Imediato | Imediato |
| **Seeking (Pular)** | Rápido (após gerar tudo) | Lento/Travado | **Muito Rápido** |
| **Uso de CPU** | Alto (Processa tudo) | Alto (Se assistir tudo) | **Otimizado** (Só o trecho visto) |
| **Complexidade** | Baixa | Média | Alta |

### Detalhe Técnico: Como fazer o HLS "On-the-Fly" no Rust

Para implementar a **Abordagem 2 (HLS)**, que resolve seus problemas de espaço e performance, a lógica do backend (Rust) seria mais ou menos assim:

1. Use uma crate como `actix-web` ou `axum` rodando em uma thread separada (o *Sidecar* do Tauri não serve aqui, tem que ser código Rust na main thread ou threadpool).
2. Rota `/playlist/:id.m3u8`:
* Rode `ffprobe` para pegar a duração.
* Retorne o texto m3u8 padrão.


3. Rota `/segment/:id/:time_start`:
* Comando FFMPEG:


```bash
ffmpeg -ss <time_start> \
       -i <caminho_arquivo_original> \
       -t <duracao_segmento> \
       -map 0:v:0 -map 0:a:0 \
       -c:v libx264 -preset ultrafast -crf 23 \
       -c:a aac -b:a 128k \
       -f mpegts \
       pipe:1

```


* Pegue o `stdout` e retorne como `body` da resposta HTTP com Content-Type `video/MP2T`.



# Streaming HLS On-the-Fly

Esta abordagem transformará o Mundam. Ao invés de esperar a conversão completa, o vídeo será processado em pequenos pedaços de ~10 segundos apenas quando o usuário solicitar aquele trecho, economizando espaço em disco e permitindo visualização imediata.

### Contexto: Implementação de Streaming HLS Virtual (Mundam)

Abaixo, apresento o plano de execução dividido em 5 etapas para migrarmos o sistema atual para este novo modelo.

---

### Plano de Execução

**Etapa 1: Configuração do Servidor HTTP Interno (Axum)**
Iremos adicionar as dependências necessárias ao `Cargo.toml` (`axum`, `tokio`, `tower-http`) e configurar um servidor HTTP básico que roda dentro do processo do Tauri (em uma thread separada do `tokio`). Isso servirá como a "ponte" entre o arquivo de vídeo e o frontend.

**Etapa 2: Implementação do Extrator de Metadados (FFprobe)**
Precisamos saber a duração exata do vídeo para "mentir" para o player que temos todos os arquivos prontos. Criaremos um *helper* em Rust que invoca o `ffprobe` e retorna a duração total em segundos.

**Etapa 3: Rota de Geração da Playlist (.m3u8)**
Implementaremos o endpoint `/playlist/:id` (ou similar). Ele não lerá um arquivo do disco. Ele calculará dinamicamente: `Duração Total / 10s = Número de Segmentos` e retornará o texto no formato padrão HLS (M3U8) apontando para a rota de segmentos.

**Etapa 4: Rota de Transcoding de Segmentos (.ts)**
Esta é a etapa mais crítica. Implementaremos o endpoint `/segment/:file_path/:index`.

* Ele calculará o `start_time` baseado no índice.
* Invocará o FFMPEG com `Stdio::piped`.
* Fará o *stream* da saída padrão (stdout) do FFMPEG diretamente para o corpo da resposta HTTP (Body Stream), garantindo latência zero.

**Etapa 5: Integração Frontend (Player HLS)**
No lado do cliente (Javascript), substituiremos a tag `src` direta por uma implementação usando uma biblioteca leve como `hls.js` (ou `video.js` se preferir), apontando para o servidor local do Rust (`http://localhost:PORT/playlist...`).



### 1. Comparativo Geral: Hls.js vs Video.js

A principal diferença é o **escopo**. Pense no **hls.js** como um "motor" e no **video.js** como um "carro completo".

| Característica | **hls.js** | **video.js** |
| --- | --- | --- |
| **O que é?** | Uma biblioteca JavaScript que implementa um **cliente HLS** (HTTP Live Streaming) em cima da tag `<video>` padrão do HTML5 e MSE (Media Source Extensions). | Um **framework de player de vídeo** completo. Ele envolve a tag de vídeo HTML5, fornece uma UI (skin) consistente entre navegadores e suporta plugins. |
| **Interface (UI)** | **Nenhuma.** Ele não possui botões, barras de progresso ou controles de volume. Ele apenas faz o vídeo tocar. A UI é responsabilidade sua. | **Completa.** Já vem com Play/Pause, barra de progresso, volume, tela cheia e legendas prontos e estilizados. |
| **HLS Suporte** | Nativo e altamente otimizado. É o "core business" da biblioteca. | Suporta HLS (via `videojs-http-streaming` ou plugando o próprio hls.js), mas é uma camada abstrata acima disso. |
| **Tamanho (Bundle)** | Muito Leve (~60kb minificado). | Pesado (~400kb+ com dependências e CSS). |
| **Customização** | Total (você constrói a tag `<video>` e os controles do zero). | Limitada à estrutura do framework (sobrescrever CSS ou criar componentes específicos do Video.js). |

---

### 2. Comparativo no Contexto Mundam (Rust/Tauri + SolidJS)

Aqui a análise muda. Estamos falando de uma aplicação Desktop (onde o ambiente do navegador é controlado: WebKit no macOS, WebView2 no Windows) e um framework reativo moderno (SolidJS).

#### **A. Integração com SolidJS**

* **hls.js (Vencedor neste ponto):**
O SolidJS trabalha com referências diretas ao DOM (`ref`). O hls.js se encaixa perfeitamente no modelo mental do Solid:
1. Você cria um `<video ref={videoRef} />`.
2. No `onMount`, você instancia `new Hls()`.
3. Anexa o `videoRef` ao hls.
4. Pronto. O controle de estado (Play/Pause, Tempo) fica 100% no controle dos seus Signals do SolidJS.


* **video.js:**
O Video.js assume que ele é o "dono" do elemento DOM. Ele cria divs, injeta estilos e manipula o DOM diretamente, o que "briga" com o Virtual DOM ou a reatividade fina do SolidJS. Você precisaria criar um "Wrapper" para impedir que o SolidJS renderize novamente o player e destrua a instância do video.js. É mais verboso e propenso a memory leaks se não destruir (`dispose()`) corretamente.

#### **B. Performance e Peso (Tauri)**

* **hls.js (Vencedor):**
No Tauri, cada kilobyte de JS que o WebView precisa carregar e parsear conta para o tempo de inicialização. O hls.js é focado apenas no transporte dos dados (baixar os chunks `.ts` do seu servidor Rust e entregar ao buffer de mídia). Ele é extremamente eficiente em CPU.
* **video.js:**
Carrega muito código que você não usará (suporte a ads, fallbacks para navegadores antigos, analytics, skinning complexo). Para um DAM local, isso é "bloatware" (peso morto).

#### **C. Estilização e UI (Mundam Design)**

* **hls.js:**
Como o Mundam é um DAM, você provavelmente quer que o player de vídeo tenha a "cara" do resto da aplicação (seus botões, suas cores, seu design system). Com hls.js, você desenha seus botões em HTML/CSS normais e apenas chama `videoRef.play()` no clique.
* **video.js:**
O player tem uma opinião visual forte. Para deixá-lo com a cara do Mundam, você terá que brigar com o CSS do video.js (`.vjs-control-bar`, `.vjs-big-play-button`, etc).

---

### Conclusão e Recomendação

Para o **Mundam**, a escolha técnica superior é o **hls.js**.

**Por que usar hls.js?**

1. **Arquitetura Limpa:** O Rust (Backend) entrega os dados. O hls.js (Engine) decodifica. O SolidJS (View) desenha a interface. Cada um com sua responsabilidade única.
2. **Controle Total:** Em um DAM, você pode querer funcionalidades específicas, como: "marcar um ponto de entrada/saída" para cortar o vídeo, ou extrair um frame específico. Fazer isso acessando a API nativa do `<video>` (que o hls.js expõe livremente) é muito mais fácil do que lutar contra a API de abstração do video.js.
3. **Stack Moderno:** O Video.js é excelente para a web aberta (sites de notícias, blogs) onde você precisa garantir compatibilidade com Safari, Chrome, Firefox e Edge de 5 anos atrás. No Tauri, você controla o motor de renderização. Você não precisa da "proteção" que o video.js oferece.
