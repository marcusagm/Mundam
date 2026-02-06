import { Component, createMemo, createSignal, createEffect, Show } from 'solid-js';
import { AudioPlayer as UIAudioPlayer, Loader } from '../../../../ui';
import { useViewport, useLibrary } from '../../../../../core/hooks';
import '../renderers.css';

interface AudioRendererProps {
    src: string;
}

export const AudioRenderer: Component<AudioRendererProps> = props => {
    const viewport = useViewport();
    const lib = useLibrary();
    const [loading, setLoading] = createSignal(true);

    const item = createMemo(() =>
        lib.items.find((i: any) => i.id.toString() === viewport.activeItemId())
    );

    // Reset loading when item changes
    createEffect(() => {
        item();
        setLoading(true);
        // We'll let the AudioPlayer's internal loading handle the rest,
        // but we add a small delay for the transition.
        setTimeout(() => setLoading(false), 500);
    });

    return (
        <div class="audio-renderer-container">
            <Show when={loading()}>
                <div class="item-switch-loader">
                    <Loader size="lg" text="Preparing audio..." />
                </div>
            </Show>
            <UIAudioPlayer
                src={props.src}
                filePath={item()?.path}
                variant="full"
                autoPlay
                title={item()?.filename}
                subtitle={item()?.format}
                class="audio-renderer-player"
            />
        </div>
    );
};
