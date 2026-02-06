export type MediaType = 'image' | 'video' | 'audio' | 'font' | 'model' | 'unknown';

export const getMediaType = (filename: string): MediaType => {
    const ext = filename.split('.').pop()?.toLowerCase();
    if (!ext) return 'unknown';

    const imageExts = [
        'jpg', 'jpeg', 'jpe', 'jfif', 'png', 'webp', 'gif', 'bmp', 'ico', 'svg', 'avif',
        'afphoto', 'afdesign', 'afpub', 'psd', 'psb', 'arw', 'cr2', 'cr3', 'nef', 'dng',
        'raf', 'orf', 'rw2', 'nrw', 'srf', 'sr2', 'crw', 'erf', 'pef', 'tga', 'tiff', 
        'tif', 'heic', 'heif', 'exr', 'hdr', 'clip', 'ai', 'eps', 'dds'
    ];
    const videoExts = [
        'mp4', 'm4v', 'webm', 'mov', 'qt', 'mxf', 'mkv', 'avi', 'wmv', 'flv', 'mpg', 
        'mpeg', 'ts', 'mts', 'm2ts', 'vob', 'm2v', 'asf', 'f4v', 'swf'
    ];
    const audioExts = ['mp3', 'wav', 'ogg', 'aac', 'flac', 'm4a', 'oga', 'opus'];
    const fontExts = ['ttf', 'otf', 'ttc', 'woff', 'woff2'];
    const modelExts = [
        'fbx', 'obj', 'glb', 'gltf', 'stl', 'dae', '3ds', 'dxf', 'lwo', 'lws', 'blend'
    ];

    if (imageExts.includes(ext)) return 'image';
    if (videoExts.includes(ext)) return 'video';
    if (audioExts.includes(ext)) return 'audio';
    if (fontExts.includes(ext)) return 'font';
    if (modelExts.includes(ext)) return 'model';

    return 'unknown';
};
