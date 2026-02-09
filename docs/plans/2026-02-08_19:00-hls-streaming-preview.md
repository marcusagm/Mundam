# HLS On-the-Fly Video Streaming

## Goal

Substituir o sistema de transcoding completo (`transcode_sync`) por **streaming HLS segmentado** para preview de vídeos pesados (mkv, avi, m2ts, etc). O sistema deve permitir **início instantâneo** e **seeking rápido**, processando apenas os trechos visualizados.

---

## Arquitetura Atual (Problema)

```
Frontend                          Backend (Rust)
    │                                 │
    │  video-stream://file.mkv       │
    ├────────────────────────────────►│
    │                                 │ transcode_sync() ← BLOQUEIA até terminar
    │                                 │ (pode levar minutos para 1GB+)
    │◄────────────────────────────────┤
    │  Arquivo MP4 completo           │
```

**Gargalo:** `src-tauri/src/protocols/video_stream.rs:78` chama `transcoder.transcode_sync()` que processa o arquivo **inteiro** antes de servir.

---

## Arquitetura Nova (Solução HLS)

```
Frontend (SolidJS)                   Backend (Rust/Axum)
    │                                     │
    │  GET /probe/file.mkv               │
    ├────────────────────────────────────►│ ffprobe → {native: false, duration: 3600}
    │◄────────────────────────────────────┤
    │                                     │
    │  GET /playlist/file.mkv            │
    ├────────────────────────────────────►│ Gera M3U8 virtual (360 segments de 10s)
    │◄────────────────────────────────────┤
    │                                     │
    │  GET /segment/file.mkv/0           │
    ├────────────────────────────────────►│ ffmpeg -ss 0 -t 10 → 10s de video
    │◄────────────────────────────────────┤ (serve do cache se existir)
    │                                     │
    │  *** USER SEEKS TO 30:00 ***       │
    │                                     │
    │  GET /segment/file.mkv/180         │ 
    ├────────────────────────────────────►│ ffmpeg -ss 1800 -t 10 → só esse trecho!
    │◄────────────────────────────────────┤
```

---

## Componentes Existentes para Reutilizar

| Componente | Arquivo | O que já faz |
|------------|---------|--------------|
| **VideoPlayer UI** | `src/components/ui/VideoPlayer.tsx` | Controls, fullscreen, quality selector, retry logic |
| **stream-utils.ts** | `src/lib/stream-utils.ts` | `needsVideoTranscoding()`, `getVideoUrl()` |
| **detector.rs** | `src-tauri/src/transcoding/detector.rs` | `needs_transcoding()`, `is_native_format()` |
| **TranscodeCache** | `src-tauri/src/transcoding/cache.rs` | Hash, cleanup, stats - **REUTILIZAR PARA SEGMENTS** |
| **ffmpeg.rs** | `src-tauri/src/ffmpeg.rs` | `get_ffmpeg_path()`, subprocess wrapper |
| **video_stream.rs** | `src-tauri/src/protocols/video_stream.rs` | Handler atual - **SUBSTITUIR** |

---

## Decisões Técnicas

| Aspecto | Decisão | Justificativa |
|---------|---------|---------------|
| **Cache** | Disk Cache (TranscodeCache) | Sistema já existe, integrado com GeneralPanel.tsx |
| **Processos** | Debounce + Cancel | Essencial para hover scrub futuro |
| **Fallback** | Reutilizar `is_native_format()` | Já existe em detector.rs e stream-utils.ts |

---

## Tasks

### Phase 1: Backend - Servidor HTTP Axum

- [x] **1.1** Adicionar deps ao `Cargo.toml`: `axum = "0.7"`, `tower-http = { version = "0.5", features = ["cors"] }` → Verify: `cargo check`
- [x] **1.2** Criar `src-tauri/src/streaming/mod.rs` com `StreamingServer::new(port, app_handle)` → Verify: Compila
- [x] **1.3** Criar `src-tauri/src/streaming/server.rs` com Axum router e CORS → Verify: Compila
- [x] **1.4** Integrar start no `src-tauri/src/lib.rs` usando `tauri::async_runtime::spawn` → Verify: Log "HLS server on :9876"
- [x] **1.5** Adicionar rota `GET /health` → Verify: `curl localhost:9876/health` = 200

### Phase 2: Backend - Rota /probe

- [x] **2.1** Criar `src-tauri/src/streaming/probe.rs` com `get_video_info(path)` usando ffprobe JSON → Verify: Unit test
- [x] **2.2** Criar struct `VideoInfo { duration_secs: f64, is_native: bool }` → Verify: Compila
- [x] **2.3** Reutilizar `detector::is_native_format()` para determinar `is_native` → Verify: Coerência com detector.rs
- [x] **2.4** Adicionar rota `GET /probe/{path}` → Verify: `curl localhost:9876/probe/test.mkv` = JSON

### Phase 3: Backend - Rota /playlist (M3U8 Virtual)

- [x] **3.1** Criar `src-tauri/src/streaming/playlist.rs` com `generate_m3u8(path, duration)` → Verify: Output M3U8 válido
- [x] **3.2** Adicionar rota `GET /playlist/{path}` com Content-Type `application/vnd.apple.mpegurl` → Verify: VLC abre
- [x] **3.3** M3U8 deve apontar para `/segment/{path}/{index}` → Verify: Paths corretos

### Phase 4: Backend - Rota /segment (Transcoding On-Demand)

- [x] **4.1** Criar `src-tauri/src/streaming/segment.rs` com `transcode_segment(path, index, segment_duration)` → Verify: Compila
- [x] **4.2** Reutilizar `ffmpeg::get_ffmpeg_path()` para localizar binary → Verify: Usa bundled ou system
- [x] **4.3** Implementar cache de segmentos usando `TranscodeCache::get_cache_path()` com suffix `-seg{index}` → Verify: Reutiliza cache
- [x] **4.4** Spawn FFMPEG: `ffmpeg -ss {start} -t 10 -i {input} -c:v libx264 -preset ultrafast -c:a aac -f mpegts pipe:1` → Verify: Binary OK
- [x] **4.5** Stream stdout para response body usando `axum::body::Body::from_stream()` → Verify: Segment plays
- [x] **4.6** Adicionar rota `GET /segment/{path}/{index}` → Verify: `curl` retorna bytes

### Phase 5: Backend - Cancelamento de Processos

- [x] **5.1** Criar `src-tauri/src/streaming/process_manager.rs` com `HashMap<SegmentKey, ChildProcess>` → Verify: Compila
- [x] **5.2** Implementar `cancel_segment(path, index)` que mata processo FFMPEG → Verify: Log "Cancelled"
- [x] **5.3** Adicionar header `X-Request-Id` para tracking → Verify: Headers presentes
- [x] **5.4** Implementar cleanup automático de processos órfãos (timeout 30s) → Verify: Sem memory leak

### Phase 6: Frontend - Integração hls.js

- [x] **6.1** Instalar: `npm install hls.js` → Verify: `npm ls hls.js`
- [x] **6.2** Criar `src/lib/hls-player.ts` com classe `HlsPlayerManager` → Verify: Compila
- [x] **6.3** Métodos: `attach(videoEl, playlistUrl)`, `detach()`, `destroy()` → Verify: TypeScript OK
- [x] **6.4** Criar hook `createHlsPlayer(videoRef: () => HTMLVideoElement | undefined)` para SolidJS → Verify: Compila

### Phase 7: Frontend - Modificar VideoPlayer

- [x] **7.1** Modificar `src/lib/stream-utils.ts`: adicionar `getHlsPlaylistUrl(path)` → Verify: Retorna `http://localhost:9876/playlist/...`
- [x] **7.2** Modificar `VideoPlayer.tsx`: detectar se src é HLS (`.m3u8` ou `localhost:9876`) → Verify: Conditional logic
- [x] **7.3** Se HLS: usar hls.js via hook. Se não: usar `<video src>` direto → Verify: Ambos funcionam
- [x] **7.4** Implementar debounce (150ms) no seek handler → Verify: Rapid seek não spawna múltiplos processes

### Phase 8: Frontend - Modificar VideoPlayer Renderer

- [x] **8.1** Modificar `src/components/features/itemview/renderers/video/VideoPlayer.tsx` → Verify: Usa novo fluxo
- [x] **8.2** Chamar `/probe` no mount para decidir entre native e HLS → Verify: Decision logged
- [x] **8.3** Se native: continuar usando `video://` protocolo existente → Verify: MP4 funciona
- [x] **8.4** Se HLS: usar playlist URL → Verify: MKV funciona

### Phase 9: Verification & Polish

- [x] **9.1** Testar MKV 1GB+ → Verify: Inicia em <2s
- [x] **9.2** Testar seek para final do vídeo → Verify: Processa só 10s
- [x] **9.3** Testar MP4 nativo → Verify: Zero transcoding (bypass)
- [x] **9.4** Verificar cache via GeneralPanel → Verify: Segments aparecem
- [x] **9.5** Cleanup cache → Verify: Segments removidos
- [x] **9.6** Stress test: 10 seeks em 2s → Verify: Só último processado

### Phase 10: Refinements - Quality & Robustness

- [x] **10.1** Backend: Aceitar parâmetro `quality` (preview, standard, high) em `/playlist` e `/segment` → Verify: Cache distinct
- [x] **10.2** Backend: Ajustar FFmpeg (CRF, scale) baseado na qualidade → Verify: Tamanho do arquivo muda
- [x] **10.3** Frontend: Atualizar `VideoPlayer.tsx` para mostrar seletor de qualidade em HLS → Verify: UI visible
- [x] **10.4** Backend: Robustez para VOB/M2TS (analyzeduration, probesize, genpts) → Verify: Seek sem erro
- [x] **10.5** Frontend: Single Active Player (pausa automática entre áudio/vídeo) → Verify: Somente 1 mídia toca

---

## Done When

- [x] Vídeos MKV/AVI/M2TS iniciam em <2 segundos
- [x] Seeking processa apenas ~10s (não o vídeo inteiro)
- [x] MP4/MOV nativos usam bypass (video:// existente)
- [x] Segments cacheados integrados com GeneralPanel.tsx (stats, cleanup)
- [x] Seek rápido cancela processos anteriores (debounce + kill)

---

## Arquivos a Criar

```
src-tauri/src/streaming/
├── mod.rs              # Exports + StreamingServer struct
├── server.rs           # Axum router + CORS + start/stop
├── probe.rs            # /probe endpoint + ffprobe wrapper
├── playlist.rs         # /playlist endpoint + M3U8 generator
├── segment.rs          # /segment endpoint + FFMPEG transcoder
└── process_manager.rs  # Process tracking + cancellation

src/lib/
└── hls-player.ts       # hls.js wrapper + SolidJS hook
```

---

## Arquivos a Modificar

| Arquivo | Modificação |
|---------|-------------|
| `Cargo.toml` | Adicionar `axum`, `tower-http` |
| `src-tauri/src/lib.rs` | Spawn streaming server |
| `src/lib/stream-utils.ts` | `getHlsPlaylistUrl()` |
| `src/components/ui/VideoPlayer.tsx` | Integrar hls.js conditionally |
| `src/components/features/itemview/renderers/video/VideoPlayer.tsx` | Usar /probe para decidir fluxo |

---

## Notes

### Comandos FFMPEG

**Probe (JSON):**
```bash
ffprobe -v quiet -print_format json -show_format -show_streams <input>
```

**Segment HLS (MPEG-TS):**
```bash
ffmpeg -ss <start_secs> -t 10 \
       -i <input> \
       -map 0:v:0 -map 0:a:0? \
       -c:v libx264 -preset ultrafast -crf 23 \
       -c:a aac -b:a 128k \
       -f mpegts \
       pipe:1
```

### M3U8 Format Example

```m3u8
#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:10
#EXT-X-MEDIA-SEQUENCE:0
#EXTINF:10.0,
/segment/path/to/file.mkv/0
#EXTINF:10.0,
/segment/path/to/file.mkv/1
...
#EXT-X-ENDLIST
```

### Cache Key para Segments

Reutilizar `TranscodeCache::generate_cache_key()` com suffix:
```rust
format!("{}-seg{:05}", base_hash, segment_index)
```

### Port Selection

Usar porta fixa `9876` em dev. Para produção, considerar porta dinâmica ou IPC.
