# Media Streaming Optimization (Option E: Hybrid FFmpeg Streaming)

## Goal
Implementar transcodificação on-the-fly com FFmpeg para formatos de áudio/vídeo não suportados nativamente pelo WebView, com cache inteligente e seleção de qualidade no player.

## Decisões Arquiteturais

| Decisão | Escolha |
|---------|---------|
| **Cache Location** | `app_data/transcoded/` (igual thumbnails) |
| **Qualidade Padrão** | Preview (2 Mbps vídeo, 128kbps áudio) |
| **Qualidades Disponíveis** | Preview, Standard, High |
| **Protocolo Áudio** | `audio://` existente + `audio-stream://` novo |
| **Protocolo Vídeo** | `video://` existente + `video-stream://` novo |

## Formatos que Precisam Transcodificação

### Áudio (OGG Container)
- `.ogg`, `.oga`, `.opus` → Transcodificar para AAC/MP4
- `.wma`, `.ac3`, `.spx` → Transcodificar para AAC/MP4

### Vídeo (Containers Não-WebView)
- `.mkv`, `.avi`, `.flv`, `.f4v` → Transcodificar para MP4 (H.264)
- `.wmv`, `.asf` → Transcodificar para MP4
- `.mpeg`, `.mpg`, `.m2v`, `.vob`, `.m2ts`, `.mts`, `.ts` → Transcodificar para MP4
- `.mxf`, `.ogv`, `.3gp`, `.rm` → Transcodificar para MP4

### Formatos Nativos (Sem Transcodificação)
- **Áudio:** `.mp3`, `.wav`, `.aac`, `.m4a`, `.flac`
- **Vídeo:** `.mp4`, `.m4v`, `.mov`, `.webm`

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
      Preview,   // 2 Mbps video, 128kbps audio
      Standard,  // 5 Mbps video, 256kbps audio  
      High,      // 10 Mbps video, 320kbps audio
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

- [ ] **3.3** Adicionar indicador de transcodificação nos players
  - Estilos para indicador de transcodificação
  - Usar tokens de `tokens.css`
  - Verificar: Indicador visível durante transcodificação

- [ ] **3.4** Adicionar seletor de qualidade ao VideoPlayer
  - Seletor de qualidade (Preview/Standard/High)
  - Estado de transcodificação com overlay
  - Verificar: Arquivo `.mkv` reproduz no player

### Phase 4: Integration & Polish

- [ ] **4.1** Criar testes com arquivos em `file-samples/`
  - Testar formatos: `.ogg`, `.opus`, `.oga`, `.wma`
  - Testar formatos: `.mkv`, `.avi`, `.flv`, `.wmv`, `.m2ts`
  - Verificar: Todos os formatos listados reproduzem

- [ ] **4.2** Adicionar cleanup automático de cache
  - Limpar arquivos com mais de 30 dias
  - Limite de espaço em disco (opcional: configurável)
  - Verificar: Cache cleanup funciona

- [ ] **4.3** Tratamento de erros robusto
  - FFmpeg não encontrado → Mensagem clara
  - Transcodificação falha → Fallback para player externo
  - Verificar: Erros exibem mensagem amigável

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

## FFmpeg Commands Reference

### Audio Transcoding (to AAC)
```bash
ffmpeg -i input.ogg -c:a aac -b:a 128k -f mp4 output.m4a
```

### Video Transcoding (to H.264)
```bash
# Preview (2 Mbps)
ffmpeg -i input.mkv -c:v libx264 -preset fast -b:v 2M -c:a aac -b:a 128k -f mp4 output.mp4

# Standard (5 Mbps)
ffmpeg -i input.mkv -c:v libx264 -preset medium -b:v 5M -c:a aac -b:a 256k -f mp4 output.mp4

# High (10 Mbps)
ffmpeg -i input.mkv -c:v libx264 -preset slow -b:v 10M -c:a aac -b:a 320k -f mp4 output.mp4
```

### Streaming (pipe output)
```bash
ffmpeg -i input.mkv -c:v libx264 -preset ultrafast -b:v 2M -c:a aac -movflags frag_keyframe+empty_moov -f mp4 pipe:1
```

---

## Done When

- [ ] Arquivos `.ogg`, `.oga`, `.opus` reproduzem no AudioPlayer
- [ ] Arquivos `.mkv`, `.avi`, `.flv`, `.m2ts` reproduzem no VideoPlayer
- [ ] Seletor de qualidade funciona no VideoPlayer
- [ ] Cache de transcodificação funciona (segunda reprodução é instantânea)
- [ ] `cargo build` e `npm run build` passam sem erros
