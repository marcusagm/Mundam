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

### Fase 5: Especialização EPS e Illustrator (PDFium) - ✅ CONCLUÍDO
- [x] Separação total da lógica de AI e EPS.
- [x] Implementação de `extract_eps_binary_pointer` (Leitura oficial do ponteiro TIFF).
- [x] Implementação de `extract_xmp_thumbnail` (Decodificação Base64 Adobe).
- [x] **PDFium Integration**: Implementado motor de renderização vetorial cross-platform para AI e EPS baseados em PDF.
- [x] Substituído o fallback instável `qlmanage` por renderização nativa PDFium.

### Fase 6: Otimização de Performance e Infra - ✅ CONCLUÍDO
- [x] Migração de scanners binários para `memmap2` (consumo mínimo de RAM).
- [x] Implementação de `scan_mmap_for_tiff` para previews legados.
- [x] Ajuste dinâmico de MIME Types para garantir compatibilidade com o Browser.

### Fase 7: Distribuição e Bundling de Binários - ✅ CONCLUÍDO
- [x] Criação do script `download-pdfium.sh` para gestão automática de binários por plataforma.
- [x] Configuração de `resources` no `tauri.conf.json` para embutir as shared libraries (`.dylib`, `.so`, `.dll`).
- [x] Propagação do `AppHandle` através de todo o pipeline de thumbnails para carregamento dinâmico de binários em produção.

## 3. Estratégias por Formatos (Atualizado)

| Formato | Preview Strategy | Técnica Utilizada |
| :--- | :--- | :--- |
| **RAW (all)** | `Raw` | `rsraw` -> Scanner Binário -> FFmpeg. |
| **HEIC/AVIF** | `Ffmpeg` | FFmpeg pipe ou conversão nativa. |
| **HDR/EXR** | `NativeExtractor` | Crate `image` (Native Tone Mapping) -> PNG. |
| **DDS/TGA** | `NativeExtractor` | Crate `image` -> PNG. |
| **PSD/PSB** | `NativeExtractor` | `psd` crate -> Composite Layer. |
| **AI (Illustrator)**| `NativeExtractor` | Extração de Stream PDF + **PDFium-render** (Alta Qualidade). |
| **EPS** | `NativeExtractor` | **Priority**: Pointer -> XMP -> PDFium (se compatível) -> FFmpeg (com Timeout). |
| **CLIP/XMind** | `NativeExtractor` | ZIP Survey (Recursive search) -> PNG. |
| **XCF/Blend** | `NativeExtractor` | Greedy Binary Scan (`memmap2`) -> JPEG/PNG. |

## 4. Notas de Manutenção (Resolução de Problemas)

- **QuickLook Removido**: A solução anterior via `qlmanage` foi descartada por causar travamentos e ser limitada ao macOS.
- **Segurança de Execução**: Implementado o crate `wait-timeout`. Todos os comandos externos (FFmpeg) agora possuem um **timeout de 15 segundos**, impedindo que arquivos corrompidos travem a interface da aplicação.
- **PDFium Integration**: Adicionado suporte ao motor do Chrome (PDFium) para garantir renderização vetorial perfeita de arquivos Adobe Illustrator e PDF em qualquer sistema operacional.
- **Espaço em Disco**: Realizada limpeza do diretório `target` (liberados 56GB).

## 5. Próximos Passos e Verificação final
- [x] Validar extração via PDFium em arquivos Illustrator (Sucesso em Logo.ai, Monocromático.ai, etc).
- [x] Garantir que falhas em comandos externos não travem o Worker de thumbnails.
- [x] Documentar e automatizar a necessidade da DLL/Dylib do PDFium para distribuição final (Tauri Bundler).
- [x] Validar fallback FFmpeg para EPS vetoriais legados (Ghostscript).

---
*Assinado: Antigravity Assistant.*
