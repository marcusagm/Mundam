<!-- omit in toc -->
# Assets - Supported formats

<!-- omit in toc -->
## Index

- [Summary of supported formats](#summary-of-supported-formats)
- [Images](#images)
  - [Raster formats](#raster-formats)
  - [Icon](#icon)
  - [Vector formats](#vector-formats)
  - [Raw camera formats](#raw-camera-formats)
- [Project](#project)
  - [Design applications](#design-applications)
  - [Mind Maps, flowcharts and sketch-likes](#mind-maps-flowcharts-and-sketch-likes)
- [3D Models](#3d-models)
  - [Applications with Native 3D Capabilities](#applications-with-native-3d-capabilities)
  - [3D Formats](#3d-formats)
- [Font](#font)
- [Audio Formats](#audio-formats)
- [Video Formats](#video-formats)
- [Documents](#documents)
- [Code Formats](#code-formats)
- [Archives](#archives)

### Status and dependency legend

- 🟢 Working; 
- 🟠 Supported but incomplete or known to have problems; 
- 🔴 unsupported; 
- ⚠️ support is registered but still needs testing.
- `External dependencies` names required runtime tools, shared libraries, or third-party crates/assets used by the implementation. `Native` means no separate runtime tool is required beyond the app and its packaged code. `FFmpeg` and `FFprobe` are resolved from app resources, the development directory, or the system `PATH`.

## Summary of supported formats

| Type      | Fully supported | Partially supported | Unsupported | Needs testing |   Total |
| --------- | --------------: | ------------------: | ----------: | ------------: | ------: |
| Images    |              38 |                   2 |           3 |             2 |      45 |
| Project   |              15 |                   1 |          21 |             0 |      37 |
| 3D Models |               8 |                   4 |          10 |             0 |      22 |
| Font      |               3 |                   0 |           4 |             0 |       7 |
| Audio     |               4 |                  16 |           0 |             2 |      22 |
| Video     |              16 |                   1 |           0 |             7 |      24 |
| Documents |               4 |                   0 |           5 |             0 |       9 |
| Code      |               0 |                   0 |          18 |             0 |      18 |
| Archives  |               0 |                   2 |           7 |             0 |       9 |
| **Total** |          **88** |              **26** |      **68** |        **11** | **193** |

Each table row in the format sections counts as one format; multiple extensions listed on the same row count once. A format is fully supported when all its listed capabilities are 🟢, unsupported when all are 🔴, and needs testing when all are ⚠️. Rows with mixed statuses are counted as partially supported.

## Images

### Raster formats

| Format                   | Extensions              | Thumbnail | Preview | Metadata | External dependencies | Notes                                                       |
| ------------------------ | ----------------------- | --------- | ------- | -------- | --------------------- | ----------------------------------------------------------- |
| Bitmap                   | bmp                     | 🟢         | 🟢       | 🟢        | Native                |                                                             |
| Animated PNG             | apng                    | 🔴         | 🔴       | 🔴        | None                  | No `.apng` provider is registered.                          |
| Portable Network Graphic | png                     | 🟢         | 🟢       | 🟢        | Native                |                                                             |
| Graphics Interchange     | gif                     | 🟢         | 🟢       | 🟢        | Native                | Animated frames are extracted by the GIF provider.          |
| Joint Photographic       | jpg, jpeg, jpe, jfif    | 🟢         | 🟢       | 🟢        | Native                | These are the registered extensions.                        |
| Tagged Image             | tiff, tif               | 🟢         | 🟢       | 🟢        | Native                |                                                             |
| Truevision               | tga                     | 🟢         | 🟢       | 🟢        | Native                | `.targa` is not a registered extension.                     |
| High Efficiency          | heic, heif              | 🟢         | 🟠       | 🟢        | FFmpeg                | Preview has reported intermittent problems.                 |
| High Efficiency Sequence | heifs                   | ⚠️         | ⚠️       | ⚠️        | FFmpeg                | Registered, but sequence-file behavior still needs testing. |
| WebP                     | webp                    | 🟢         | 🟢       | 🟢        | FFmpeg                | FFmpeg is used by the preview implementation.               |
| JPEG XL                  | jxl                     | 🟢         | 🟢       | 🟢        | jxl-oxide, FFmpeg     | Rust decoder first, with FFmpeg fallback.                   |
| JPEG 2000                | jp2, j2c                | 🔴         | 🔴       | 🔴        | None                  | No JPEG 2000 provider is registered.                        |
| OpenEXR                  | exr                     | 🟢         | 🟢       | 🟢        | Native                |                                                             |
| AV1 Image Format         | avif, avifs             | 🟢         | 🟢       | 🟢        | FFmpeg                |                                                             |
| High Dynamic Range       | hdr                     | 🟢         | 🟢       | 🟢        | Native                |                                                             |
| Portable Any Map         | pnm, ppm, pgm, pbm, pam | 🟢         | 🟢       | 🟢        | Native                |                                                             |

### Icon

| Format          | Extensions | Thumbnail | Preview | Metadata | External dependencies | Notes |
| --------------- | ---------- | --------- | ------- | -------- | --------------------- | ----- |
| Icon File       | ico        | 🟢         | 🟢       | 🟢        | Native                |       |
| Cur File        | cur        | 🟢         | 🟢       | 🟢        | Native                |       |
| Apple Icon File | icns       | 🟢         | 🟢       | 🟢        | Native                |       |

### Vector formats

| Format                   | Extensions | Thumbnail | Preview | Metadata | External dependencies          | Notes                                                                        |
| ------------------------ | ---------- | --------- | ------- | -------- | ------------------------------ | ---------------------------------------------------------------------------- |
| Scalable Vector Graphics | svg, svgz  | 🟢         | 🟢       | 🟢        | Native (resvg)                 | SVGZ is decompressed with gzip support.                                      |
| Encapsulated PostScript  | eps        | 🟢         | 🟢       | 🟢        | PDFium, pstopdf or Ghostscript | Conversion is attempted as a fallback when no embedded preview is available. |
| Portable Document Format | pdf        | 🟢         | 🟢       | 🟢        | PDFium                         | Uses the PDFium shared library.                                              |
| PostScript               | ps         | 🟢         | 🟢       | 🟢        | PDFium, pstopdf or Ghostscript | Conversion is attempted as a fallback when no embedded preview is available. |

### Raw camera formats

The RAW pipeline uses FlashRaw for metadata, embedded previews, and sensor decoding. The statuses below do not guarantee support for every camera model or compression mode: FlashRaw tracks support per format mode, and its upstream corpus does not cover every file variant. Known exceptions include Nikon High Efficiency NEF sensor data, Kodak DC120 JPEG RAW, and JPEG XL-compressed DNG (which requires FlashRaw's optional `jpegxl` feature). Embedded previews may still be available when sensor data cannot be decoded.

| Format                   | Extensions    | Thumbnail | Preview | Metadata | External dependencies                  | Notes                                                                                                                                                                                           |
| ------------------------ | ------------- | --------- | ------- | -------- | -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Hasselblad Raw           | 3fr, fff      | 🟢         | 🟢       | 🟢        | FlashRaw                               |                                                                                                                                                                                                 |
| Phase One Raw            | iiq           | 🟢         | 🟢       | 🟢        | FlashRaw                               |                                                                                                                                                                                                 |
| Sony Raw Format          | arw, sr2, srf | 🟢         | 🟢       | 🟢        | FlashRaw                               |                                                                                                                                                                                                 |
| Samsung Raw              | srw           | 🟢         | 🟢       | 🟢        | FlashRaw                               |                                                                                                                                                                                                 |
| Adobe Digital Negative   | dng           | 🟢         | 🟢       | 🟢        | FlashRaw                               | JPEG XL-compressed DNG requires FlashRaw's optional `jpegxl` feature.                                                                                                                           |
| Canon Raw Format         | cr2, cr3, crw | 🟢         | 🟢       | 🟢        | FlashRaw                               | One malformed/non-RAW CRW corpus entry is rejected.                                                                                                                                              |
| DirectDraw Surface       | dds           | 🟢         | 🟢       | 🟢        | Native                                 |                                                                                                                                                                                                 |
| DJI RAW                  | dcr           | 🔴         | 🔴       | 🔴        | None                                   | Removed from the v2 registry by obsolescence.                                                                                                                                                   |
| Epson Raw Format         | erf           | 🟢         | 🟢       | 🟢        | FlashRaw                               |
| GoPro RAW Format         | gpr           | 🟢         | 🟢       | 🟢        | FlashRaw (Rust, MIT OR Apache-2.0)     | VC-5 sensor data is decoded and developed in Rust. Dimensions and normalized camera metadata come from FlashRaw; `rexif` adds available EXIF fields; semantic metadata is not implemented. |
| Kodak Digital            | kdc           | 🟢         | 🟢       | 🟢        | FlashRaw, sips (macOS)                 | Kodak DC120 JPEG RAW remains an exception; macOS `sips` handles the legacy sample where available.                                                                                              |
| Mamiya Electronic Format | mef           | 🟢         | 🟢       | 🟢        | FlashRaw                               |
| Leaf Camera RAW          | mos           | 🟢         | 🟢       | 🟢        | FlashRaw                               | Some metadata fields can be reported as unknown.                                                                                                                                                |
| Minolta Raw              | mrw           | 🟢         | 🟢       | 🟢        | FlashRaw                               |
| Nikon Electronic         | nef, nrw      | 🟢         | 🟢       | 🟢        | FlashRaw                               | Nikon High Efficiency NEF sensor data is not decoded; embedded preview/metadata may still be available.                                                                                         |
| Olympus Raw Format       | orf           | 🟢         | 🟢       | 🟢        | FlashRaw                               |
| Panasonic Raw            | rw2           | 🟢         | 🟢       | 🟢        | FlashRaw                               |
| Pentax Raw               | pef           | 🟢         | 🟢       | 🟢        | FlashRaw                               |
| Fujifilm RAW             | raf           | 🟢         | 🟢       | 🟢        | FlashRaw                               |
| Generic RAW              | raw           | 🔴         | 🔴       | 🟢        | FlashRaw                               | Generic `.raw` has metadata but no generated thumbnail or preview.                                                                                                                              |
| Leica Raw                | rwl           | ⚠️         | ⚠️       | ⚠️        | FlashRaw                               | Registered Leica extension; still needs dedicated manual testing.                                                                                                                               |
| Sigma RAW                | x3f           | 🟢         | 🟢       | 🟢        | FlashRaw                               |

## Project

### Design applications

| Format              | Extensions               | Thumbnail | Preview | Metadata | External dependencies          | Notes                                                                   |
| ------------------- | ------------------------ | --------- | ------- | -------- | ------------------------------ | ----------------------------------------------------------------------- |
| Adobe Photoshop     | psd, psb                 | 🟢         | 🟢       | 🟢        | psd crate                      |                                                                         |
| Adobe Illustrator   | ai                       | 🟢         | 🟢       | 🟢        | PDFium, Ghostscript (fallback) | Handles AI files containing PDF/PostScript data.                        |
| Adobe After Effects | aep                      | 🔴         | 🔴       | 🔴        | None                           | No provider is registered.                                              |
| Adobe Premiere      | prproj                   | 🔴         | 🔴       | 🔴        | None                           | No provider is registered.                                              |
| Adobe Animate       | fla                      | 🔴         | 🔴       | 🔴        | None                           | No provider is registered.                                              |
| Adobe Audition      | au                       | 🔴         | 🔴       | 🔴        | None                           | No project provider is registered for this extension.                   |
| Affinity            | afdesign, afphoto, afpub | 🟢         | 🟢       | 🟢        | zip crate                      | `.af` is not among the registered extensions.                           |
| Aseprite            | ase, aseprite            | 🟢         | 🟢       | 🟢        | asefile crate                  |                                                                         |
| Clip Studio Paint   | clip                     | 🟢         | 🟢       | 🟢        | SQLite                         | Reads the embedded Canvas table for dimensions and resolution.          |
| Corel Painter       | rif, riff                | 🟢         | 🟢       | 🟢        | Native                         |                                                                         |
| CorelDRAW           | cdr                      | 🟢         | 🟢       | 🟢        | Native                         |                                                                         |
| Figma               | fig                      | 🟢         | 🟢       | 🟢        | zip crate                      | Comment extraction may be improved.                                     |
| FireAlpaca          | alp                      | 🔴         | 🔴       | 🔴        | None                           | No provider is registered.                                              |
| GIMP                | xcf                      | 🟢         | 🟠       | 🟢        | Native                         | Complex layer blending modes remain incomplete.                         |
| Krita               | kra                      | 🟢         | 🟢       | 🟢        | zip crate                      | `.krz` and `.krita` are not registered aliases.                         |
| MediBang            | mdp                      | 🟢         | 🟢       | 🟢        | zlib                           | `.medibang` is not a registered alias.                                  |
| Paint Tool SAI      | sai, sai2                | 🟢         | 🟢       | 🟢        | Native                         |                                                                         |
| Penpot              | penpot                   | 🟢         | 🟢       | 🟢        | zstd                           |                                                                         |
| Rebelle             | reb                      | 🟢         | 🟢       | 🟢        | zip crate                      |                                                                         |
| Sketch              | sketch                   | 🟢         | 🟢       | 🟢        | zip crate                      |                                                                         |
| Sketchbook          | tiff                     | 🔴         | 🔴       | 🔴        | None                           | TIFF is supported as an image; there is no Sketchbook project provider. |
| Piskel              | piskel                   | 🔴         | 🔴       | 🔴        | None                           | No provider is registered.                                              |
| Procreate           | pro                      | 🔴         | 🔴       | 🔴        | None                           | No provider is registered.                                              |
| ARRIRAW             | ari                      | 🔴         | 🔴       | 🔴        | None                           | No provider is registered.                                              |
| Blackmagic RAW      | braw                     | 🔴         | 🔴       | 🔴        | None                           | No provider is registered.                                              |
| DaVinci Resolve     | drp                      | 🔴         | 🔴       | 🔴        | None                           | No provider is registered.                                              |
| Final Cut Pro       | fcpxml                   | 🔴         | 🔴       | 🔴        | None                           | No provider is registered.                                              |
| Adobe InDesign      | idml, indd               | 🔴         | 🔴       | 🔴        | None                           | No provider is registered.                                              |
| RED Digital Cinema  | r3d                      | 🔴         | 🔴       | 🔴        | None                           | No provider is registered.                                              |

### Mind Maps, flowcharts and sketch-likes

| Format     | Extensions                  | Thumbnail | Preview | Metadata | External dependencies | Notes                      |
| ---------- | --------------------------- | --------- | ------- | -------- | --------------------- | -------------------------- |
| XMind      | xmind                       | 🟢         | 🟢       | 🟢        | zip crate             |                            |
| Excalidraw | excalidraw, excalidraw.json | 🔴         | 🔴       | 🔴        | None                  | No provider is registered. |
| tldraw     | tldraw, tldraw.json         | 🔴         | 🔴       | 🔴        | None                  | No provider is registered. |
| draw.io    | drawio, drawio.xml          | 🔴         | 🔴       | 🔴        | None                  | No provider is registered. |
| Miro       | miro, miro.json             | 🔴         | 🔴       | 🔴        | None                  | No provider is registered. |
| FigJam     | figjam, figjam.json         | 🔴         | 🔴       | 🔴        | None                  | No provider is registered. |
| Mural      | mural, mural.json           | 🔴         | 🔴       | 🔴        | None                  | No provider is registered. |
| Eraser     | eraser.io, eraser.io.json   | 🔴         | 🔴       | 🔴        | None                  | No provider is registered. |

## 3D Models

### Applications with Native 3D Capabilities

| Format        | Extensions | Thumbnail | Preview | Metadata | External dependencies | Notes                                              |
| ------------- | ---------- | --------- | ------- | -------- | --------------------- | -------------------------------------------------- |
| Blender       | blend      | 🟢         | 🟢       | 🟢        | Native                | Preview is extracted from the embedded REND block. |
| Autodesk Maya | ma, mb     | 🔴         | 🔴       | 🔴        | None                  | No provider is registered.                         |
| Cinema 4D     | c4d        | 🔴         | 🔴       | 🔴        | None                  | No provider is registered.                         |
| Modo          | modo       | 🔴         | 🔴       | 🔴        | None                  | No provider is registered.                         |
| Houdini       | hip        | 🔴         | 🔴       | 🔴        | None                  | No provider is registered.                         |
| ZBrush        | ztl, zpr   | 🔴         | 🔴       | 🔴        | None                  | No provider is registered.                         |
| SketchUp      | skp        | 🔴         | 🔴       | 🔴        | None                  | No provider is registered.                         |
| 3ds Max       | max        | 🔴         | 🔴       | 🔴        | None                  | No provider is registered.                         |
| Rhino 3D      | 3dm        | 🔴         | 🔴       | 🔴        | None                  | No provider is registered.                         |

### 3D Formats

| Format       | Extensions            | Thumbnail | Preview | Metadata | External dependencies | Notes                                                                                                             |
| ------------ | --------------------- | --------- | ------- | -------- | --------------------- | ----------------------------------------------------------------------------------------------------------------- |
| OBJ          | obj                   | 🟢         | 🟢       | 🟢        | Assimp                | Converted to GLB for viewing.                                                                                     |
| STL          | stl                   | 🟢         | 🟢       | 🟢        | Assimp                | Converted to GLB for viewing.                                                                                     |
| FBX          | fbx                   | 🟢         | 🟢       | 🟢        | Assimp                | Converted to GLB for viewing.                                                                                     |
| glTF         | gltf, glb             | 🟢         | 🟢       | 🟢        | Assimp                |                                                                                                                   |
| USD          | usd, usda, usdc, usdz | 🔴         | 🔴       | 🟠        | None                  | Registered, but thumbnail/preview strategies are `None` and metadata extraction returns empty objects.            |
| Alembic      | abc                   | 🔴         | 🔴       | 🔴        | None                  | No provider is registered.                                                                                        |
| STEP         | step, stp             | 🔴         | 🔴       | 🟠        | None                  | Registered for identification only; no thumbnail, preview, or meaningful metadata extraction yet.                 |
| IGES         | iges, igs             | 🔴         | 🔴       | 🟠        | None                  | Registered for identification only; no thumbnail, preview, or meaningful metadata extraction yet.                 |
| COLLADA      | dae                   | 🟢         | 🟢       | 🟢        | Assimp                | Converted to GLB for viewing.                                                                                     |
| 3MF          | 3mf                   | 🟢         | 🟢       | 🟢        | Assimp                | Converted to GLB for viewing.                                                                                     |
| PLY          | ply                   | 🔴         | 🔴       | 🔴        | None                  | No provider is registered.                                                                                        |
| AutoCAD      | dwg, dxf              | 🔴         | ⚠️       | 🟠        | Assimp                | Preview strategy is registered, but extraction depends on Assimp support; metadata extraction is currently empty. |
| LightWave 3D | lwo, lws              | 🟢         | 🟢       | 🟢        | Assimp                | Converted to GLB for viewing.                                                                                     |

## Font

| Format               | Extensions     | Thumbnail | Preview | Metadata | External dependencies | Notes                                       |
| -------------------- | -------------- | --------- | ------- | -------- | --------------------- | ------------------------------------------- |
| TrueType Font        | ttf, ttc       | 🟢         | 🟢       | 🟢        | ttf-parser, resvg     |                                             |
| OpenType Font        | otf            | 🟢         | 🟢       | 🟢        | ttf-parser, resvg     | `otc` is not a registered extension.        |
| Web Open Font Format | woff, woff2    | 🟢         | 🟢       | 🟢        | ttf-parser, resvg     |                                             |
| Variable Font        | vttf, vf, fvar | 🔴         | 🔴       | 🔴        | None                  | Not registered.                             |
| Color Font           | color, COLR    | 🔴         | 🔴       | 🔴        | None                  | Not registered as a standalone file format. |
| FontForge Font       | sf, sfd        | 🔴         | 🔴       | 🔴        | None                  | Not registered.                             |
| Embedded OpenType    | eot            | 🔴         | 🔴       | 🔴        | None                  | Not registered.                             |

## Audio Formats

| Format                        | Extensions      | Thumbnail | Playback | Metadata | Waveform extraction | Thumbnail strategy | Playback strategy | External dependencies                         | Notes                                                                                                                                       |
| ----------------------------- | --------------- | --------- | -------- | -------- | ------------------- | ------------------ | ----------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Audible Audio                 | aax             | 🟢         | ⚠️        | ⚠️        | ⚠️                   | Icon               | AudioHls          | FFmpeg, FFprobe                               | Playback, metadata, and waveform extraction are implemented, but this extension was not manually tested; protected files may not transcode. |
| Dolby Digital Audio           | ac3             | 🟢         | 🟢        | 🟢        | 🟠                   | Icon               | AudioHls          | FFmpeg, FFprobe                               | Manual testing found waveform extraction/display failures for some files, especially very small or very large ones.                         |
| Audio Interchange File Format | aiff, aif, aifc | 🟢         | 🟢        | 🟢        | 🟠                   | Icon               | AudioHls          | FFmpeg, FFprobe                               | Manual testing found waveform extraction/display failures for some files, especially very small or very large ones.                         |
| Adaptive Multi-Rate           | amr             | 🟢         | 🟢        | 🟢        | 🟢                   | Icon               | AudioHls          | FFmpeg, FFprobe                               |                                                                                                                                             |
| Monkey's Audio                | ape             | 🟢         | 🟢        | 🟢        | 🟢                   | Icon               | AudioHls          | FFmpeg, FFprobe                               |                                                                                                                                             |
| Waveform Audio File Format    | wav             | 🟢         | 🟢        | 🟢        | 🟠                   | Icon               | Native            | FFmpeg, FFprobe                               | Manual testing found waveform extraction/display failures for some files, especially very small or very large ones.                         |
| Broadcast Wave Format         | bwf             | 🟢         | ⚠️        | ⚠️        | ⚠️                   | Icon               | Native            | FFmpeg, FFprobe                               | Registered as a WAV-provider alias but was not manually tested.                                                                             |
| Core Audio Format             | caf             | 🟢         | 🟢        | 🟢        | 🟠                   | Icon               | AudioHls          | FFmpeg, FFprobe                               | Manual testing found waveform extraction/display failures for some files, especially very small or very large ones.                         |
| Digital Theater Systems       | dts             | 🟢         | 🟢        | 🟢        | 🟠                   | Icon               | AudioHls          | FFmpeg, FFprobe                               | Manual testing found waveform extraction/display failures for some files, especially very small or very large ones.                         |
| Free Lossless Audio Codec     | flac            | 🟢         | 🟢        | 🟢        | 🟠                   | Icon               | Native            | FFmpeg, FFprobe                               | Manual testing found waveform extraction/display failures for some files, especially very small or very large ones.                         |
| MPEG-4 Audio                  | m4a, m4r, aac   | 🟢         | 🟢        | 🟢        | 🟠                   | Icon               | AudioLinearHls    | FFmpeg, FFprobe                               | Tested m4a playback supports ALAC and AAC through HLS; waveform misses are reported for some AAC files.                                     |
| MPEG-4 Audio Book             | m4b             | ⚠️         | ⚠️        | ⚠️        | ⚠️                   | Icon               | AudioLinearHls    | FFmpeg, FFprobe                               | Registered by the MPEG-4 audio provider but not manually tested.                                                                            |
| Musical Instrument Digital    | mid, midi       | 🟢         | 🟢        | 🟢        | 🟢                   | Icon               | AudioLinearHls    | FFmpeg, FFprobe, rustysynth, SoundFont (.sf2) | MIDI is synthesized to WAV before waveform extraction. A SoundFont is required at runtime; waveform extraction was manually verified.       |
| Matroska Audio                | mka             | 🟢         | 🟢        | 🟢        | 🟢                   | Icon               | AudioHls          | FFmpeg, FFprobe                               |                                                                                                                                             |
| MPEG Audio Layer II / III     | mp2, mp3        | 🟢         | 🟢        | 🟢        | 🟠                   | Icon               | Native            | FFmpeg, FFprobe                               | Manual testing found waveform extraction/display failures for some files, especially very small or very large ones.                         |
| Musepack Audio                | mpc             | ⚠️         | ⚠️        | ⚠️        | ⚠️                   | Icon               | AudioHls          | FFmpeg, FFprobe                               | Registered; still needs a manual check.                                                                                                     |
| Ogg Audio                     | ogg, oga        | 🟢         | 🟢        | 🟢        | 🟠                   | Icon               | AudioHls          | FFmpeg, FFprobe                               | Manual testing found waveform extraction/display failures for some files, especially very small or very large ones.                         |
| Opus                          | opus            | 🟢         | 🟢        | 🟢        | 🟠                   | Icon               | AudioHls          | FFmpeg, FFprobe                               | Manual testing found waveform extraction/display failures for some files, especially very small or very large ones.                         |
| RealAudio                     | ra              | 🟢         | 🟢        | 🟢        | 🟠                   | Icon               | AudioHls          | FFmpeg, FFprobe                               | Manual testing found waveform extraction/display failures for some files, especially very small or very large ones.                         |
| Speex                         | spx             | 🟢         | 🟢        | 🟢        | 🟠                   | Icon               | AudioHls          | FFmpeg, FFprobe                               | Manual testing found waveform extraction/display failures for some files, especially very small or very large ones.                         |
| Windows Media Audio           | wma             | 🟢         | 🟢        | 🟢        | 🟠                   | Icon               | AudioHls          | FFmpeg, FFprobe                               | Manual testing found waveform extraction/display failures for some files, especially very small or very large ones.                         |
| WavPack                       | wv              | 🟢         | 🟢        | 🟢        | 🟠                   | Icon               | AudioHls          | FFmpeg, FFprobe                               | Manual testing found waveform extraction/display failures for some files, especially very small or very large ones.                         |

Audio providers use a generic file icon rather than generating audio thumbnails. Audio metadata is extracted with FFprobe. Waveforms are extracted separately from playback via FFmpeg; MIDI is first synthesized to WAV. 🟠 indicates the report's known extraction/display misses for some files (particularly very small or very large ones), while ⚠️ means the format's waveform behavior has not been manually tested. Caching waveform data remains pending.

## Video Formats

| Format                | Extensions         | Thumbnail | Playback | Metadata | Thumbnail strategy | Playback strategy | External dependencies | Notes                                                                                  |
| --------------------- | ------------------ | --------- | -------- | -------- | ------------------ | ----------------- | --------------------- | -------------------------------------------------------------------------------------- |
| 3GPP                  | 3gp, 3g2           | 🟢         | 🟢        | 🟢        | Ffmpeg             | Hls               | FFmpeg, FFprobe       |                                                                                        |
| AVI                   | avi                | 🟢         | 🟢        | 🟢        | Ffmpeg             | Hls               | FFmpeg, FFprobe       |                                                                                        |
| DivX                  | divx               | ⚠️         | ⚠️        | ⚠️        | Ffmpeg             | Hls               | FFmpeg, FFprobe       | Registered as an AVI-provider alias; still needs manual testing.                       |
| ASF / Windows Media   | asf, wmv           | 🟢         | 🟢        | 🟢        | Ffmpeg             | Hls               | FFmpeg, FFprobe       |                                                                                        |
| Flash Video           | flv, f4v           | 🟢         | 🟢        | 🟢        | Ffmpeg             | Hls               | FFmpeg, FFprobe       |                                                                                        |
| H.264 (raw)           | h264               | ⚠️         | ⚠️        | ⚠️        | Ffmpeg             | LinearHls         | FFmpeg, FFprobe       | Registered; manual testing is still needed.                                            |
| HEVC                  | hevc               | 🟢         | 🟠        | 🟢        | Ffmpeg             | LinearHls         | FFmpeg, FFprobe       | Local M3U8 playback has reported decoder instability.                                  |
| H.265                 | h265               | ⚠️         | ⚠️        | ⚠️        | Ffmpeg             | LinearHls         | FFmpeg, FFprobe       | Registered as an HEVC-provider alias; still needs manual testing.                      |
| MPEG transport stream | vob, ts, mts, m2ts | 🟢         | 🟢        | 🟢        | Ffmpeg             | Hls               | FFmpeg, FFprobe       |                                                                                        |
| MPEG video            | mpg, mpeg, m2v     | 🟢         | 🟢        | 🟢        | Ffmpeg             | LinearHls         | FFmpeg, FFprobe       |                                                                                        |
| MPEG-4 video          | mp4, m4v           | 🟢         | 🟢        | 🟢        | Ffmpeg             | Native            | FFmpeg, FFprobe       | Browser-native playback; FFmpeg is still used for thumbnails and FFprobe for metadata. |
| Matroska video        | mkv                | 🟢         | 🟢        | 🟢        | Ffmpeg             | Hls               | FFmpeg, FFprobe       |                                                                                        |
| Motion JPEG           | mjpeg              | 🟢         | 🟢        | 🟢        | Ffmpeg             | LinearHls         | FFmpeg, FFprobe       |                                                                                        |
| Motion JPEG           | mjpg               | ⚠️         | ⚠️        | ⚠️        | Ffmpeg             | LinearHls         | FFmpeg, FFprobe       | Alias registered; still needs a manual check.                                          |
| MXF                   | mxf                | 🟢         | 🟢        | 🟢        | Ffmpeg             | Hls               | FFmpeg, FFprobe       |                                                                                        |
| Ogg video             | ogv                | 🟢         | 🟢        | 🟢        | Ffmpeg             | Hls               | FFmpeg, FFprobe       |                                                                                        |
| QuickTime             | mov                | 🟢         | 🟢        | 🟢        | Ffmpeg             | Native            | FFmpeg, FFprobe       | Browser-native playback.                                                               |
| QuickTime             | qt                 | ⚠️         | ⚠️        | ⚠️        | Ffmpeg             | Native            | FFmpeg, FFprobe       | Registered alias; browser-native playback, but still needs manual testing.             |
| RealMedia             | rm                 | 🟢         | 🟢        | 🟢        | Ffmpeg             | Hls               | FFmpeg, FFprobe       |                                                                                        |
| RealMedia             | rmvb               | ⚠️         | ⚠️        | ⚠️        | Ffmpeg             | Hls               | FFmpeg, FFprobe       | Registered alias; still needs manual testing.                                          |
| Shockwave Flash       | swf                | 🟢         | 🟢        | 🟢        | Ffmpeg             | LinearHls         | FFmpeg, FFprobe       |                                                                                        |
| WebM                  | webm               | 🟢         | 🟢        | 🟢        | Ffmpeg             | Native            | FFmpeg, FFprobe       | Browser-native playback; FFmpeg is still used for thumbnails and FFprobe for metadata. |
| Windows Recorded TV   | wtv                | 🟢         | 🟢        | 🟢        | Ffmpeg             | Hls               | FFmpeg, FFprobe       |                                                                                        |
| YUV4MPEG2             | y4m                | ⚠️         | ⚠️        | ⚠️        | Ffmpeg             | LinearHls         | FFmpeg, FFprobe       | Registered; manual testing is still needed.                                            |

All video providers use FFmpeg for thumbnails and FFprobe for technical metadata. `Native` means browser playback without an HLS conversion; `Hls` and `LinearHls` are the registered server-streaming strategies, not interchangeable labels. The HLS server requires FFmpeg at runtime.

## Documents

| Format                   | Extensions                       | Thumbnail | Preview | Metadata | External dependencies | Notes                                        |
| ------------------------ | -------------------------------- | --------- | ------- | -------- | --------------------- | -------------------------------------------- |
| Plain Text               | txt, log                         | 🟢         | 🟢       | 🟢        | Native                |                                              |
| Markdown                 | md                               | 🟢         | 🟢       | 🟢        | Native                |                                              |
| Structured Data          | json, xml, csv                   | 🟢         | 🟢       | 🟢        | Native                | Parsed through the structured-data provider. |
| Portable Document Format | pdf                              | 🟢         | 🟢       | 🟢        | PDFium                | Uses the PDFium shared library.              |
| Microsoft Word Document  | doc, docx                        | 🔴         | 🔴       | 🔴        | None                  | No document provider is registered.          |
| OpenDocument Text        | odt                              | 🔴         | 🔴       | 🔴        | None                  | No document provider is registered.          |
| Rich Text Format         | rtf                              | 🔴         | 🔴       | 🔴        | None                  | No document provider is registered.          |
| SpreadsheetML            | xls, xlsx, xlsm, xlt, xltx, xltm | 🔴         | 🔴       | 🔴        | None                  | No spreadsheet provider is registered.       |
| OpenDocument Spreadsheet | ods                              | 🔴         | 🔴       | 🔴        | None                  | No spreadsheet provider is registered.       |

## Code Formats

| Format                 | Extensions                | Thumbnail | Preview | Metadata | External dependencies | Notes                                                           |
| ---------------------- | ------------------------- | --------- | ------- | -------- | --------------------- | --------------------------------------------------------------- |
| C                      | c                         | 🔴         | 🔴       | 🔴        | None                  | No source-code/document provider is registered.                 |
| C++                    | cpp, cc, cxx, h, hpp, hxx | 🔴         | 🔴       | 🔴        | None                  | No source-code/document provider is registered.                 |
| Java                   | java                      | 🔴         | 🔴       | 🔴        | None                  | No source-code/document provider is registered.                 |
| Python                 | py, pyw                   | 🔴         | 🔴       | 🔴        | None                  | No source-code/document provider is registered.                 |
| JavaScript             | js, jsx                   | 🔴         | 🔴       | 🔴        | None                  | No source-code/document provider is registered.                 |
| TypeScript             | ts, tsx                   | 🔴         | 🔴       | 🔴        | None                  | No source-code/document provider is registered.                 |
| HTML/XHTML             | html, htm, xhtml, xht     | 🔴         | 🔴       | 🔴        | None                  | No source-code/document provider is registered.                 |
| Cascading Style Sheets | css                       | 🔴         | 🔴       | 🔴        | None                  | No source-code/document provider is registered.                 |
| PHP                    | php                       | 🔴         | 🔴       | 🔴        | None                  | No source-code/document provider is registered.                 |
| Ruby                   | rb, rbw                   | 🔴         | 🔴       | 🔴        | None                  | No source-code/document provider is registered.                 |
| Go                     | go                        | 🔴         | 🔴       | 🔴        | None                  | No source-code/document provider is registered.                 |
| Rust                   | rs                        | 🔴         | 🔴       | 🔴        | None                  | No source-code/document provider is registered.                 |
| Swift                  | swift                     | 🔴         | 🔴       | 🔴        | None                  | No source-code/document provider is registered.                 |
| Kotlin                 | kt, kts                   | 🔴         | 🔴       | 🔴        | None                  | No source-code/document provider is registered.                 |
| Shell                  | sh, bash, zsh, fish       | 🔴         | 🔴       | 🔴        | None                  | No source-code/document provider is registered.                 |
| SQL                    | sql                       | 🔴         | 🔴       | 🔴        | None                  | No source-code/document provider is registered.                 |
| TOML                   | toml                      | 🔴         | 🔴       | 🔴        | None                  | No structured-data provider is registered for this extension.   |
| YAML                   | yaml, yml                 | 🔴         | 🔴       | 🔴        | None                  | No structured-data provider is registered for these extensions. |

JSON, XML, and CSV are supported as structured documents in the Documents section; the remaining source-code formats above are not registered.

## Archives

| Format         | Extensions     | Thumbnail | Preview | Metadata | External dependencies | Notes                                                                                                  |
| -------------- | -------------- | --------- | ------- | -------- | --------------------- | ------------------------------------------------------------------------------------------------------ |
| ZIP            | zip            | 🟢         | 🔴       | 🔴        | zip crate             | Can extract an embedded thumbnail; no separate preview or metadata capability is implemented.          |
| Comic Book ZIP | cbz            | 🟢         | 🔴       | 🔴        | zip crate             | Can extract the first image as a thumbnail; no separate preview or metadata capability is implemented. |
| RAR            | rar            | 🔴         | 🔴       | 🔴        | None                  | Recognized for indexing only; no thumbnail, preview, or metadata capability.                           |
| 7-Zip          | 7z             | 🔴         | 🔴       | 🔴        | None                  | Recognized for indexing only; no thumbnail, preview, or metadata capability.                           |
| TAR            | tar            | 🔴         | 🔴       | 🔴        | None                  | Recognized for indexing only; no thumbnail, preview, or metadata capability.                           |
| GZIP           | gz             | 🔴         | 🔴       | 🔴        | None                  | Recognized for indexing only; compound `.tar.gz` files match by their `.gz` extension.                 |
| TAR Bzip2      | tar.bz2, .tbz2 | 🔴         | 🔴       | 🔴        | None                  | No provider is registered.                                                                             |
| TAR Xz         | tar.xz, .txz   | 🔴         | 🔴       | 🔴        | None                  | No provider is registered.                                                                             |
| ISO Image      | iso            | 🔴         | 🔴       | 🔴        | None                  | No provider is registered.                                                                             |
