import { Component, createMemo } from 'solid-js';
import { AudioPlayer as UIAudioPlayer } from '../../../../ui';
import { useViewport, useLibrary } from '../../../../../core/hooks';
import { useAudioSource } from '../../../../../core/hooks/useAudioSource';
import '../renderers.css';

interface AudioRendererProps {
    /** Full file path */
    path: string;
}

export const AudioRenderer: Component<AudioRendererProps> = props => {
    const viewport = useViewport();
    const lib = useLibrary();

    const { audioUrl } = useAudioSource(() => props.path);

    const item = createMemo(() =>
        lib.items.find((i: any) => i.id.toString() === viewport.activeItemId())
    );

    return (
        <div class="audio-renderer-container">
            <UIAudioPlayer
                src={audioUrl()}
                filePath={props.path}
                variant="full"
                title={item()?.filename}
                subtitle={item()?.format}
                class="audio-renderer-player"
            />
        </div>
    );
};
