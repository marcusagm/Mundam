import { TranscodeQuality } from '../../../lib/stream-utils';

export type QualityOption = {
    id: TranscodeQuality;
    label: string;
};

export const QUALITY_OPTIONS: QualityOption[] = [
    { id: 'preview', label: 'Preview' },
    { id: 'standard', label: 'Standard' },
    { id: 'high', label: 'High' }
];

export interface VideoPlayerProps {
    src: string;
    variant?: 'full' | 'compact';
    autoPlay?: boolean;
    title?: string;
    quality?: TranscodeQuality;
    onQualityChange?: (quality: TranscodeQuality) => void;
    onEnded?: () => void;
    onError?: (error: string) => void;
    /** Show quality selector button (default: true for transcoded videos) */
    showQualitySelector?: boolean;
    class?: string;
    /** Forced duration in seconds (useful for HLS where metadata might report Infinity) */
    forcedDuration?: number;
}
