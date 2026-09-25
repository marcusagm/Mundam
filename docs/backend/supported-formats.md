# Supported formats

## Images

### Raster formats

| Format                   | Extensions                     | Thumbnail | Preview | Metadata | Extraction strategy | Notes                                                               |
| ------------------------ | ------------------------------ | --------- | ------- | -------- | ------------------- | ------------------------------------------------------------------- |
| Bitmap                   | bmp                            | 🟢         | 🟢       | 🟢        | Native              |                                                                     |
| Animated PNG             | apng                           | 🟢         | 🟢       | 🟢        | Native              |                                                                     |
| Portable Network Graphic | png                            | 🟢         | 🟢       | 🟢        | Native              |                                                                     |
| Graphics Interchange     | gif                            | 🟢         | 🟢       | 🟢        | External            |                                                                     |
| Joint Photographic       | jpg, jpeg, jpe, jif, jfif, jfi | 🟢         | 🟢       | 🟢        | Native              |                                                                     |
| Tagged Image             | tiff, tif                      | 🟢         | 🟢       | 🟢        | Native              |                                                                     |
| Truevision               | tga, targa                     | 🟢         | 🟢       | 🟢        | Native              |                                                                     |
| High Efficiency          | heic, heif                     | 🟢         | 🟢       | 🟢        | Native              | Intermitent problems in preview, instability in decoder M3U8 local. |
| High Efficiency Sequence | heifs                          | 🟢         | 🟢       | 🟢        | Native              | Not tested.                                                         |
| WebP                     | webp                           | 🟢         | 🟢       | 🟢        | Native              |                                                                     |
| JPEG XL                  | jxl                            | 🟢         | 🟢       | 🟢        | Native              |                                                                     |
| JPEG 2000                | jp2, j2c                       | 🟢         | 🟢       | 🟢        | External            |                                                                     |
| OpenEXR                  | exr                            | 🟢         | 🟢       | 🟢        | Native              |                                                                     |
| AV1 Image Format         | avif, avifs                    | 🟢         | 🟢       | 🟢        | Native              |                                                                     |
| High Dynamic Range       | hdr                            | 🟢         | 🟢       | 🟢        | Native              |                                                                     |
| Portable Any Map         | pnm, ppm, pgm, pbm, pam        | 🟢         | 🟢       | 🟢        | Native              |                                                                     |

### Icon

| Format          | Extensions | Thumbnail | Preview | Metadata | Extraction strategy | Notes |
| --------------- | ---------- | --------- | ------- | -------- | ------------------- | ----- |
| Icon File       | ico        | 🟢         | 🟢       | 🟢        | Native              |       |
| Cur File        | cur        | 🟢         | 🟢       | 🟢        | Native              |       |
| Apple Icon File | icns       | 🟢         | 🟢       | 🟢        | Native              |       |

### Vector formats

| Format                   | Extensions | Thumbnail | Preview | Metadata | Extraction strategy | Notes |
| ------------------------ | ---------- | --------- | ------- | -------- | ------------------- | ----- |
| Scalable Vector Graphics | svg, svgz  | 🟢         | 🟢       | 🟢        | Native              |       |
| Encapsulated PostScript  | eps        | 🟢         | 🟢       | 🟢        | Native              |       |
| Portable Document Format | pdf        | 🟢         | 🟢       | 🟢        | External            |       |
| PostScript               | ps         | 🟢         | 🟢       | 🟢        | Native              |       |

### Raw camera formats

| Format                   | Extensions         | Thumbnail | Preview | Metadata | Extraction strategy | Notes                                                         |
| ------------------------ | ------------------ | --------- | ------- | -------- | ------------------- | ------------------------------------------------------------- |
| Hasselblad Raw           | 3fr, fff, iiq      | 🟢         | 🟢       | 🟢        | External            |                                                               |
| Sony Raw Format          | arw, sr2, srf, srw | 🟢         | 🟢       | 🟢        | External            |                                                               |
| Adobe Digital Negative   | dng                | 🟢         | 🟢       | 🟢        | External            |                                                               |
| Canon Raw Format         | cr2, cr3, crw      | 🟢         | 🟢       | 🟢        | External            |                                                               |
| DirectDraw Surface       | dds                | 🟢         | 🟢       | 🟢        | Native              |                                                               |
| DJI RAW                  | dcr                | 🔴         | 🔴       | 🔴        | None                | Removed from the v2 registry by obsolescence.                 |
| Epson Raw Format         | erf                | 🟢         | 🟢       | 🟢        | External            |                                                               |
| GoPro RAW Format         | gpr                | 🟢         | 🟢       | 🟢        | Native              |                                                               |
| Kodak Digital            | kdc                | 🟢         | 🟢       | 🟢        | External            | No embedded preview for legacy formats (DC120).               |
| Mamiya Electronic Format | mef                | 🟢         | 🟢       | 🟢        | External            |                                                               |
| Leaf Camera RAW          | mos                | 🟢         | 🟢       | 🟢        | External            | Metadata generating 'Unknown to this library' in some fields. |
| Minolta Raw              | mrw                | 🟢         | 🟢       | 🟢        | External            |                                                               |
| Nikon Electronic         | nef, nrw           | 🟢         | 🟢       | 🟢        | External            |                                                               |
| Olympus Raw Format       | orf                | 🟢         | 🟢       | 🟢        | External            |                                                               |
| Panasonic Raw            | rw2, rwl           | 🟢         | 🟢       | 🟢        | External            |                                                               |
| Pentax Raw               | pef                | 🟢         | 🟢       | 🟢        | External            |                                                               |
| Fujifilm RAW             | raf                | 🟢         | 🟢       | 🟢        | External            |                                                               |
| Leica Raw                | raw                | 🔴         | 🔴       | 🟢        | External            | Both versions without thumbnails and preview.                 |
| Sigma RAW                | x3f                | 🟢         | 🟢       | 🟢        | External            |                                                               |

## Project

### Design applications

| Format              | Extensions                   | Thumbnail | Preview | Metadata | Extraction strategy | Notes                                   |
| ------------------- | ---------------------------- | --------- | ------- | -------- | ------------------- | --------------------------------------- |
| Adobe Photoshop     | psd, psb                     | 🟢         | 🟢       | 🟢        | Native              |                                         |
| Adobe Illustrator   | ai                           | 🟢         | 🟢       | 🟢        | Native              |                                         |
| Adobe After Effects | aep                          | 🔴         | 🔴       | 🔴        | External            | Generic icon stub, did not index in v2. |
| Adobe Premiere      | prproj                       | 🔴         | 🔴       | 🔴        | External            | Generic icon stub, did not index in v2. |
| Adobe Animate       | fla                          | 🟢         | 🟢       | 🟢        | External            |                                         |
| Adobe Audition      | au                           | 🟢         | 🟢       | 🟢        | External            |                                         |
| Affinity            | af, afdesign, afphoto, afpub | 🟢         | 🟢       | 🟢        | Native              |                                         |
| Aseprite            | ase, aseprite                | 🟢         | 🟢       | 🟢        | Native              |                                         |
| Clip Studio Paint   | clip                         | 🟢         | 🟢       | 🟢        | Native              |                                         |
| Corel Painter       | rif, riff                    | 🟢         | 🟢       | 🟢        | Native              |                                         |
| CorelDRAW           | cdr                          | 🟢         | 🟢       | 🟢        | Native              |                                         |
| Figma               | fig                          | 🟢         | 🟢       | 🟢        | Native              | May improve comments extraction.        |
| FireAlpaca          | alp                          | 🟢         | 🟢       | 🟢        | External            |                                         |
| GIMP                | xcf                          | 🟢         | 🟢       | 🟢        | Native              | Complex blending modes pending.         |
| Krita               | kra, krz, krita              | 🟢         | 🟢       | 🟢        | Native              |                                         |
| MediBang            | mdp, medibang                | 🟢         | 🟢       | 🟢        | Native              |                                         |
| Paint Tool SAI      | sai, sai2                    | 🟢         | 🟢       | 🟢        | Native              |                                         |
| Penpot              | penpot                       | 🟢         | 🟢       | 🟢        | Native              |                                         |
| Rebelle             | reb                          | 🟢         | 🟢       | 🟢        | Native              |                                         |
| Sketch              | sketch                       | 🟢         | 🟢       | 🟢        | Native              |                                         |
| Sketchbook          | tiff                         | 🟢         | 🟢       | 🟢        | External            |                                         |
| Piskel              | piskel                       | 🔴         | 🔴       | 🔴        | External            |                                         |
| Procreate           | pro                          | 🔴         | 🔴       | 🔴        | External            |                                         |
| ARRIRAW             | ari                          | 🟠         | 🟠       | 🟠        | External            | Not tested.                             |
| Blackmagic RAW      | braw                         | 🔴         | 🔴       | 🔴        | External            | v2 did not index, identified as video.  |
| DaVinci Resolve     | drp                          | 🟠         | 🟠       | 🟠        | External            | Not tested.                             |
| Final Cut Pro       | fcpxml                       | 🟠         | 🟠       | 🟠        | External            | Not tested.                             |
| Adobe InDesign      | idml, indd                   | 🟠         | 🟠       | 🟠        | External            | Not tested.                             |
| RED Digital Cinema  | r3d                          | 🟠         | 🟠       | 🟠        | External            | Not tested.                             |

### Mind Maps, flowcharts and sketch-likes

| Format     | Extensions                  | Thumbnail | Preview | Metadata | Extraction strategy | Notes |
| ---------- | --------------------------- | --------- | ------- | -------- | ------------------- | ----- |
| XMind      | xmind                       | 🟢         | 🟢       | 🟢        | Native              |       |
| excalidraw | excalidraw, excalidraw.json | 🟢         | 🟢       | 🟢        | External            |       |
| tldraw     | tldraw, tldraw.json         | 🟢         | 🟢       | 🟢        | External            |       |
| drawio     | drawio, drawio.xml          | 🟢         | 🟢       | 🟢        | External            |       |
| Miro       | miro, miro.json             | 🟢         | 🟢       | 🟢        | External            |       |
| FigJam     | figjam, figjam.json         | 🟢         | 🟢       | 🟢        | External            |       |
| Mural      | mural, mural.json           | 🟢         | 🟢       | 🟢        | External            |       |
| eraser.io  | eraser.io, eraser.io.json   | 🟢         | 🟢       | 🟢        | External            |       |

## 3D Models

### Applications with Native 3D Capabilities

| Format        | Extensions | Thumbnail | Preview | Metadata | Extraction strategy | Notes |
| ------------- | ---------- | --------- | ------- | -------- | ------------------- | ----- |
| Blender       | blend      | 🟢         | 🟢       | 🟢        | Native              |       |
| Autodesk Maya | ma, mb     | 🟢         | 🟢       | 🟢        | Native              |       |
| Cinema 4D     | c4d        | 🟢         | 🟢       | 🟢        | Native              |       |
| Modo          | modo       | 🟢         | 🟢       | 🟢        | Native              |       |
| Houdini       | hip        | 🟢         | 🟢       | 🟢        | Native              |       |
| LightWave 3D  | lwo, lws   | 🟢         | 🟢       | 🟢        | Native              |       |
| ZBrush        | ztl, zpr   | 🟢         | 🟢       | 🟢        | External            |       |
| SketchUp      | skp        | 🟢         | 🟢       | 🟢        | External            |       |
| 3ds Max       | max        | 🟢         | 🟢       | 🟢        | External            |       |
| Rhino 3D      | 3dm        | 🟢         | 🟢       | 🟢        | External            |       |

### 3D Formats

| Format       | Extensions      | Thumbnail | Preview | Metadata | Extraction strategy | Notes |
| ------------ | --------------- | --------- | ------- | -------- | ------------------- | ----- |
| OBJ          | obj             | 🟢         | 🟢       | 🟢        | Native              |       |
| STL          | stl             | 🟢         | 🟢       | 🟢        | Native              |       |
| FBX          | fbx             | 🟢         | 🟢       | 🟢        | External            |       |
| glTF         | gltf, glb       | 🟢         | 🟢       | 🟢        | Native              |       |
| USD          | usd, usdc, usdz | 🟢         | 🟢       | 🟢        | Native              |       |
| Alembic      | abc             | 🟢         | 🟢       | 🟢        | Native              |       |
| STEP         | step,stp        | 🟢         | 🟢       | 🟢        | Native              |       |
| IGES         | iges, igs       | 🟢         | 🟢       | 🟢        | Native              |       |
| COLLADA      | dae             | 🟢         | 🟢       | 🟢        | Native              |       |
| 3MF          | 3mf             | 🟢         | 🟢       | 🟢        | Native              |       |
| PLY          | ply             | 🟢         | 🟢       | 🟢        | Native              |       |
| LightWave 3D | lwo, lws        | 🟢         | 🟢       | 🟢        | External            |       |

## Font

| Format                 | Extensions                | Thumbnail | Preview | Metadata | Extraction strategy | Notes                              |
| ---------------------- | ------------------------- | --------- | ------- | -------- | ------------------- | ---------------------------------- |
| TrueType Font          | ttf, ttc                  | 🟢         | 🟢       | 🟢        | Native              |                                    |
| OpenType Font          | otf, otc                  | 🟢         | 🟢       | 🟢        | Native              |                                    |
| Web Open Font Format   | woff, woff2               | 🟢         | 🟢       | 🟢        | Native              |                                    |
| Variable Font          | vttf, vf, fvar            | 🔴         | 🔴       | 🔴        | None                | Not supported.                     |
| Color Font             | color, COLR               | 🔴         | 🔴       | 🔴        | None                | Not supported.                     |
| OpenType Variable Font | opentype, variable, vfont | 🔴         | 🔴       | 🔴        | None                | Not supported.                     |
| FontForge Font         | sf, sfd                   | 🔴         | 🔴       | 🔴        | None                | Not supported.                     |
| Embedded OpenType Font | eot                       | 🔴         | 🔴       | 🔴        | None                | Not supported by glyph extraction. |

## Audio Formats

| Format                        | Extensions      | Thumbnail | Playback | Metadata | Thumbnail strategy | Playback strategy | Extraction strategy  | Notes                                               |
| ----------------------------- | --------------- | --------- | -------- | -------- | ------------------ | ----------------- | -------------------- | --------------------------------------------------- |
| Advanced Audio Coding         | aac             | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               | Some files had missing waveforms depending on size. |
| Audible Audio                 | aax             | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | ffmpeg               | Not tested.                                         |
| Dolby Digital Audio           | ac3             | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               | Some files had missing waveforms depending on size. |
| Audio Interchange File Format | aiff, aif, aifc | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               | Some files had missing waveforms depending on size. |
| Adaptive Multi-Rate           | amr             | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               |                                                     |
| Monkey's Audio                | ape             | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               |                                                     |
| Broadcast Wave Format         | bwf             | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               | Not tested.                                         |
| Core Audio Format             | caf             | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               | Some files had missing waveforms depending on size. |
| Digital Theater Systems       | dts             | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               | Some files had missing waveforms depending on size. |
| Free Lossless Audio Codec     | flac            | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               | Some files had missing waveforms depending on size. |
| Apple Lossless Audio Codec    | m4a             | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | AudioLinearHls       | M4A ALAC converted to AAC on-the-fly.               |
| MPEG-4 Ringtone               | m4r             | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | AudioLinearHls       |                                                     |
| Musical Instrument Digital    | mid, midi       | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | AudioLinearHls/Synth | Synthesizes to WAV on the fly.                      |
| Matroska Audio                | mka             | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               |                                                     |
| MPEG-1 Audio Layer II         | mp2             | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               | Some files had missing waveforms depending on size. |
| MP3                           | mp3             | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               | Some files had missing waveforms depending on size. |
| Ogg Audio                     | oga, ogg        | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               | Some files had missing waveforms depending on size. |
| Opus                          | opus            | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               | Some files had missing waveforms depending on size. |
| RealAudio                     | ra              | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               | Some files had missing waveforms depending on size. |
| Speex                         | spx             | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               | Some files had missing waveforms depending on size. |
| Waveform Audio File Format    | wav             | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               | Some files had missing waveforms depending on size. |
| Windows Media Audio           | wma             | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               | Some files had missing waveforms depending on size. |
| WavPack                       | wv              | 🟢         | 🟢        | 🟢        | Generic icon       | hls               | Native               | Some files had missing waveforms depending on size. |

## Video Formats

| Format | Extensions | Thumbnail | Playback | Metadata | Thumbnail strategy | Playback strategy | Extraction strategy | Notes                              |
| ------ | ---------- | --------- | -------- | -------- | ------------------ | ----------------- | ------------------- | ---------------------------------- |
| 3G2    | 3g2        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| 3GP    | 3gp        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| ASF    | asf        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| AVI    | avi        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| DivX   | divx       | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              | Not tested.                        |
| F4V    | f4v        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| FLV    | flv        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| H.264  | h264       | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              | Not tested.                        |
| H.265  | h265       | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              | Not tested.                        |
| HEVC   | hevc       | 🟠         | 🟠        | 🟢        | ffmpeg             | hls               | Native              | Instability in local M3U8 decoder. |
| M2TS   | m2ts       | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| M2V    | m2v        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| M4V    | m4v        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| MJPEG  | mjpeg      | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| MJPG   | mjpg       | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              | Not tested.                        |
| MKV    | mkv        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| MOV    | mov        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| MP4    | mp4        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| MPEG   | mpeg       | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| MPG    | mpg        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| MTS    | mts        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| MXF    | mxf        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| OGV    | ogv        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| QT     | qt         | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              | Not tested.                        |
| RM     | rm         | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| RMVB   | rmvb       | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              | Not tested.                        |
| SWF    | swf        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| TS     | ts         | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| VOB    | vob        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| WEBM   | webm       | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| WMV    | wmv        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| WTV    | wtv        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              |                                    |
| Y4M    | y4m        | 🟢         | 🟢        | 🟢        | ffmpeg             | hls               | Native              | Not tested.                        |

## Documents

| Format                   | Extensions                       | Thumbnail | Preview | Metadata | Extraction strategy | Notes |
| ------------------------ | -------------------------------- | --------- | ------- | -------- | ------------------- | ----- |
| Microsoft Word Document  | doc, docx                        | 🟢         | 🟢       | 🟢        | External            |       |
| OpenDocument Text        | odt                              | 🟢         | 🟢       | 🟢        | External            |       |
| Rich Text Format         | rtf                              | 🟢         | 🟢       | 🟢        | External            |       |
| Portable Document Format | pdf                              | 🟢         | 🟢       | 🟢        | External            |       |
| Markdown                 | md                               | 🟢         | 🟢       | 🟢        | External            |       |
| SpreadsheetML            | xls, xlsx, xlsm, xlt, xltx, xltm | 🟢         | 🟢       | 🟢        | External            |       |
| OpenDocument Spreadsheet | ods                              | 🟢         | 🟢       | 🟢        | External            |       |
| Comma-Separated Values   | csv                              | 🟢         | 🟢       | 🟢        | External            |       |

## Code Formats

| Format                 | Extensions                | Thumbnail | Preview | Metadata | Extraction strategy | Notes |
| ---------------------- | ------------------------- | --------- | ------- | -------- | ------------------- | ----- |
| C                      | c                         | 🟢         | 🟢       | 🟢        | External            |       |
| C++                    | cpp, cc, cxx, h, hpp, hxx | 🟢         | 🟢       | 🟢        | External            |       |
| Java                   | java                      | 🟢         | 🟢       | 🟢        | External            |       |
| Python                 | py, pyw                   | 🟢         | 🟢       | 🟢        | External            |       |
| JavaScript             | js, jsx                   | 🟢         | 🟢       | 🟢        | External            |       |
| TypeScript             | ts, tsx                   | 🟢         | 🟢       | 🟢        | External            |       |
| HTML/XHTML             | html, htm, xhtml, xht     | 🟢         | 🟢       | 🟢        | External            |       |
| Cascading Style Sheets | css                       | 🟢         | 🟢       | 🟢        | External            |       |
| PHP                    | php                       | 🟢         | 🟢       | 🟢        | External            |       |
| Ruby                   | rb, rbw                   | 🟢         | 🟢       | 🟢        | External            |       |
| Go                     | go                        | 🟢         | 🟢       | 🟢        | External            |       |
| Rust                   | rs                        | 🟢         | 🟢       | 🟢        | External            |       |
| Swift                  | swift                     | 🟢         | 🟢       | 🟢        | External            |       |
| Kotlin                 | kt, kts                   | 🟢         | 🟢       | 🟢        | External            |       |
| Shell                  | sh, bash, zsh, fish       | 🟢         | 🟢       | 🟢        | External            |       |
| SQL                    | sql                       | 🟢         | 🟢       | 🟢        | External            |       |
| JSON                   | json                      | 🟢         | 🟢       | 🟢        | External            |       |
| XML                    | xml                       | 🟢         | 🟢       | 🟢        | External            |       |
| TOML                   | toml                      | 🟢         | 🟢       | 🟢        | External            |       |
| YAML                   | yaml, yml                 | 🟢         | 🟢       | 🟢        | External            |       |

## Archives

| Format    | Extensions     | Thumbnail | Preview | Metadata | Extraction strategy | Notes |
| --------- | -------------- | --------- | ------- | -------- | ------------------- | ----- |
| Zip       | zip, Z         | 🟢         | 🟢       | 🟢        | External            |       |
| RAR       | rar            | 🟢         | 🟢       | 🟢        | External            |       |
| 7z        | 7z             | 🟢         | 🟢       | 🟢        | External            |       |
| Tar       | tar            | 🟢         | 🟢       | 🟢        | External            |       |
| Tar Gzip  | tar.gz, .tgz   | 🟢         | 🟢       | 🟢        | External            |       |
| Tar Bzip2 | tar.bz2, .tbz2 | 🟢         | 🟢       | 🟢        | External            |       |
| Tar Xz    | tar.xz, .txz   | 🟢         | 🟢       | 🟢        | External            |       |
| ISO Image | iso            | 🟢         | 🟢       | 🟢        | External            |       |
