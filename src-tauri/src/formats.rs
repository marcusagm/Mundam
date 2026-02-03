use serde::Serialize;
use strum_macros::{EnumIter, Display};
use std::path::Path;
use std::fs::File;
use std::io::Read;

#[derive(Debug, Clone, Serialize, EnumIter, Display, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MediaType {
    Image,
    Video,
    Audio,
    Project, // ex: .psd, .ai
    Archive, // ex: .zip
    Model3D,
    Font,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub enum ThumbnailStrategy {
    NativeImage, // Rust native decoders (image-rs, zune-jpeg)
    Ffmpeg,      // Video and complex formats
    Webview,     // SVG, HTML
    ZipPreview,  // Affinity, OpenOffice etc
    Icon,        // Fallback for files without preview
    None,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileFormat {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub mime_types: &'static [&'static str],
    pub type_category: MediaType,
    #[serde(skip)]
    pub strategy: ThumbnailStrategy,
}

impl FileFormat {
    /// Detects the real format of the file.
    /// Priority: 1. Magic Bytes (infer) -> 2. Extension (mime_guess/fallback) -> 3. None
    pub fn detect(path: &Path) -> Option<&'static FileFormat> {
        // Optimization: Open file once if possible.
        // For simple usage, we just wrap detect_header.
        if let Ok(mut file) = File::open(path) {
            return Self::detect_header(&mut file, path);
        }
        
        // Fallback if file open fails (locked?) - rely on extension only
        Self::detect_extension(path)
    }

    /// Detects format from an open file handle (reads header and rewinds).
    /// Used to avoid re-opening files in high-performance loops.
    pub fn detect_header(file: &mut File, path_fallback: &Path) -> Option<&'static FileFormat> {
        // 1. Try reading first bytes (Header)
        // 1024 bytes is enough for almost all magic bytes (infer usually needs < 300)
        let mut buffer = [0u8; 1024]; 
        
        // Read header
        if file.read(&mut buffer).is_ok() {
            // Rewind file for subsequent use!
            let _ = std::io::Seek::seek(file, std::io::SeekFrom::Start(0));

            if let Some(kind) = infer::get(&buffer) {
                // Check registry for the MIME returned by infer
                if let Some(fmt) = SUPPORTED_FORMATS.iter().find(|f| f.mime_types.contains(&kind.mime_type())) {
                    return Some(fmt);
                }
            }
        }

        // 2. Fallback: Extension
        Self::detect_extension(path_fallback)
    }

    fn detect_extension(path: &Path) -> Option<&'static FileFormat> {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            return SUPPORTED_FORMATS.iter().find(|f| f.extensions.contains(&ext_lower.as_str()));
        }
        None
    }
}

// THE MASTER REGISTRY
pub const SUPPORTED_FORMATS: &[FileFormat] = &[
    // --- IMAGES (NATIVE) ---
    FileFormat {
        name: "JPEG Image",
        extensions: &["jpg", "jpeg", "jpe", "jfif"],
        mime_types: &["image/jpeg"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
    },
    FileFormat {
        name: "PNG Image",
        extensions: &["png"],
        mime_types: &["image/png"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
    },
    FileFormat {
        name: "WebP Image",
        extensions: &["webp"],
        mime_types: &["image/webp"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
    },
    FileFormat {
        name: "GIF Image",
        extensions: &["gif"],
        mime_types: &["image/gif"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
    },
    FileFormat {
        name: "Bitmap Image",
        extensions: &["bmp"],
        mime_types: &["image/bmp"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
    },
    FileFormat {
        name: "Windows Icon",
        extensions: &["ico"],
        mime_types: &["image/x-icon", "image/vnd.microsoft.icon"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
    },
    FileFormat {
        name: "TIFF Image",
        extensions: &["tif", "tiff"],
        mime_types: &["image/tiff"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
    },
    FileFormat {
        name: "Targa Image",
        extensions: &["tga"],
        mime_types: &["image/x-tga", "image/targa"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
    },

    // --- RAW PHOTOS (FFMPEG/LIBRAW - mapped to Ffmpeg or Native based on impl) ---
    FileFormat {
        name: "Canon Raw",
        extensions: &["cr2", "cr3", "crw"],
        mime_types: &["image/x-canon-cr2", "image/x-canon-crw"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Ffmpeg, 
    },
    FileFormat {
        name: "Nikon Raw",
        extensions: &["nef", "nrw"],
        mime_types: &["image/x-nikon-nef"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Ffmpeg,
    },
    FileFormat {
        name: "Sony Raw",
        extensions: &["arw", "srf", "sr2"],
        mime_types: &["image/x-sony-arw", "image/x-sony-srf", "image/x-sony-sr2"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Ffmpeg,
    },
    FileFormat {
        name: "Adobe DNG",
        extensions: &["dng"],
        mime_types: &["image/x-adobe-dng"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Ffmpeg,
    },
    FileFormat {
        name: "Fujifilm Raw",
        extensions: &["raf"],
        mime_types: &["image/x-fuji-raf"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Ffmpeg,
    },
    FileFormat {
        name: "Olympus Raw",
        extensions: &["orf"],
        mime_types: &["image/x-olympus-orf"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Ffmpeg,
    },
    FileFormat {
        name: "Panasonic Raw",
        extensions: &["rw2"],
        mime_types: &["image/x-panasonic-rw2"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Ffmpeg,
    },

    // --- MODERN FORMATS ---
    FileFormat {
        name: "High Efficiency Image",
        extensions: &["heic", "heif"],
        mime_types: &["image/heic", "image/heif"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Ffmpeg,
    },
    FileFormat {
        name: "AV1 Image",
        extensions: &["avif"],
        mime_types: &["image/avif"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Ffmpeg,
    },

    // --- DESIGN & VECTORS ---
    FileFormat {
        name: "Scalable Vector Graphics",
        extensions: &["svg"],
        mime_types: &["image/svg+xml"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Webview, // SVG renders best in WebView
    },
    FileFormat {
        name: "Adobe Photoshop",
        extensions: &["psd", "psb"],
        mime_types: &["image/vnd.adobe.photoshop"],
        type_category: MediaType::Project,
        strategy: ThumbnailStrategy::Ffmpeg, 
    },
    FileFormat {
        name: "Adobe Illustrator",
        extensions: &["ai", "eps"],
        mime_types: &["application/postscript", "application/illustrator"],
        type_category: MediaType::Project,
        strategy: ThumbnailStrategy::Ffmpeg, // Attempt to extract PDF preview stream
    },

    // --- 3D MODELS ---
    FileFormat {
        name: "Blender Project",
        extensions: &["blend"],
        mime_types: &["application/x-blender"],
        type_category: MediaType::Model3D,
        strategy: ThumbnailStrategy::Icon,
    },
    FileFormat {
        name: "FBX Model",
        extensions: &["fbx"],
        mime_types: &["application/octet-stream"], // Often generic
        type_category: MediaType::Model3D,
        strategy: ThumbnailStrategy::Icon,
    },
    FileFormat {
        name: "OBJ Model",
        extensions: &["obj"],
        mime_types: &["model/obj", "text/plain"],
        type_category: MediaType::Model3D,
        strategy: ThumbnailStrategy::Icon,
    },

    // --- FONTS ---
    FileFormat {
        name: "TrueType Font",
        extensions: &["ttf"],
        mime_types: &["font/ttf"],
        type_category: MediaType::Font,
        strategy: ThumbnailStrategy::Icon,
    },
     FileFormat {
        name: "OpenType Font",
        extensions: &["otf"],
        mime_types: &["font/otf"],
        type_category: MediaType::Font,
        strategy: ThumbnailStrategy::Icon,
    },

    // --- ZIP PREVIEW FORMATS ---
    FileFormat {
        name: "Affinity Design",
        extensions: &["afdesign"],
        mime_types: &["application/x-affinity-design"],
        type_category: MediaType::Project,
        strategy: ThumbnailStrategy::ZipPreview,
    },
    FileFormat {
        name: "Affinity Photo",
        extensions: &["afphoto"],
        mime_types: &["application/x-affinity-photo"],
        type_category: MediaType::Project,
        strategy: ThumbnailStrategy::ZipPreview,
    },
     FileFormat {
        name: "Affinity Publisher",
        extensions: &["afpub"],
        mime_types: &["application/x-affinity-publisher"],
        type_category: MediaType::Project,
        strategy: ThumbnailStrategy::ZipPreview,
    },
    FileFormat {
        name: "Clip Studio Paint",
        extensions: &["clip"],
        mime_types: &["application/x-clip-studio-paint"],
        type_category: MediaType::Project,
        strategy: ThumbnailStrategy::ZipPreview,
    },
    FileFormat {
        name: "XMind Map",
        extensions: &["xmind"],
        mime_types: &["application/x-xmind"],
        type_category: MediaType::Project,
        strategy: ThumbnailStrategy::ZipPreview,
    },

    // --- VIDEOS ---
    FileFormat {
        name: "MPEG-4 Video",
        extensions: &["mp4", "m4v"],
        mime_types: &["video/mp4"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
    },
    FileFormat {
        name: "WebM Video",
        extensions: &["webm"],
        mime_types: &["video/webm"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
    },
    // --- PROFESSIONAL VIDEO & CGI ---
    FileFormat {
        name: "QuickTime Video",
        extensions: &["mov", "qt"],
        mime_types: &["video/quicktime"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
    },
    FileFormat {
        name: "Material Exchange Format",
        extensions: &["mxf"],
        mime_types: &["application/mxf", "video/mxf"],
        type_category: MediaType::Video,
        strategy: ThumbnailStrategy::Ffmpeg,
    },
    FileFormat {
        name: "OpenEXR Image",
        extensions: &["exr"],
        mime_types: &["image/x-exr"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Ffmpeg, 
    },
    FileFormat {
        name: "Radiance HDR",
        extensions: &["hdr"],
        mime_types: &["image/vnd.radiance"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::Ffmpeg, 
    },
];
