# Implementação de Suporte a Arquivos GIMP (.xcf) no Mundam

Este documento detalha o processo de refatoração e implementação realizado para fornecer suporte de alta fidelidade a arquivos do GIMP (`.xcf`), incluindo geração de thumbnails e previews com composição multi-camada.

## 📋 Visão Geral

- **Objetivo**: Adicionar suporte a arquivos `.xcf`, garantindo visualizações fiéis ao projeto original, mesmo sem thumbnails embutidas.
- **Data**: 12 de Fevereiro de 2026
- **Status**: Concluído ✅

---

## 🚀 Etapas da Implementação

### 1. Análise Técnica do Formato
Inicialmente, exploramos três níveis de suporte:
- **Nível 1**: Extração de thumbnail embutida (metadado `PROP_25`).
- **Nível 2**: Uso de bibliotecas externas (FFmpeg/ImageMagick).
- **Nível 3**: Decodificação nativa dos blocos RLE do XCF.

**Descoberta**: Muitos arquivos `.xcf` profissionais não possuem a propriedade de thumbnail, e o FFmpeg falha consistentemente na leitura de versões modernas do formato. Decidimos pela **decodificação nativa**.

### 2. Desenvolvimento do Extrator de Alta Fidelidade
Criamos o módulo `src-tauri/src/thumbnails/extractors/xcf.rs` com as seguintes capacidades:
- **Parser Binário Robusto**: Suporte a versões legadas e modernas (v11+ com offsets de 64 bits).
- **Decodificador RLE**: Implementação customizada para descompactar os blocos de pixels (tiles) de 64x64.
- **Composição Multi-Camada**:
    - Identificação de camadas visíveis via `PROP_VISIBLE`.
    - Respeito ao posicionamento global através de `PROP_OFFSETS`.
    - Renderização de fundo para frente (Bottom-to-Top).
- **Alpha Blending (Porter-Duff)**: Fórmula matemática para fusão precisa de pixels transparentes e opacos.

### 3. Integração ao Core do Sistema
- **Registry de Formatos**: Atualizamos o master registry em `definitions.rs` para apontar o `.xcf` como `NativeExtractor`.
- **Pipeline de Thumbnails**: Registramos o módulo em `mod.rs` dentro de `extractors`.
- **Otimização de Logs**: Adicionamos o `.xcf` à lista de exclusão do FFmpeg em `src-tauri/src/thumbnails/mod.rs` para evitar mensagens de erro desnecessárias no terminal e acelerar o processamento.

### 4. Gestão de Dependências
- Adicionada a crate `byteorder = "1.5"` ao `Cargo.toml` para garantir a leitura segura de tipos numéricos BigEndian usados no cabeçalho do XCF.

---

## 📊 Resultados e Performance

Durante os testes com a biblioteca de amostras:
- **Fidelidade**: Imagens complexas como `Space_Captain.xcf` e `Husk.xcf` foram renderizadas com todas as camadas visíveis preservadas.
- **Performance**: Mesmo com a decodificação completa, arquivos médios foram processados em menos de 1 segundo. Arquivos grandes (ex: `Husk.xcf` com 21s) mostram a complexidade da renderização manual, mas garantem a disponibilidade da prévia.
- **Estabilidade**: O fallback para o `NativeExtractor` agora ocorre instantaneamente, sem tentativas frustradas via ferramentas externas.

## 🛠️ Arquivos Modificados
- `src-tauri/src/thumbnails/extractors/xcf.rs` (Novo)
- `src-tauri/src/thumbnails/extractors/mod.rs`
- `src-tauri/src/thumbnails/mod.rs`
- `src-tauri/src/formats/definitions.rs`
- `src-tauri/Cargo.toml`
- `README.md` (Atualização de status do formato)

---

## ⏱️ Benchmarks de Geração: Debug vs. Release

Abaixo está o comparativo de tempo de geração entre o modo funcional (Debug) e o modo otimizado (Release). O ganho de performance é massivo devido às otimizações do Rust em operações de descompressão RLE e Alpha Blending.

| Arquivo | Tempo (Debug) | Tempo (Release) | Ganho (Otimização) |
| :--- | :--- | :--- | :--- |
| `Space_Captain.xcf` | 45.55s | **1.80s** | **25.3x** |
| `Husk.xcf` | 21.31s | **954ms** | **22.3x** |
| `Science_Officer.xcf` | 19.79s | **823ms** | **24.0x** |
| `Pilot.xcf` | 18.10s | **786ms** | **23.0x** |
| `Psion.xcf` | 13.47s | **604ms** | **22.3x** |
| `gimp-splash.xcf` | 8.64s | **239ms** | **36.1x** |
| `staffsaurian.xcf` | 1.83s | **100ms** | **18.3x** |
| `cleaver.xcf` | 1.67s | **213ms** | **7.8x** |
| `Coconut.xcf` | 1.53s | **75ms** | **20.4x** |
| `FBI_walk_cycle.xcf` | 1.45s | **87ms** | **16.6x** |
| `1.xcf` | 1.32s | **38ms** | **34.7x** |
| `chainmail.xcf` | 1.29s | **44ms** | **29.3x** |
| `SlicerOpenAnatomy.xcf` | 1.21s | **33ms** | **36.6x** |
| `maskman.xcf` | 1.15s | **108ms** | **10.6x** |
| `staffsaurian2.xcf` | 949ms | **72ms** | **13.1x** |
| `razors.xcf` | 956ms | **67ms** | **14.2x** |
| `rammask.xcf` | 936ms | **145ms** | **6.4x** |
| `brasslamp.xcf` | 932ms | **47ms** | **19.8x** |
| `scepter.xcf` | 888ms | **39ms** | **22.7x** |
| `tail-merman.xcf` | 724ms | **97ms** | **7.4x** |
| `tribal_m-48x64.xcf` | 664ms | **47ms** | **14.1x** |
| `hoof.xcf` | 650ms | **72ms** | **9.0x** |
| `default_icon.xcf` | 648ms | **71ms** | **9.1x** |
| `Sample-GIMP.xcf` | 564ms | **46ms** | **12.2x** |
| `WPoldBarnstar.xcf` | 542ms | **88ms** | **6.1x** |
| `tribal_elder_m-24x32.xcf` | 219ms | **50ms** | **4.3x** |
| `razors60.xcf` | 345ms | **42ms** | **8.2x** |
| `water.xcf` | 344ms | **34ms** | **10.1x** |
| `professor_hurt_no_hat.xcf` | 330ms | **23ms** | **14.3x** |
| `rammask-small.xcf` | 272ms | **57ms** | **4.7x** |
| `hammer.xcf` | 270ms | **120ms** | **2.2x** |
| `Limestone_wall01.xcf` | 257ms | **46ms** | **5.5x** |

### Conclusão do Benchmark
O modo **Release** transformou o que era um processamento pesado em algo quase instantâneo para a maioria dos arquivos. Arquivos que levavam quase 1 minuto agora são renderizados em **menos de 2 segundos**. 

Isso valida a nossa abordagem de implementar o decodificador RLE e o Compositor de Camadas manualmente em Rust, aproveitando a segurança e o poder de otimização da linguagem para entregar uma experiência de "zero latência" na galeria do Mundam.
