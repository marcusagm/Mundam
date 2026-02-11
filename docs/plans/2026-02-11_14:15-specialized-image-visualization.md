# Plano de Implementação: Visualização de Imagens Especializadas (RAW, HEIC, HDR, EPS, etc.)

**Data:** 2026-02-11
**Hora Atualizada:** 18:15
**Autor:** Antigravity (Assistant)

## 1. Contexto e Objetivo

O Mundam já possui suporte robusto para thumbnails de diversos formatos. No entanto, a visualização em "tela cheia" (ItemView) falha para formatos que o WebView não suporta nativamente (RAW, HEIC, HDR, PSD, etc.). 

Este plano estabelece a criação de um sistema de **Estratégias de Visualização** (similar ao de Thumbnails) para processar esses arquivos "on-the-fly" através de um protocolo customizado (`asset://image`), garantindo uma visualização fluida e de alta qualidade.

## 2. Mudanças Arquiteturais

### 2.1 Expansão do Sistema de Formatos (`src-tauri/src/formats`)
Introduzir um novo campo `preview_strategy` no registro mestre de formatos para desacoplar a lógica de "como visualizar" da extensão do arquivo.

- **`PreviewStrategy::BrowserNative`**: O navegador renderiza diretamente (JPG, PNG, WebP, SVG).
- **`PreviewStrategy::Raw`**: Extração de preview via `rsraw` (LibRaw).
- **`PreviewStrategy::Ffmpeg`**: Extração via FFmpeg (HEIC, AVIF, HDR).
- **`PreviewStrategy::NativeExtractor`**: Lógica Rust customizada (PSD, Affinity, CLIP).
- **`PreviewStrategy::Convert`**: Conversão via crate `image` para formatos "estranhos" (DDS, EXR, TGA).

### 2.2 Protocolo Customizado (`src-tauri/src/protocols/image.rs`)
Refatorar o handler do protocolo para consultar a estratégia do formato e rotear para o processador correto.

## 3. Passo a Passo da Implementação

### Fase 1: Fundação (Arquitetura) - ✅ CONCLUÍDO
- [x] Criação do enum `PreviewStrategy` em `src-tauri/src/formats/types.rs`.
- [x] Adição do campo `preview_strategy` em `FileFormat`.
- [x] Padronização das estratégias em `src-tauri/src/formats/definitions.rs`.

### Fase 2: Motor de RAW (rsraw) - ✅ CONCLUÍDO
- [x] Implementação de `extract_raw_preview` com fallback robusto para scanner binário.
- [x] Integração com `rsraw` para extração de previews de alta resolução.

### Fase 3: Motor Moderno (HEIC, HDR, EXR) - ✅ CONCLUÍDO
- [x] Habilitadas as features `hdr`, `openexr`, `dds` e `tiff` no crate `image`.
- [x] Implementada conversão nativa HDR -> SDR (Tone Mapping 8-bit RGB) para `hdr` e `exr`.
- [x] Uso de FFmpeg como fallback de alta qualidade para codecs modernos.

### Fase 4: Design e Projetos (PSD, AI, CLIP, XCF) - ✅ CONCLUÍDO
- [x] **PSD/PSB**: Extração via `psd` crate com fallback binário.
- [x] **CLIP/XMind**: Extração exaustiva (case-insensitive) de ZIP.
- [x] **XCF/BLEND**: Scanner binário `memmap2` para localizar previews embutidos.
- [x] **Affinity**: Extrator nativo para arquivos `.afphoto`, `.afdesign` e `.afpub`.

### Fase 5: Especialização EPS (PostScript) - ✅ CONCLUÍDO
- [x] Separação total da lógica de AI e EPS.
- [x] Implementação de `extract_eps_binary_pointer` (Leitura oficial do ponteiro TIFF).
- [x] Implementação de `extract_xmp_thumbnail` (Decodificação Base64 Adobe).
- [x] **Renderização Nativa macOS**: Implementado fallback via `qlmanage` (QuickLook) para EPS vetoriais puros.

### Fase 6: Otimização de Performance e Infra - ✅ CONCLUÍDO
- [x] Migração de scanners binários para `memmap2` (consumo mínimo de RAM).
- [x] Implementação de `scan_mmap_for_tiff` para previews legados.
- [x] Ajuste dinâmico de MIME Types para garantir compatibilidade com o Browser.

## 3. Estratégias por Formatos (Atualizado)

| Formato | Preview Strategy | Técnica Utilizada |
| :--- | :--- | :--- |
| **RAW (all)** | `Raw` | `rsraw` -> Scanner Binário -> FFmpeg. |
| **HEIC/AVIF** | `Ffmpeg` | FFmpeg pipe ou conversão nativa. |
| **HDR/EXR** | `NativeExtractor` | Crate `image` (Native Tone Mapping) -> PNG. |
| **DDS/TGA** | `NativeExtractor` | Crate `image` -> PNG. |
| **PSD/PSB** | `NativeExtractor` | `psd` crate -> Composite Layer. |
| **AI (Illustrator)**| `NativeExtractor` | Extração de Stream PDF (nativo no Mac). |
| **EPS** | `NativeExtractor` | **Priority**: Pointer -> XMP -> QuickLook -> FFmpeg. |
| **CLIP/XMind** | `NativeExtractor` | ZIP Survey (Recursive search) -> PNG. |
| **XCF/Blend** | `NativeExtractor` | Greedy Binary Scan (`memmap2`) -> JPEG/PNG. |

## 4. Notas de Manutenção (Resolução de Problemas)

- **Espaço em Disco**: Realizada limpeza do diretório `target` (liberados 56GB) após erro de falta de espaço no build.
- **Avisos de Código**: Todos os warnings de compilação em `mod.rs` e `binary_jpeg.rs` foram limpos.
- **Dependências de Sistema**: O sistema detecta a ausência de Ghostscript e prioriza o motor nativo do macOS para EPS.

## 5. Próximos Passos e Verificação final

- [x] Validar EPS sem preview embutido (Renderização via QuickLook).
- [x] Validar conversão de TIFF -> PNG para previews de Illustrator.
- [ ] Monitorar uso de memória no scanner `memmap2` em arquivos > 1GB.
- [ ] Implementar sistema de cache para previews convertidos (opcional, para velocidade).

---
*Assinado: Antigravity Assistant.*
