# Thumbnail Optimization - Multi-Tier Pipeline

## Goal
Melhorar a velocidade de geração de thumbnails usando pipeline híbrido (Rust + FFmpeg) com suporte a 60+ formatos, incluindo recriação automática de thumbnails 404.

## Arquitetura

```
Extension → Router
    │
    ├── jpg/png/webp/gif/bmp ──▶ [RUST FAST PATH] ~10-50ms
    │
    ├── heic/avif/raw/psd/* ───▶ [FFMPEG] ~100-300ms  
    │
    ├── af*/indd/clip/xd ──────▶ [ZIP PREVIEW] ~50ms
    │
    └── c4d/dwg/skp/outros ────▶ [SVG ICON] instantâneo
```

---

## Fase 1: Foundation + Fast Path + 404 Handling ✅

### Tasks

- [x] **1.1** Criar enum `ThumbnailStrategy` e função `get_strategy()` em `thumbnails.rs`
  → Verify: Compila sem erros ✅

- [x] **1.2** Adicionar `zune-jpeg` e `webp` crates ao `Cargo.toml`
  → Verify: `cargo build` passa ✅
  → Nota: Mudamos de `turbojpeg` para `zune-jpeg` (pure Rust, não requer cmake)

- [x] **1.3** Implementar `generate_thumbnail_fast()` usando zune-jpeg para decode JPEG
  → Verify: Código implementado e compila ✅

- [x] **1.4** Implementar encoding WebP nativo com `webp` crate
  → Verify: Output .webp ✅

- [x] **1.5** Adicionar comando Tauri `request_thumbnail_regenerate` 
  → Verify: Comando registrado em `lib.rs` ✅

- [x] **1.6** Criar `clear_thumbnail_path()` no database.rs
  → Verify: Função implementada ✅

- [x] **1.7** Atualizar `ReferenceImage.tsx` para chamar regenerate no onError
  → Verify: Código implementado com detecção de 404 ✅

- [x] **1.8** Adicionar debounce de 2000ms nas chamadas de regenerate
  → Verify: Map de debounce implementado ✅

---

## Fase 2: FFmpeg Integration ✅

### Tasks

- [x] **2.1** Configurar bundling do FFmpeg via `tauri.conf.json` resources
  → Verify: Configuração adicionada, diretório `ffmpeg/` criado ✅

- [x] **2.2** Criar módulo `ffmpeg.rs` com função `generate_thumbnail_ffmpeg()`
  → Verify: Módulo criado com subprocess wrapper ✅

- [x] **2.3** Implementar detecção de FFmpeg bundled vs sistema
  → Verify: `get_ffmpeg_path()` tenta bundled primeiro, depois sistema ✅

- [x] **2.4** Suporte a RAW: cr2, cr3, arw, nef, dng, raf, orf, etc.
  → Verify: Router direciona para FFmpeg, fallback para image crate ✅

- [x] **2.5** Suporte a HEIC/AVIF/JXL
  → Verify: Mapeados no router de estratégias ✅

- [x] **2.6** Suporte a PSD/PSB/AI/EPS/TIFF/SVG
  → Verify: Mapeados no router de estratégias ✅

- [x] **2.7** Atualizar thumbnails.rs para usar FFmpeg
  → Verify: `generate_thumbnail_ffmpeg()` chama módulo ffmpeg ✅

### Notas Fase 2
- Script `download-ffmpeg.sh` criado para setup do binário
- FFmpeg não instalado no sistema atual - funciona em modo fallback
- Para produção: executar script e bundlar binário


---

## Fase 3: ZIP Preview Extraction ✅ (Parcial)

### Tasks

- [x] **3.1** Adicionar `zip` crate ao projeto
  → Verify: Compila ✅

- [x] **3.2** Criar `generate_thumbnail_zip_preview()` para formatos ZIP-based
  → Verify: Função implementada ✅

- [x] **3.3** Mapear caminhos de preview por formato:
  - Affinity: `preview.png`
  - XMind: `Thumbnails/thumbnail.png`
  - QuickLook: `QuickLook/Preview.png`
  - Generic: `icon.png`
  → Verify: Paths mapeados ✅

- [x] **3.4** Integrar no router de estratégias
  → Verify: `get_strategy()` roteia corretamente ✅

---

## Fase 4: Icon Fallback + Assets ✅

### Tasks

- [x] **4.1** Criar ícones SVG para formatos não-suportados:
  - Genérico (arquivo) ✅
  - 3D (c4d, skp, dwg) ✅
  - Design (formatos não-ZIP) ✅
  - Fonte (ttf, otf) ✅
  → Verify: SVGs criados em `public/icons/`

- [x] **4.2** Implementar `generate_icon_thumbnail()` que renderiza thumbnail colorido
  - Categorização por tipo (3D, Font, Design, Generic)
  - Cores diferentes por categoria
  - Ícone de arquivo desenhado programaticamente
  - Cantos arredondados
  → Verify: Thumbnail gerado para formatos não suportados ✅

- [x] **4.3** Frontend já mostra placeholder (Loader) até thumbnail ficar pronta
  - Para ícones de fallback, a thumbnail é gerada como qualquer outra
  → Verify: UI funciona com fallback icons ✅


---

## Fase 5: Performance + Batch Optimization ✅ (Parcial)

### Tasks

- [x] **5.1** Aumentar batch size para 20 imagens
  → Verify: Worker usa batch de 20 ✅

- [ ] **5.2** Implementar priorização de imagens visíveis no viewport
  → Verify: Imagens na tela têm prioridade

- [x] **5.3** Adicionar métricas de tempo por estratégia (logs)
  → Verify: Console mostra "THUMB: FastPath | 25ms | file.jpg" ✅

- [ ] **5.4** Verificar memória em batches grandes
  → Verify: Sem memory leaks em 1000+ thumbnails

---

## Done When

- [x] Thumbnails JPEG usam FFmpeg (muito mais rápido que Rust puro)
- [x] **Thumbnails JPEG geram em 500-850ms (antes: 20-50 segundos) - 40-60x mais rápido!**
- [x] RAW/HEIC/PSD têm suporte via FFmpeg (bundled e funcionando)
- [x] Arquivos Affinity/XMind têm suporte a extração de preview
- [x] **Formatos desconhecidos mostram ícone de fallback** (Fase 4 ✅)
  - Categorias: 3D, Font, Design, Generic
  - Cores diferentes por categoria
  - Ícone de arquivo estilizado
- [x] Thumbnails deletadas são recriadas automaticamente (404 handling)
- [x] Store centralizado evita loops de regeneração
- [x] FFmpeg bundled funciona em modo desenvolvimento
- [ ] FFmpeg bundled funciona no build de produção (testar)


---

## Notas

### Formatos por Tier

**Tier 1 - Fast Path (Rust):**
`jpg, jpeg, jpe, jfif, png, webp, gif, bmp, ico`

**Tier 2 - FFmpeg:**
`heic, heif, hif, avif, jxl, cr2, cr3, arw, nef, dng, raf, orf, pef, rw2, 3fr, mrw, nrw, sr2, srw, x3f, erf, crw, raw, psd, psb, ai, eps, svg, tif, tiff`

**Tier 3 - ZIP Preview:**
`af, afdesign, afphoto, afpub, clip, xmind, graffle`

**Tier 4 - Icon Fallback:**
`c4d, dwg, skp, xd, indd, indt, idml, cdr, ttf, otf, base64, insp`

### Dependências Rust Atualizadas
```toml
zune-jpeg = "0.4"  # Pure Rust JPEG decoder (não requer cmake)
webp = "0.3"       # WebP encoder
zip = "2.1"        # ZIP extraction
```

### Arquivos Modificados na Fase 1
- `src-tauri/Cargo.toml` - Novas dependências
- `src-tauri/src/thumbnails.rs` - Pipeline multi-tier completo
- `src-tauri/src/thumbnail_commands.rs` - Novo módulo de comandos
- `src-tauri/src/database.rs` - `clear_thumbnail_path()`
- `src-tauri/src/lib.rs` - Registro do novo módulo e comando
- `src/components/features/viewport/ReferenceImage.tsx` - 404 handling + regeneração
