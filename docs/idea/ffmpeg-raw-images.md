# FFmpeg Raw Images and TIFF Processing

FFmpeg can handle raw image and video data. However, because raw files lack a header with essential information, you must explicitly specify the video parameters like resolution, pixel format, and frame rate in your command.

## Raw Image and Video Data

### Key Command Line Parameters

*   **-f rawvideo**: Specifies that the input format is raw video data.
*   **-pixel_format** (or **-pix_fmt**): Defines the color format (e.g., `rgb24`, `rgba`, `gray10le`). You can use `ffmpeg -pix_fmts` to see a full list of supported formats.
*   **-video_size** (or **-s**): Sets the resolution of the image(s), e.g., `1920x1080`.
*   **-framerate** (or **-r**): Sets the frame rate (frames per second) for a sequence of raw images.

### Examples

#### 1. Convert a single raw image file to a standard format (e.g., PNG)
This command reads a raw file with specified parameters and outputs a standard image file.

```bash
ffmpeg -f rawvideo -pixel_format rgba -video_size 320x240 -i input.raw output.png
```

#### 2. Convert a sequence of raw images into a video (e.g., MP4)
This command reads multiple raw image files named in a sequence (e.g., `img001.raw`, `img002.raw`) and compiles them into a video.

```bash
ffmpeg -pixel_format gray10le -video_size 1600x1300 -framerate 30 -i CapturedImage-%03d.raw output.mp4
```
*Note: The `%03d` is a placeholder that matches three-digit sequential numbers in the filenames.*

#### 3. Output a standard image file's raw binary data
You can also use FFmpeg to extract the raw pixel data from an existing image file and output it to a raw file or pipe it to another program.

```bash
ffmpeg -i input.png -f rawvideo output.raw
```

---

## TIFF Image Conversion

FFmpeg converts TIFF images and sequences to various formats or videos using `-codec:video tiff` for encoding.

### Key TIFF Conversion Commands

**Convert Video to TIFF Sequence:**
```bash
ffmpeg -i input.mp4 -codec:video tiff -pixel_format rgb24 frame%04d.tiff
```

**Convert TIFF Sequence to Video:**
```bash
ffmpeg -framerate 24 -i frame%04d.tiff -codec:video libx264 -pixel_format yuv420p output.mp4
```

**TIFF Compression (Reduce Size):**
```bash
ffmpeg -i input.png -codec:video tiff -compression_algo deflate output.tiff
```

**Convert Multipage TIFF to Video:**
```bash
ffmpeg -i multipage.tiff -codec:video libx264 -pixel_format yuv420p output.mp4
```

### Crucial Parameters

*   **-pixel_format rgb24** (or **rgba**): Ensures correct color spaces for TIFF, preventing inverted colors or incompatibility.
*   **-compression_algo**: Options include `raw`, `lzw`, `deflate`.
*   **-dpi**: Set specific Dots Per Inch for output (e.g., `-dpi 300`).

### Common Pitfalls

*   **16-bit Images**: If working with 16-bit TIFFs, ensure the output format supports it (e.g., `-pixel_format gray16le` or `rgb48le`).
*   **Input Sequence**: Use pattern matching like `image%03d.tiff` for sequential files.

For more detailed information and advanced options, consult the official [FFmpeg documentation](https://ffmpeg.org/documentation.html).


---

Para gerar **thumbnails e previews maiores de arquivos RAW** via `ffmpeg`, é fundamental entender uma limitação estrutural:

> **FFmpeg não é um decodificador RAW fotográfico completo.**
> Ele normalmente **não demosaica o sensor RAW real**, mas sim extrai o **preview JPEG embutido** (quando existente).

Isso, porém, é exatamente o que você quer na maioria dos casos: **rápido, eficiente e com qualidade suficiente para preview**.

---

# 1. Estratégia Técnica Recomendada

## ✔ Estratégia ideal (produção)

**Fluxo híbrido e robusto:**

1. **Tentar extrair o preview embutido via ffmpeg**
2. Se falhar → fallback para `libraw` ou `dcraw`
3. Gerar thumbnail e preview escalado via ffmpeg

Isso entrega:

* Performance alta
* Compatibilidade ampla
* Qualidade previsível
* Controle de tamanho

---

# 2. Extraindo Preview Embutido com FFmpeg

### Verificar se existe stream JPEG embutido

```bash
ffmpeg -i input.cr2
```

Normalmente aparecerá algo como:

```
Stream #0:0: Video: mjpeg
```

Se existir MJPEG → é o preview embutido.

---

## Extrair o preview original

```bash
ffmpeg -i input.cr2 -map 0:v:0 -c copy preview.jpg
```

Isso:

* Não reencoda
* É instantâneo
* Mantém máxima qualidade do preview embutido

---

# 3. Gerar Thumbnail Pequeno

```bash
ffmpeg -i input.cr2 \
-map 0:v:0 \
-vf "scale=320:-1" \
-q:v 3 \
thumbnail.jpg
```

Ou mantendo proporção exata:

```bash
-vf "scale=320:320:force_original_aspect_ratio=decrease"
```

---

# 4. Gerar Preview Maior (ex: 1920px)

```bash
ffmpeg -i input.cr2 \
-map 0:v:0 \
-vf "scale=1920:-1:flags=lanczos" \
-q:v 2 \
preview_large.jpg
```

Use `lanczos` para melhor qualidade de upscale/downscale.

---

# 5. Problema Real: CR3

`cr3` é o formato mais problemático.

Dependendo da build do FFmpeg:

* Pode não listar stream MJPEG
* Pode falhar silenciosamente

### Solução profissional

Usar:

```
libraw + rawpy
ou
exiftool -b -PreviewImage
```

Exemplo com exiftool (extremamente confiável):

```bash
exiftool -b -PreviewImage input.cr3 > preview.jpg
```

Esse método suporta praticamente todos os RAW listados.

---

# 6. Suporte por formato

| Formato | Preview embutido via ffmpeg | Observação                |
| ------- | --------------------------- | ------------------------- |
| dng     | ✔                           | Geralmente contém preview |
| cr2     | ✔                           | Estável                   |
| nef     | ✔                           | Estável                   |
| nrw     | ✔                           | Baseado em NEF            |
| rw2     | ✔                           | Panasonic                 |
| raf     | ✔                           | Fuji                      |
| orf     | ✔                           | Olympus                   |
| pef     | ✔                           | Pentax                    |
| erf     | ✔                           | Epson                     |
| sr2     | ✔                           | Sony antigo               |
| srf     | ✔                           | Sony                      |
| cr3     | ⚠ parcial                   | Depende da build          |
| crw     | ✔                           | Canon antigo              |
| arw     | ✔                           | Sony                      |

---

# 7. Melhor Arquitetura (Recomendado para Aplicações)

Se você está construindo um sistema (imagino algo backend ou app desktop):

## Pipeline ideal

```
try ffmpeg preview extraction
    if success → scale versions
    else → fallback libraw
```

Pseudo:

```bash
ffmpeg -i file.raw -map 0:v:0 -c copy preview.jpg || fallback_tool
```

---

# 8. Performance Considerations

Extrair preview embutido:

* ~5–20ms
* Zero demosaicing
* Ideal para galerias

Demosaicar RAW real:

* 300–1500ms por imagem
* CPU heavy
* Só use se precisar do RAW full fidelity

---

# 9. Caso Você Precise do RAW Verdadeiro (Full Decode)

FFmpeg não é a ferramenta ideal.

Use:

* `libraw`
* `rawtherapee-cli`
* `darktable-cli`

Exemplo:

```bash
darktable-cli input.cr2 output.jpg --width 1920
```

---

# 10. Melhor Configuração para Qualidade

Se for reencodar:

```bash
-q:v 2
```

Se quiser controle mais fino:

```bash
-qscale:v 2
```

Ou para PNG (lossless):

```bash
-vf "scale=1920:-1" preview.png
```

---

# 11. Conclusão Direta

Se o objetivo for:

### ✔ Thumbnails rápidos

Use preview embutido via ffmpeg

### ✔ Preview médio/alto para UI

Extraia preview embutido e reescale

### ✔ Fidelidade RAW real

Use libraw/darktable



----

Há uma distinção técnica crítica que precisa ser feita:

Os parâmetros que você listou (`-f rawvideo`, `-pixel_format`, `-video_size`, etc.) são para **raw frame buffers não estruturados** (dados RGB/YCbCr crus, sem metadados).
Arquivos fotográficos como `cr2`, `nef`, `arw`, `dng`, etc. **não são rawvideo** — são **containers TIFF/ISO BMFF proprietários contendo dados Bayer + metadados + previews JPEG**.

Portanto:

> ❗ Você NÃO deve usar `-f rawvideo` para arquivos RAW de câmera.

Esses parâmetros só se aplicam quando você já tem um buffer de pixels puro.

---

# ✅ Parâmetros Ideais para Arquivos RAW Fotográficos no FFmpeg

O comportamento correto depende do que você quer:

---

# CENÁRIO 1 — Extrair Preview Embutido (RECOMENDADO)

Esse é o uso correto para FFmpeg com RAW fotográfico.

### 🔎 Verificar streams

```bash
ffmpeg -hide_banner -i input.cr2
```

Você procura por:

```
Stream #0:0: Video: mjpeg
```

---

## 🎯 Extrair preview sem reencodar (máxima qualidade, máxima performance)

```bash
ffmpeg -i input.cr2 -map 0:v:0 -c copy preview.jpg
```

Parâmetros importantes:

* `-map 0:v:0` → seleciona o primeiro stream de vídeo (preview JPEG)
* `-c copy` → evita reencode

Isso é o ideal para thumbnails e previews rápidos.

---

# CENÁRIO 2 — Gerar Thumbnail Redimensionado

```bash
ffmpeg -i input.cr2 \
-map 0:v:0 \
-vf "scale=320:-1:flags=lanczos" \
-q:v 3 \
thumbnail.jpg
```

Parâmetros relevantes:

| Parâmetro       | Função            |
| --------------- | ----------------- |
| `-map 0:v:0`    | Seleciona preview |
| `-vf scale=...` | Redimensionamento |
| `flags=lanczos` | Melhor algoritmo  |
| `-q:v 2-4`      | Qualidade JPEG    |

---

# CENÁRIO 3 — Gerar Preview Maior (ex: 2048px)

```bash
ffmpeg -i input.nef \
-map 0:v:0 \
-vf "scale=2048:-1:flags=lanczos" \
-q:v 2 \
preview_large.jpg
```

---

# ⚠ Quando NÃO Existe Preview MJPEG

Alguns CR3 ou DNG podem não expor stream via FFmpeg.

Nesse caso:

```bash
ffmpeg -i input.cr3
```

Se não listar MJPEG → FFmpeg não consegue acessar preview.

Use fallback:

```bash
exiftool -b -PreviewImage input.cr3 > preview.jpg
```

---

# E SE EU QUISER DECODIFICAR O SENSOR RAW REAL?

FFmpeg **não é ferramenta adequada** para demosaicing Bayer com qualidade fotográfica.

Mesmo que compile com `libraw`, o controle é limitado.

Ferramentas corretas:

* `libraw`
* `dcraw`
* `darktable-cli`
* `rawtherapee-cli`

Exemplo:

```bash
darktable-cli input.cr2 output.jpg --width 2048
```

---

# 🔬 Parâmetros Técnicos Relevantes em RAW Fotográfico

Quando FFmpeg consegue ler o preview, você pode controlar:

## Forçar pixel format de saída

```bash
-pix_fmt yuvj420p
```

ou

```bash
-pix_fmt rgb24
```

Mas normalmente não precisa especificar.

---

## Controlar corespace

Alguns RAW usam AdobeRGB.

```bash
-color_primaries bt709 \
-colorspace bt709 \
-color_trc bt709
```

Só use se houver necessidade de normalização.

---

## Controlar compressão JPEG

```bash
-q:v 2
```

Intervalo:

* 1–2 → qualidade alta
* 3–5 → bom equilíbrio
* > 6 → compressão agressiva

---

# 🚫 O que NÃO Usar para RAW Fotográfico

| Parâmetro                 | Motivo                                 |
| ------------------------- | -------------------------------------- |
| `-f rawvideo`             | RAW fotográfico não é rawvideo         |
| `-video_size`             | Resolução já está no container         |
| `-framerate`              | Não é sequência de frames              |
| `-pixel_format` (entrada) | Bayer não é interpretado como rawvideo |

---

# 🧠 Diferença Estrutural Importante

### RAW fotográfico

* Estruturado
* Contém EXIF
* Pode conter múltiplos previews
* Pode conter JPEG fullsize
* Pode conter dados Bayer

### rawvideo

* Fluxo binário puro
* Sem header
* Sem metadados
* Você deve informar resolução e pixel format

---

# 🎯 Melhor Conjunto de Parâmetros para Seu Caso (Galeria / DAM / Sistema)

Recomendação prática:

## Thumbnail (320px)

```bash
ffmpeg -i input.raw \
-map 0:v:0 \
-vf "scale=320:-1:flags=lanczos" \
-q:v 3 \
thumbnail.jpg
```

## Preview Médio (1600–2048px)

```bash
ffmpeg -i input.raw \
-map 0:v:0 \
-vf "scale=2048:-1:flags=lanczos" \
-q:v 2 \
preview.jpg
```

## Ultra rápido (sem resize)

```bash
ffmpeg -i input.raw -map 0:v:0 -c copy preview.jpg
```

---

# 🏁 Conclusão Técnica

Se estiver trabalhando com:

* DAM
* Galeria web
* Desktop app
* Pipeline backend

O melhor approach é:

1. Extrair preview embutido
2. Redimensionar
3. Fallback para libraw se necessário

---
