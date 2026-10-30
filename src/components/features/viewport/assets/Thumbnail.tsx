import { createMemo, Show, type JSX } from 'solid-js';
import { Loader } from '../../../ui/Loader';
import { FileIcon } from './FileIcon';
import { useThumbnail } from './useThumbnail';
import type { ThumbnailProperties } from './thumbnailTypes';
import './thumbnail.css';
import './file-icon.css';

export type { ThumbnailProperties } from './thumbnailTypes';

/**
 * Displays a thumbnail or a file icon using real worker outcomes instead of AssetState.
 *
 * @param thumbnailProperties - Reactive asset and image properties.
 * @returns The generated image, a pending indicator, or an explained file fallback.
 */
export function Thumbnail(thumbnailProperties: ThumbnailProperties): JSX.Element {
    const thumbnail = useThumbnail(thumbnailProperties);
    const aspectRatio = createMemo(() =>
        thumbnailProperties.width && thumbnailProperties.height
            ? `${thumbnailProperties.width} / ${thumbnailProperties.height}`
            : undefined
    );
    return (
        <div
            class="thumbnail-container"
            style={{ 'aspect-ratio': aspectRatio() }}
            data-id={thumbnailProperties.id}
            data-thumbnail-state={thumbnail.snapshot()?.state || 'loading'}
            aria-busy={thumbnail.spinner()}
            title={thumbnail.label()}
        >
            <Show when={thumbnail.fallback()}>
                <div
                    class="thumbnail-file-fallback"
                    role="img"
                    aria-label={thumbnail.label() || 'Thumbnail unavailable'}
                >
                    <FileIcon
                        extension={thumbnail.extension()}
                        mediaType={thumbnailProperties.mediaType || 'Unknown'}
                    />
                </div>
            </Show>
            <Show when={thumbnail.spinner()}>
                <div class="asset-placeholder">
                    <Loader size="sm" />
                </div>
            </Show>
            <Show when={thumbnail.source()}>
                <img
                    src={thumbnail.source()}
                    alt={thumbnailProperties.alt}
                    draggable={false}
                    onLoad={thumbnail.handleLoad}
                    onError={thumbnail.handleError}
                    class={thumbnail.isLoaded() ? 'loaded' : 'loading'}
                />
            </Show>
        </div>
    );
}
