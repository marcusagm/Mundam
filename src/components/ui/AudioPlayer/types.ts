export interface AudioPlayerProps {
    src: string;
    filePath?: string; // Original system path for waveform extraction
    variant?: 'full' | 'compact';
    autoPlay?: boolean;
    title?: string;
    subtitle?: string;
    onEnded?: () => void;
    onError?: (error: string) => void;
    class?: string;
}
