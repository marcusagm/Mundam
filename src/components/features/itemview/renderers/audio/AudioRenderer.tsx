import { Component, createMemo } from 'solid-js';
import { AudioPlayer as UIAudioPlayer } from '../../../../ui';
import { useViewport, useLibrary } from '../../../../../core/hooks';
import '../renderers.css';

interface AudioRendererProps {
    src: string;
}

export const AudioRenderer: Component<AudioRendererProps> = props => {
    const viewport = useViewport();
    const lib = useLibrary();

    const item = createMemo(() =>
        lib.items.find((i: any) => i.id.toString() === viewport.activeItemId())
    );

    return (
        <div class="audio-renderer-container">
            {/* Remove the blocking loader overlay, let AudioPlayer handle it */}
            <UIAudioPlayer
                src={props.src}
                filePath={item()?.path}
                variant="full"
                // autoPlay
                title={item()?.filename}
                subtitle={item()?.format}
                class="audio-renderer-player"
            />
        </div>
    );
};
