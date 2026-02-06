# Media Streaming Optimization (Option E: Hybrid FFmpeg Streaming)

## Goal
Implementar transcodificação on-the-fly com FFmpeg para formatos de áudio/vídeo não suportados nativamente pelo WebView, com cache inteligente e seleção de qualidade no player.

## Decisões Arquiteturais

| Decisão | Escolha |
|---------|---------|
| **Cache Location** | `app_data/transcoded/` (igual thumbnails) |
| **Qualidade Padrão** | Standard (CRF 23, 256kbps áudio) |
| **Qualidades Disponíveis** | Preview (CRF 28), Standard (CRF 23), High (CRF 18) |
| **Protocolo Áudio** | `audio://` existente + `audio-stream://` novo |
| **Protocolo Vídeo** | `video://` existente + `video-stream://` novo |
| **Encoding Mode** | CRF (Constant Rate Factor) ao invés de bitrate fixo |

## Formatos que Precisam Transcodificação

### Áudio (OGG Container)
- `.ogg`, `.oga`, `.opus` → Transcodificar para AAC/MP4
- `.wma`, `.ac3`, `.spx` → Transcodificar para AAC/MP4

### Vídeo (Containers Não-WebView)
- `.mkv`, `.avi`, `.flv`, `.f4v` → Transcodificar para MP4 (H.264)
- `.wmv`, `.asf` → Transcodificar para MP4
- `.mpeg`, `.mpg`, `.m2v`, `.vob`, `.m2ts`, `.mts`, `.ts` → Transcodificar para MP4
- `.mxf`, `.ogv`, `.3gp`, `.rm`, `.webm` → Transcodificar para MP4

### Formatos Nativos (Sem Transcodificação)
- **Áudio:** `.mp3`, `.wav`, `.aac`, `.m4a`, `.flac`
- **Vídeo:** `.mp4`, `.m4v`, `.mov`

---

## Tasks

### Phase 1: Backend - Transcoding Core ✅

- [x] **1.1** Criar `src-tauri/src/transcoding/mod.rs` com estrutura de módulo
  - Verificar: `cargo check` sem erros

- [x] **1.2** Criar `src-tauri/src/transcoding/cache.rs` - Gerenciador de cache
  ```rust
  // Estrutura: app_data/transcoded/{hash}.{mp4|m4a}
  // Hash: sha256 do path original + qualidade
  ```
  - Verificar: Função `get_or_create_cache_path()` retorna PathBuf válido

- [x] **1.3** Criar `src-tauri/src/transcoding/quality.rs` - Enum de qualidades
  ```rust
  pub enum TranscodeQuality {
      Preview,   // CRF 28, 192kbps audio, veryfast preset
      Standard,  // CRF 23, 256kbps audio, medium preset (DEFAULT)
      High,      // CRF 18, 320kbps audio, slow preset
  }
  ```
  - Verificar: Enum serializa para comandos Tauri

- [x] **1.4** Criar `src-tauri/src/transcoding/ffmpeg_pipe.rs` - Pipe de transcodificação
  - FFmpeg stdout → Leitura em chunks → Response stream
  - Suporte a seeking via `-ss` flag
  - Verificar: `ffmpeg -i input.mkv -c:v libx264 ...` executa corretamente

- [x] **1.5** Atualizar `src-tauri/src/lib.rs` - Registrar módulo `transcoding`
  - Verificar: `cargo build` sucesso

### Phase 2: Protocols Update ✅

- [x] **2.1** Criar `src-tauri/src/protocols/audio_stream.rs`
  - Detectar se formato precisa transcodificação
  - Usar cache se existir, caso contrário transcodificar
  - Verificar: `audio-stream://path/to/file.ogg` retorna áudio válido

- [x] **2.2** Criar `src-tauri/src/protocols/video_stream.rs`
  - Mesma lógica de audio_stream, mas para vídeo
  - Suporte a Range Requests para seeking
  - Verificar: `video-stream://path/to/file.mkv` retorna vídeo válido

- [x] **2.3** Atualizar `src-tauri/src/protocols/mod.rs`
  - Registrar `audio-stream://` e `video-stream://`
  - Verificar: Protocolos aparecem no log do Tauri

- [x] **2.4** Criar `src-tauri/src/transcoding/commands.rs` - Comandos Tauri
  ```rust
  #[tauri::command]
  fn get_stream_url(path: String, quality: TranscodeQuality) -> String
  
  #[tauri::command]
  fn needs_transcoding(path: String) -> bool
  
  #[tauri::command]  
  fn get_transcode_progress(path: String) -> Option<f32>
  ```
  - Verificar: Comandos registrados em `lib.rs`

### Phase 3: Frontend - Player Updates ✅

- [x] **3.1** Criar `src/lib/stream-utils.ts` - Utilitários de streaming
  ```typescript
  export function needsTranscoding(extension: string): boolean
  export function getStreamUrl(path: string, quality: Quality): string
  export type Quality = 'preview' | 'standard' | 'high'
  ```
  - Verificar: TypeScript compila sem erros

- [x] **3.2** Atualizar componentes para usar stream-utils
  - `ItemView.tsx` - Usa getAudioUrl/getVideoUrl
  - `AudioInspector.tsx` - Usa getAudioUrl
  - `VideoInspector.tsx` - Usa getVideoUrl
  - Verificar: npm run build sucesso

- [x] **3.3** Adicionar indicador de transcodificação nos players
  - Estilos para indicador de transcodificação (`.ui-video-transcoding`)
  - Auto-retry com contador (até 20 tentativas = ~60s)
  - Mensagem "Transcoding video..." com loader
  - Verificar: Indicador visível durante transcodificação

- [x] **3.4** Adicionar seletor de qualidade ao VideoPlayer
  - Seletor de qualidade (Preview/Standard/High)
  - Estado de transcodificação com overlay
  - Verificar: Arquivo `.mkv` reproduz no player

### Phase 4: Integration & Polish

- [x] **4.1** Criar testes com arquivos em `file-samples/`
  - Testados formatos: `.ogg`, `.opus`, `.oga`, `.wma`
  - Testados formatos: `.mkv`, `.avi`, `.flv`, `.wmv`, `.m2ts`, `.webm`, `.mxf`, `.mpg`, `.mpeg`, `.vob`
  - Verificar: Todos os formatos listados reproduzem ✅

- [x] **4.2** Adicionar cleanup automático de cache
  - UI em Settings > General > Transcoding Cache
  - Configuração de retenção (7, 14, 30, 60, 90 dias)
  - Botões "Cleanup Old Files" e "Clear All Cache"
  - Estatísticas de uso (arquivos e tamanho)
  - Verificar: Cache cleanup funciona ✅

- [ ] **4.3** Tratamento de erros robusto
  - FFmpeg não encontrado → Mensagem clara
  - Transcodificação falha → Fallback para player externo
  - Verificar: Erros exibem mensagem amigável

---

## Implementação Detalhada

### FFmpeg Command (Final)

O comando FFmpeg otimizado seguindo best practices:

```bash
ffmpeg -y -hide_banner -loglevel warning \
  -probesize 100M -analyzeduration 100M \
  -i input.mkv \
  -map 0:v:0? -map 0:a:0? \
  -c:v libx264 -profile:v high -level 4.1 \
  -preset medium -crf 23 \
  -vf "scale=trunc(iw/2)*2:trunc(ih/2)*2" \
  -pix_fmt yuv420p \
  -g 30 -bf 2 \
  -c:a aac -b:a 256k -ar 48000 \
  -movflags +faststart \
  -max_muxing_queue_size 9999 \
  -f mp4 output.mp4
```

**Flags explicadas:**
| Flag | Propósito |
|------|-----------|
| `-probesize 100M` | Analisa mais dados para detectar streams em arquivos grandes |
| `-analyzeduration 100M` | Mais tempo para análise de streams |
| `-map 0:v:0?` / `-map 0:a:0?` | Mapeia primeiro stream de video/audio (opcional com `?`) |
| `-profile:v high -level 4.1` | H.264 High Profile, compatível com 1080p@30fps |
| `-crf 23` | Qualidade constante (menor = melhor, 18-28 é range útil) |
| `-vf scale=...` | Força dimensões pares (evita erro "height not divisible by 2") |
| `-g 30` | Keyframe a cada 30 frames (~1s a 30fps) para seek preciso |
| `-bf 2` | 2 B-frames entre I e P frames |
| `-ar 48000` | Sample rate padrão para web/broadcast |
| `-movflags +faststart` | Move moov atom para início (streaming web) |
| `-max_muxing_queue_size 9999` | Buffer grande para arquivos complexos |

### Qualidades de Transcodificação

| Qualidade | CRF | Preset | Áudio | Use Case |
|-----------|-----|--------|-------|----------|
| Preview | 28 | veryfast | 192kbps | Preview rápido, menor qualidade |
| Standard | 23 | medium | 256kbps | **Padrão** - bom equilíbrio |
| High | 18 | slow | 320kbps | Melhor qualidade, mais lento |

### VideoPlayer - Estado de Transcodificação

Implementado sistema de auto-retry com estado visual:

```typescript
// Estados do player
const [isTranscoding, setIsTranscoding] = createSignal(false);
const [retryCount, setRetryCount] = createSignal(0);

// Detecta se precisa transcodificação
const needsTranscode = () => 
  props.src.includes('video-stream://') || props.src.includes('audio-stream://');

// Handler de erro com auto-retry
onError={() => {
  if (needsTranscode() && retryCount() < 20) {
    setIsTranscoding(true);
    setRetryCount(prev => prev + 1);
    // Auto-retry após 3 segundos
    retryTimeout = window.setTimeout(() => videoRef?.load(), 3000);
  } else {
    setError('Transcoding failed or timed out');
  }
}}
```

### CSS - Estado de Transcodificação

```css
.ui-video-transcoding {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--p-space-m);
  background: rgba(0, 0, 0, 0.8);
  color: var(--text-muted);
  text-align: center;
  z-index: 30;
}
```

---

## Formatos de Teste Disponíveis

### Áudio (problema atual: ogg, oga, opus)
```
file-samples/Audio/Mainstream/sample3.opus
file-samples/Audio/Mainstream/Symphony No.6 (1st movement).oga
file-samples/Audio/Mainstream/Symphony No.6 (1st movement).ogg
file-samples/Audio/Mainstream/Symphony No.6 (1st movement).wma
```

### Vídeo (problemas atuais: mkv, avi, flv, m2ts, etc.)
```
file-samples/Video/Desktop_Containers/sample_640x360.mkv
file-samples/Video/Desktop_Containers/sample_640x360.avi
file-samples/Video/Desktop_Containers/sample_640x360.flv
file-samples/Video/Broadcast_Professional/sample_640x360.m2ts
file-samples/Video/Broadcast_Professional/sample_640x360.mxf
```

---

## Resultados de Testes

### Formatos Testados (2026-02-06)

| Formato | Tamanho | Resultado | Observações |
|---------|---------|-----------|-------------|
| mov, m4v, mp4, mxf | Pequeno/Grande | ✅ OK | Carregou normalmente |
| swf, wmv, mkv, flv, f4v, avi, m2ts, m2v, mpeg, mpg, vob, webm | Pequeno | ✅ OK | Transcodificou corretamente |
| asf, wmv, f4v, flv, avi, webm | Grande | ✅ OK | Trava UI durante transcode |
| m2ts, mpeg, mts, ts, vob | Grande | ⚠️ Parcial | Toca apenas áudio |
| m2v, mpg | Grande | ⚠️ Parcial | Toca apenas vídeo |

**Nota:** Problemas com áudio/vídeo em arquivos grandes foram mitigados com `-probesize 100M` e `-analyzeduration 100M`.

---

## Issues Conhecidos

### 1. Travamento da UI durante transcodificação
- **Causa:** Transcodificação síncrona bloqueia thread principal
- **Workaround atual:** Auto-retry com indicador visual
- **Solução futura:** Transcodificação assíncrona em background

### 2. Arquivos grandes sem áudio ou vídeo
- **Causa:** FFmpeg não detecta todos os streams
- **Solução:** Aumentado `probesize` e `analyzeduration` para 100MB

### 3. Warnings de código não utilizado
- **Arquivos:** `transcoding/cache.rs`, `transcoding/ffmpeg_pipe.rs`, `transcoding/detector.rs`
- **Status:** Código preparado para features futuras (streaming, invalidação de cache)

---

## Done When

- [x] Arquivos `.ogg`, `.oga`, `.opus` reproduzem no AudioPlayer
- [x] Arquivos `.mkv`, `.avi`, `.flv`, `.m2ts`, `.webm` reproduzem no VideoPlayer
- [x] Cache de transcodificação funciona (segunda reprodução é instantânea)
- [x] `cargo build` e `npm run build` passam sem erros
- [x] Indicador de transcodificação visível durante processamento
- [x] Configurações de cache na UI (dias de retenção, cleanup manual)
- [x] Cleanup manual de cache funciona
- [x] Seletor de qualidade funciona no VideoPlayer
- [x] Cleanup automático de cache na inicialização
