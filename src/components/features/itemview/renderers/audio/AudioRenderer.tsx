import { Component, createEffect, createMemo } from 'solid-js';
import { AudioPlayer as UIAudioPlayer } from '../../../../ui';
import { useViewport, useLibrary } from '../../../../../core/hooks';
import { useAudioSource } from '../../../../../core/hooks/useAudioSource';
import '../renderers.css';

interface AudioRendererProps {
    /** Asset ID */
    assetId: string;
    /** Full file path */
    path: string;
    /** Called once the audio source is resolved */
    onReady?: () => void;
}

export const AudioRenderer: Component<AudioRendererProps> = props => {
    const viewport = useViewport();
    const lib = useLibrary();

    const { audioUrl } = useAudioSource(
        () => props.assetId,
        () => props.path
    );

    createEffect(() => {
        if (audioUrl()) props.onReady?.();
    });

    const item = createMemo(() =>
        lib.items.find(libraryItem => libraryItem.id.toString() === viewport.activeItemId())
    );

    return (
        <div class="audio-renderer-container">
            <UIAudioPlayer
                src={audioUrl()}
                filePath={props.path}
                assetId={props.assetId}
                variant="full"
                title={item()?.filename}
                subtitle={item()?.format}
                class="audio-renderer-player"
            />
        </div>
    );
};
