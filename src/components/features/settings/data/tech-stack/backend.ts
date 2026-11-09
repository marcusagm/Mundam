import { TechCategory } from './types';

export const backendEngine: TechCategory = {
    title: 'Backend Native Engine (Rust)',
    items: [
        {
            name: 'asefile',
            description: 'Aseprite format parsing (.ase, .aseprite).',
            url: 'https://crates.io/crates/asefile'
        },
        {
            name: 'axum & tower-http',
            description: 'Web application framework & middleware.',
            url: 'https://tokio.rs/#axum'
        },
        {
            name: 'base64',
            description: 'Base64 encoding/decoding.',
            url: 'https://crates.io/crates/base64'
        },
        {
            name: 'blake3',
            description: 'Fast cryptographic hash for content identity and duplicate detection.',
            url: 'https://crates.io/crates/blake3'
        },
        {
            name: 'byteorder',
            description: 'Reading/writing numbers in big/little endian.',
            url: 'https://crates.io/crates/byteorder'
        },
        {
            name: 'chrono',
            description: 'Date and time library for Rust.',
            url: 'https://crates.io/crates/chrono'
        },
        {
            name: 'dashmap',
            description: 'Concurrent hash map for shared runtime state.',
            url: 'https://crates.io/crates/dashmap'
        },
        {
            name: 'ddsfile & image_dds',
            description: 'DDS texture container reading/writing.',
            url: 'https://crates.io/crates/ddsfile'
        },
        {
            name: 'fast_image_resize',
            description: 'SIMD-accelerated image resize.',
            url: 'https://crates.io/crates/fast_image_resize'
        },
        {
            name: 'flashraw',
            description: 'Pure-Rust RAW parser for camera formats including GoPro GPR/VC-5.',
            url: 'https://crates.io/crates/flashraw'
        },
        {
            name: 'flate2 & zstd',
            description: 'Compression algorithms.',
            url: 'https://crates.io/crates/zstd'
        },
        {
            name: 'icns',
            description: 'Reads/writes macOS icon files.',
            url: 'https://crates.io/crates/icns'
        },
        {
            name: 'image & imagesize',
            description: 'Image processing and dimension detection.',
            url: 'https://crates.io/crates/image'
        },
        {
            name: 'infer & mime_guess',
            description: 'MIME types discovery and magic bytes.',
            url: 'https://crates.io/crates/infer'
        },
        {
            name: 'jxl-oxide',
            description: 'JPEG XL decoding.',
            url: 'https://crates.io/crates/jxl-oxide'
        },
        {
            name: 'kmeans_colors & palette',
            description: 'Color clustering and space conversion.',
            url: 'https://crates.io/crates/kmeans_colors'
        },
        {
            name: 'memmap2',
            description: 'Memory-mapped file API.',
            url: 'https://crates.io/crates/memmap2'
        },
        {
            name: 'midly, rustysynth & hound',
            description: 'MIDI parsing, software synthesis, and WAV audio.',
            url: 'https://crates.io/crates/midly'
        },
        {
            name: 'notify',
            description: 'Cross-platform filesystem notification system.',
            url: 'https://crates.io/crates/notify'
        },
        {
            name: 'pdfium-render',
            description: 'High-level bindings to Google PDFium.',
            url: 'https://crates.io/crates/pdfium-render'
        },
        {
            name: 'percent-encoding & urlencoding',
            description: 'URL encoding for strings.',
            url: 'https://crates.io/crates/urlencoding'
        },
        {
            name: 'psd',
            description: 'Parsing Photoshop PSD files.',
            url: 'https://crates.io/crates/psd'
        },
        {
            name: 'quick-xml',
            description: 'High performance XML reader/writer.',
            url: 'https://crates.io/crates/quick-xml'
        },
        {
            name: 'rayon',
            description: 'Data-parallelism library for Rust.',
            url: 'https://crates.io/crates/rayon'
        },
        {
            name: 'resvg & tiny-skia',
            description: 'SVG rendering engine.',
            url: 'https://crates.io/crates/resvg'
        },
        {
            name: 'rexif',
            description: 'EXIF metadata parsing.',
            url: 'https://crates.io/crates/rexif'
        },
        {
            name: 'serde & serde_json',
            description: 'Data serialization framework.',
            url: 'https://serde.rs/'
        },
        {
            name: 'sqlx (sqlite)',
            description: 'Asynchronous SQL toolkit.',
            url: 'https://crates.io/crates/sqlx'
        },
        {
            name: 'strum & strum_macros',
            description: 'Macros for working with enums.',
            url: 'https://crates.io/crates/strum'
        },
        {
            name: 'tauri-plugin-*',
            description: 'Plugins for dialogs, fs, HTTP, MCP, Opener, SQL, clipboard.',
            url: 'https://github.com/tauri-apps/plugins-workspace'
        },
        {
            name: 'thiserror',
            description: 'Derive macro for standard Error trait.',
            url: 'https://crates.io/crates/thiserror'
        },
        {
            name: 'tokio & tokio-util',
            description: 'Asynchronous runtime and utilities.',
            url: 'https://tokio.rs/'
        },
        {
            name: 'tracing & tracing-subscriber',
            description: 'Application-level logging and profiling.',
            url: 'https://crates.io/crates/tracing'
        },
        {
            name: 'ttf-parser',
            description: 'Parses TrueType and OpenType font metadata.',
            url: 'https://crates.io/crates/ttf-parser'
        },
        {
            name: 'unicode-normalization',
            description: 'Unicode normalization for consistent string handling.',
            url: 'https://crates.io/crates/unicode-normalization'
        },
        {
            name: 'uuid',
            description: 'Unique identifiers generation.',
            url: 'https://crates.io/crates/uuid'
        },
        {
            name: 'wait-timeout',
            description: 'Process waiting with timeout.',
            url: 'https://crates.io/crates/wait-timeout'
        },
        {
            name: 'walkdir',
            description: 'Directory tree traversal recursively.',
            url: 'https://crates.io/crates/walkdir'
        },
        {
            name: 'webp & zune-jpeg',
            description: 'Specific fast image codecs (WebP, JPEG).',
            url: 'https://crates.io/crates/zune-jpeg'
        },
        {
            name: 'zip',
            description: 'ZIP archive reading/writing.',
            url: 'https://crates.io/crates/zip'
        }
    ]
};
