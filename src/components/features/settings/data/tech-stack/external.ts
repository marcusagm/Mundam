import { TechCategory } from './types';

export const externalTools: TechCategory = {
    title: 'External System Tools',
    items: [
        {
            name: 'FFmpeg & FFprobe',
            description: 'High-performance multimedia processing, transcoding, and media metadata.',
            url: 'https://ffmpeg.org/'
        },
        {
            name: 'Assimp',
            description: 'Open Asset Import Library for 3D model standardization.',
            url: 'https://www.assimp.org/'
        },
        {
            name: 'PDFium',
            description: 'Google PDFium for high-fidelity PDF page rendering.',
            url: 'https://pdfium.googlesource.com/pdfium/'
        },
        {
            name: 'SQLite',
            description: 'Local embedded database for the library and settings.',
            url: 'https://www.sqlite.org/'
        },
        {
            name: 'TimGM6mb SoundFont',
            description: 'Sample bank used by rustysynth for MIDI synthesis and audio previews.',
            url: 'https://musescore.org/en/handbook/soundfonts-and-sfz-files'
        }
    ]
};
