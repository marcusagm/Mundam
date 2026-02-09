# Relatório de Implementação: HLS Video Streaming

**Data:** 09/02/2026
**Status:** Concluído
**Autor:** Antigravity (Assistant)

## 1. Resumo Executivo

O sistema de streaming de vídeo foi refatorado para utilizar o protocolo **HLS (HTTP Live Streaming)** para formatos não nativos (MKV, AVI, TS, VOB), substituindo o método anterior de transcodificação síncrona completa.

Esta mudança permite:
- **Início instantâneo** de vídeos grandes (antes demorava minutos).
- **Seek rápido** e eficiente (processa apenas pequenos segmentos de 10s sob demanda).
- **Seleção de qualidade** (Preview, Standard, High) em tempo real.
- **Robustez** contra falhas de processo e formatos corrompidos.

## 2. Arquitetura Implementada

### 2.1 Backend (Rust / Axum)

Um servidor HTTP leve foi embutido na aplicação Tauri (porta 9876), expondo os seguintes endpoints:

- `GET /health`: Healthcheck.
- `GET /probe/*path`: Retorna metadados do vídeo (duração, codec) usando `ffprobe`. Decora se o vídeo é "nativo" (suportado pelo player web) ou precisa de HLS.
- `GET /playlist/*path?quality=...`: Gera dinamicamente um arquivo `.m3u8` virtual, mapeando o vídeo em segmentos de 10 segundos.
- `GET /segment/*path/:index?quality=...`: Executa o transcoding on-demand de um único segmento usando `ffmpeg`.

### 2.2 Gerenciamento de Processos

Implementou-se um `ProcessManager` robusto:
- Cada segmento gera um processo `ffmpeg` isolado.
- **Debounce:** Requisições rápidas de seek cancelam os processos anteriores imediatamente (`kill` / `taskkill`).
- **Cleanup:** Uma tarefa em background varre processos "stale" (ativos há >30s) a cada 10s e os elimina para evitar vazamento de memória/CPU.

### 2.3 Melhorias de FFmpeg

Para suportar formatos broadcast complexos (VOB, M2TS, MPEG-PS) que falhavam no seek rápido:
- Adicionadas flags de análise profunda: `-analyzeduration 100M`, `-probesize 50M`.
- Ignorar dados desconhecidos: `-ignore_unknown`.
- Regenerar timestamps: `-fflags +genpts`.
- Desativar legendas (fonte comum de crashes no seek): `-sn`.
- Seek **antes** do input (`-ss`) para velocidade.

### 2.4 Cache

O sistema de cache (`TranscodeCache`) foi atualizado para:
- Suportar diretórios aninhados (`hls_segments`).
- Incluir parâmetro de qualidade na chave de cache (segmentos Standard não conflitam com High).
- Limpeza recursiva de segmentos antigos.

## 3. Frontend (SolidJS / hls.js)

### 3.1 Integração HLS
- Biblioteca `hls.js` integrada via hook customizado.
- Detecção automática: Se `probe` retornar `native: false`, usa HLS. Caso contrário, usa protocolo `video://` direto (zero transcoding).

### 3.2 UX Improvements
- **Seletor de Qualidade:** Botão sempre visível para streams HLS. A troca religa o stream na nova qualidade instantaneamente.
- **Single Active Player:** Lógica global (`videoStore`, `audioStore`) garante que apenas uma mídia toque por vez.
    - Play no Vídeo → Pausa todos os Áudios.
    - Play no Áudio → Pausa todos os Vídeos.
    - Play no Inspector → Pausa ItemView (e vice-versa).

## 4. Testes Realizados

- **Formatos:** MKV (H.264/AAC), AVI (DivX), TS/M2TS (MPEG-2), VOB (MPEG-PS), MP4 (Nativo).
- **Stress:** Seek rápido (scrubbing) funciona sem travar a UI ou acumular processos.
- **Qualidade:** Troca entre Preview/Standard/High altera visivelmente a resolução e bitrate.
- **Erro:** Arquivos corrompidos mostram erro amigável ou tentam recuperar.

## 5. Próximos Passos (Sugestões)

- **Hardware Acceleration:** Habilitar NVENC/VideoToolbox no FFmpeg se disponível.
- **HTTPS:** Se necessário futuramente (atualmente localhost é seguro).
- **Preload Inteligente:** Pré-carregar próximo segmento para reduzir latência em conexões lentas (já gerenciado parcialmente pelo hls.js).
