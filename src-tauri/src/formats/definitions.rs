use super::FileFormat;
use super::types::{MediaType, ThumbnailStrategy, PlaybackStrategy};

// THE MASTER REGISTRY
pub const SUPPORTED_FORMATS: &[FileFormat] = &[
    // --- IMAGES (NATIVE) ---
    FileFormat {
        name: "JPEG Image",
        extensions: &["jpg", "jpeg", "jpe", "jfif"],
        mime_types: &["image/jpeg"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "PNG Image",
        extensions: &["png"],
        mime_types: &["image/png"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "WebP Image",
        extensions: &["webp"],
        mime_types: &["image/webp"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "GIF Image",
        extensions: &["gif"],
        mime_types: &["image/gif"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Bitmap Image",
        extensions: &["bmp"],
        mime_types: &["image/bmp"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Windows Icon",
        extensions: &["ico"],
        mime_types: &["image/x-icon", "image/vnd.microsoft.icon"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "TIFF Image",
        extensions: &["tif", "tiff"],
        mime_types: &["image/tiff"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Windows Cursor",
        extensions: &["cur"],
        mime_types: &["image/x-icon"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Targa Image",
        extensions: &["tga"],
        mime_types: &["image/x-tga", "image/targa"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeExtractor,
        playback: PlaybackStrategy::None,
    },

    // --- RAW PHOTOS ---
    FileFormat {
        name: "Canon Raw",
        extensions: &["cr2", "cr3", "crw"],
        mime_types: &["image/x-canon-cr2", "image/x-canon-crw"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Raw,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Nikon Raw",
        extensions: &["nef", "nrw"],
        mime_types: &["image/x-nikon-nef"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Raw,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Sony Raw",
        extensions: &["arw", "srf", "sr2"],
        mime_types: &["image/x-sony-arw", "image/x-sony-srf", "image/x-sony-sr2"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Raw,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Adobe DNG",
        extensions: &["dng"],
        mime_types: &["image/x-adobe-dng"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Raw,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Fujifilm Raw",
        extensions: &["raf"],
        mime_types: &["image/x-fuji-raf"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Raw,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Olympus Raw",
        extensions: &["orf"],
        mime_types: &["image/x-olympus-orf"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Raw,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Panasonic Raw",
        extensions: &["rw2"],
        mime_types: &["image/x-panasonic-rw2"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Raw,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Pentax/Epson Raw",
        extensions: &["pef", "erf"],
        mime_types: &["image/x-pentax-pef", "image/x-epson-erf"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Raw,
        playback: PlaybackStrategy::None,
    },

    // --- MODERN FORMATS ---
    FileFormat {
        name: "High Efficiency Image",
        extensions: &["heic", "heif"],
        mime_types: &["image/heic", "image/heif"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "AV1 Image",
        extensions: &["avif"],
        mime_types: &["image/avif"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::None,
    },

    // --- DESIGN & VECTORS ---
    FileFormat {
        name: "Scalable Vector Graphics",
        extensions: &["svg"],
        mime_types: &["image/svg+xml"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Webview,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Adobe Photoshop",
        extensions: &["psd", "psb"],
        mime_types: &["image/vnd.adobe.photoshop"],
        type_category: MediaType::Project,
        strategy: ThumbnailStrategy::NativeExtractor,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Adobe Illustrator",
        extensions: &["ai", "eps"],
        mime_types: &["application/postscript", "application/illustrator"],
        type_category: MediaType::Project,
        strategy: ThumbnailStrategy::NativeExtractor,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "GIMP Image",
        extensions: &["xcf"],
        mime_types: &["image/x-xcf"],
        type_category: MediaType::Project,
        strategy: ThumbnailStrategy::Icon, // TODO: XCF parser
        playback: PlaybackStrategy::None,
    },

    // --- 3D MODELS ---
    FileFormat {
        name: "Blender Project",
        extensions: &["blend"],
        mime_types: &["application/x-blender"],
        type_category: MediaType::Model3D,
        strategy: ThumbnailStrategy::NativeExtractor, // Extract internal preview
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "FBX Model",
        extensions: &["fbx"],
        mime_types: &["application/octet-stream"], // Often generic
        type_category: MediaType::Model3D,
        strategy: ThumbnailStrategy::Model3D,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "OBJ Model",
        extensions: &["obj"],
        mime_types: &["model/obj", "text/plain"],
        type_category: MediaType::Model3D,
        strategy: ThumbnailStrategy::Model3D,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "GL Transmission Format",
        extensions: &["gltf"],
        mime_types: &["model/gltf+json"],
        type_category: MediaType::Model3D,
        strategy: ThumbnailStrategy::Model3D,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Binary GLTF",
        extensions: &["glb"],
        mime_types: &["model/gltf-binary"],
        type_category: MediaType::Model3D,
        strategy: ThumbnailStrategy::Model3D,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Collada Model",
        extensions: &["dae"],
        mime_types: &["model/vnd.collada+xml"],
        type_category: MediaType::Model3D,
        strategy: ThumbnailStrategy::Model3D,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Stereolithography",
        extensions: &["stl"],
        mime_types: &["model/stl", "application/vnd.ms-pki.stl"],
        type_category: MediaType::Model3D,
        strategy: ThumbnailStrategy::Model3D,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "3D Studio",
        extensions: &["3ds"],
        mime_types: &["application/x-3ds"],
        type_category: MediaType::Model3D,
        strategy: ThumbnailStrategy::Model3D,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "AutoCAD DXF",
        extensions: &["dxf"],
        mime_types: &["image/vnd.dxf"],
        type_category: MediaType::Model3D,
        strategy: ThumbnailStrategy::Model3D,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "LightWave Object",
        extensions: &["lwo"],
        mime_types: &["image/x-lwo"],
        type_category: MediaType::Model3D,
        strategy: ThumbnailStrategy::Model3D,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "LightWave Scene",
        extensions: &["lws"],
        mime_types: &["image/x-lws"],
        type_category: MediaType::Model3D,
        strategy: ThumbnailStrategy::Model3D,
        playback: PlaybackStrategy::None,
    },

    // --- FONTS ---
    FileFormat {
        name: "TrueType Font",
        extensions: &["ttf"],
        mime_types: &["font/ttf"],
        type_category: MediaType::Font,
        strategy: ThumbnailStrategy::Font,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "OpenType Font",
        extensions: &["otf"],
        mime_types: &["font/otf"],
        type_category: MediaType::Font,
        strategy: ThumbnailStrategy::Font,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "TrueType Collection",
        extensions: &["ttc"],
        mime_types: &["font/collection", "font/ttc"],
        type_category: MediaType::Font,
        strategy: ThumbnailStrategy::Font,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Web Open Font Format",
        extensions: &["woff"],
        mime_types: &["font/woff"],
        type_category: MediaType::Font,
        strategy: ThumbnailStrategy::Font, // fontdb doesn't support WOFF
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Web Open Font Format 2",
        extensions: &["woff2"],
        mime_types: &["font/woff2"],
        type_category: MediaType::Font,
        strategy: ThumbnailStrategy::Font, // fontdb doesn't support WOFF2
        playback: PlaybackStrategy::None,
    },

    // --- ZIP PREVIEW FORMATS ---
    FileFormat {
        name: "Affinity Design",
        extensions: &["afdesign"],
        mime_types: &["application/x-affinity-design"],
        type_category: MediaType::Project,
        strategy: ThumbnailStrategy::NativeExtractor,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Affinity Photo",
        extensions: &["afphoto"],
        mime_types: &["application/x-affinity-photo"],
        type_category: MediaType::Project,
        strategy: ThumbnailStrategy::NativeExtractor,
        playback: PlaybackStrategy::None,
    },
     FileFormat {
        name: "Affinity Publisher",
        extensions: &["afpub"],
        mime_types: &["application/x-affinity-publisher"],
        type_category: MediaType::Project,
        strategy: ThumbnailStrategy::NativeExtractor,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Clip Studio Paint",
        extensions: &["clip"],
        mime_types: &["application/x-clip-studio-paint"],
        type_category: MediaType::Project,
        strategy: ThumbnailStrategy::ZipPreview,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "XMind Map",
        extensions: &["xmind"],
        mime_types: &["application/x-xmind"],
        type_category: MediaType::Project,
        strategy: ThumbnailStrategy::ZipPreview,
        playback: PlaybackStrategy::None,
    },

    // --- VIDEOS ---
    FileFormat {
        name: "OpenEXR Image",
        extensions: &["exr"],
        mime_types: &["image/x-exr"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeExtractor,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Radiance HDR",
        extensions: &["hdr"],
        mime_types: &["image/vnd.radiance"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeExtractor,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "DirectDraw Surface",
        extensions: &["dds"],
        mime_types: &["image/vnd-ms.dds"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeExtractor,
        playback: PlaybackStrategy::None,
    },
    FileFormat {
        name: "Netpbm Formats",
        extensions: &["pbm", "pgm", "ppm", "pnm", "pam"],
        mime_types: &["image/x-portable-bitmap", "image/x-portable-graymap", "image/x-portable-pixmap", "image/x-portable-anymap"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeExtractor,
        playback: PlaybackStrategy::None,
    },

    // --- VIDEOS (FFMPEG) ---
    FileFormat {
        name: "MPEG-4 Video",
        extensions: &["mp4", "m4v"],
        mime_types: &["video/mp4"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::Native,
    },
    FileFormat {
        name: "WebM Video",
        extensions: &["webm"],
        mime_types: &["video/webm"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::Hls, // USER LIST: webm -> HLS
    },
    FileFormat {
        name: "QuickTime Video",
        extensions: &["mov", "qt"], // USER: mov -> Native. qt assumed Native as alias.
        mime_types: &["video/quicktime"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::Native,
    },
    FileFormat {
        name: "Matroska Video",
        extensions: &["mkv"],
        mime_types: &["video/x-matroska"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::Hls, // USER LIST: mkv -> HLS
    },
     FileFormat {
        name: "Matroska Audio",
        extensions: &["mka"],
        mime_types: &["audio/x-matroska"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::AudioHls, // USER: Transcode replaced by LinearHLS
    },
    FileFormat {
        name: "Material Exchange Format",
        extensions: &["mxf"],
        mime_types: &["application/mxf", "video/mxf"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::Hls, // USER LIST: mxf -> HLS
    },
    FileFormat {
        name: "Windows Media Video",
        extensions: &["wmv", "asf"],
        mime_types: &["video/x-ms-wmv", "video/x-ms-asf"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::Hls, // USER LIST: wmv, asf -> HLS
    },
    FileFormat {
        name: "Flash Video",
        extensions: &["flv", "f4v"],
        mime_types: &["video/x-flv"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::Hls, // USER LIST: flv, f4v -> HLS
    },
    FileFormat {
        name: "Shockwave Flash",
        extensions: &["swf"],
        mime_types: &["application/x-shockwave-flash"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::LinearHls, // USER LIST: swf -> HLS Linear
    },
    FileFormat {
        name: "MPEG-1/2 Video",
        extensions: &["mpg", "mpeg", "m2v"],
        mime_types: &["video/mpeg", "video/mp2p"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::LinearHls, // USER LIST: mpg, mpeg, m2v -> HLS Linear
    },
    FileFormat {
        name: "MPEG Transport Stream / DVD",
        extensions: &["vob", "ts", "mts", "m2ts"],
        mime_types: &["video/mp2t", "video/x-m2ts"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::Hls, // USER LIST: ts, mts, vob, m2ts -> HLS
    },
    FileFormat {
        name: "AVI Video",
        extensions: &["avi", "divx"],
        mime_types: &["video/x-msvideo", "video/avi"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::Hls, // USER LIST: avi -> HLS
    },
    FileFormat {
        name: "3GPP Video",
        extensions: &["3gp", "3g2"],
        mime_types: &["video/3gpp", "video/3gpp2"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::Hls, // USER LIST: 3gp, 3g2 -> HLS
    },
    FileFormat {
        name: "RealMedia Video",
        extensions: &["rm", "rmvb"],
        mime_types: &["application/vnd.rn-realmedia"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::Hls, // USER LIST: rm -> HLS
    },
     FileFormat {
        name: "RealAudio",
        extensions: &["ra"],
        mime_types: &["audio/vnd.rn-realaudio"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::AudioHls, // USER: Transcode replaced by LinearHLS
    },
    FileFormat {
        name: "Windows Recorded TV Show",
        extensions: &["wtv"],
        mime_types: &["video/x-wtv"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::Hls, // USER LIST: wtv -> HLS
    },
    FileFormat {
        name: "Ogg Video",
        extensions: &["ogv"],
        mime_types: &["video/ogg"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::Hls, // USER LIST: ogv -> HLS
    },
    FileFormat {
        name: "Motion JPEG",
        extensions: &["mjpeg", "mjpg"],
        mime_types: &["video/x-motion-jpeg"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::LinearHls, // USER LIST: mjpeg -> HLS Linear
    },
    FileFormat {
        name: "High Efficiency Video Coding",
        extensions: &["hevc"],
        mime_types: &["video/mp4"], // Often inside mp4 container usually, but raw hevc exists
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
        playback: PlaybackStrategy::LinearHls, // Assumption based on others, kept consistent
    },

    // --- AUDIO ---
    FileFormat {
        name: "MP3 Audio",
        extensions: &["mp3"],
        mime_types: &["audio/mpeg", "audio/mp3"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::Native, // USER LIST: mp3 -> Native
    },
     FileFormat {
        name: "MPEG-1 Audio Layer II",
        extensions: &["mp2"],
        mime_types: &["audio/mpeg"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::Native, // USER LIST: mp2 -> Native
    },
    FileFormat {
        name: "Waveform Audio",
        extensions: &["wav"],
        mime_types: &["audio/wav", "audio/x-wav", "audio/wave"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::Native, // USER LIST: wav -> Native
    },
    FileFormat {
        name: "FLAC Audio",
        extensions: &["flac"],
        mime_types: &["audio/flac", "audio/x-flac"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::Native, // USER LIST: flac -> Native
    },
    FileFormat {
        name: "Ogg Audio",
        extensions: &["ogg", "oga"],
        mime_types: &[],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::AudioHls, // USER LIST: ogg, oga -> LISTED IN BOTH HLS and Transcode. Prefer HLS for safety.
    },
    FileFormat {
        name: "Opus Audio",
        extensions: &["opus"],
        mime_types: &["audio/opus"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::AudioHls, // USER: Transcode replaced by LinearHLS
    },
    FileFormat {
        name: "MPEG-4 Audio",
        extensions: &["m4a", "aac", "m4r"],
        mime_types: &["audio/mp4", "audio/aac", "audio/x-m4a"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::Native, // USER LIST: aac, m4a, m4r -> Native
    },
    FileFormat {
        name: "Windows Media Audio",
        extensions: &["wma"],
        mime_types: &["audio/x-ms-wma"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::AudioHls,
    },
    FileFormat {
        name: "AIFF Audio",
        extensions: &["aiff", "aif"],
        mime_types: &["audio/x-aiff", "audio/aiff"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::AudioHls, // USER: Transcode replaced by LinearHLS
    },
    FileFormat {
        name: "Compact AIFF Audio",
        extensions: &["aifc"],
        mime_types: &["audio/x-aiff"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::AudioHls, // USER LIST: aifc -> HLS
    },
    FileFormat {
        name: "Speex Audio",
        extensions: &["spx"],
        mime_types: &["audio/ogg"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::AudioHls, // USER: Transcode replaced by LinearHLS
    },
    FileFormat {
        name: "Dolby Digital",
        extensions: &["ac3"],
        mime_types: &["audio/ac3"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::AudioHls,
    },
    FileFormat {
        name: "DTS Audio",
        extensions: &["dts"],
        mime_types: &["audio/vnd.dts"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::AudioHls,
    },
    FileFormat {
        name: "AMR Audio",
        extensions: &["amr"],
        mime_types: &["audio/amr"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::AudioHls,
    },
    FileFormat {
        name: "Monkey's Audio",
        extensions: &["ape"],
        mime_types: &["audio/x-ape"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::AudioHls,
    },
    FileFormat {
        name: "WavPack Audio",
        extensions: &["wv"],
        mime_types: &["audio/wavpack", "audio/x-wavpack"],
        type_category: MediaType::Audio,
        strategy: ThumbnailStrategy::Icon,
        playback: PlaybackStrategy::AudioHls,
    },
];
