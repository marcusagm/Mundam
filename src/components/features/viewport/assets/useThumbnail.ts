import { batch, createEffect, createMemo, createSignal, on, onCleanup, untrack } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import { getAssetUrl } from '../../../../core/utils/assetUrl';
import {
    clearPendingRegeneration,
    getCompletedThumbnail,
    isPendingRegeneration,
    markPendingRegeneration,
    subscribeThumbnailReady
} from '../../../../core/store/thumbnailStore';
import {
    invalidateThumbnailProcessing,
    subscribeThumbnailProcessing,
    type ThumbnailProcessingSnapshot
} from '../../../../core/store/thumbnailProcessing';
import type { ThumbnailProperties } from './thumbnailTypes';

const loadedUrls = new Set<string>();
const repairRequests = new Set<string>();
const CACHE_LIMIT = 1000;

function remember(cache: Set<string>, key: string): void {
    cache.add(key);
    while (cache.size > CACHE_LIMIT) {
        const oldest = cache.values().next().value;
        if (oldest !== undefined) cache.delete(oldest);
    }
}

function stateLabel(snapshot: ThumbnailProcessingSnapshot | null): string | undefined {
    switch (snapshot?.state) {
        case 'failed':
            return 'Thumbnail generation failed. Automatic retries are exhausted.';
        case 'skipped':
            return 'No thumbnail extractor is available for this file.';
        case 'retry_wait':
            return 'Thumbnail generation failed. A retry is scheduled.';
        case 'unavailable':
            return 'Thumbnail source is unavailable.';
        case 'unknown':
            return 'Thumbnail processing status is unavailable.';
        default:
            return undefined;
    }
}

function shouldRefreshImage(
    previousState: string | undefined,
    status: ThumbnailProcessingSnapshot,
    ready: boolean,
    regenerating: boolean
): boolean {
    return (
        status.state === 'succeeded' &&
        !ready &&
        (regenerating || ['pending', 'retry_wait'].includes(previousState || ''))
    );
}

/**
 * Renders authoritative outcomes while keeping cache repair bounded across virtualization.
 * @param properties - Reactive properties; asset identity may change without unmounting.
 * @returns Accessors and handlers used by the presentation component.
 */
export function useThumbnail(properties: ThumbnailProperties) {
    const [snapshot, setSnapshot] = createSignal<ThumbnailProcessingSnapshot | null>(null);
    const [localThumbnail, setLocalThumbnail] = createSignal<string | null>(null);
    const [localError, setLocalError] = createSignal(false);
    const [loaded, setLoaded] = createSignal(false);
    const [imageVersion, setImageVersion] = createSignal(0);
    const [generation, setGeneration] = createSignal(0);
    let observedGeneration: number | undefined;
    let observedState: string | undefined;
    let readyForAttempt = false;

    createEffect(
        on(
            () => properties.id,
            identifier => {
                batch(() => {
                    setSnapshot(null);
                    setLocalThumbnail(untrack(() => getCompletedThumbnail(identifier)) || null);
                    setLocalError(false);
                    setLoaded(false);
                    setImageVersion(0);
                    setGeneration(0);
                });
                observedGeneration = undefined;
                observedState = undefined;
                readyForAttempt = false;
                const stopStatus = subscribeThumbnailProcessing(identifier, status => {
                    if (properties.id !== identifier) return;
                    batch(() => {
                        setSnapshot(status);
                        if (!status) return;
                        setGeneration(status.generation);
                        const generationChanged =
                            observedGeneration !== undefined &&
                            observedGeneration !== status.generation;
                        if (generationChanged) {
                            setLocalError(false);
                            setLoaded(false);
                            readyForAttempt = false;
                        }
                        if (status.state === 'pending' && observedState !== 'pending')
                            readyForAttempt = false;
                        if (
                            shouldRefreshImage(
                                observedState,
                                status,
                                readyForAttempt,
                                isPendingRegeneration(identifier)
                            )
                        ) {
                            setLocalError(false);
                            setLoaded(false);
                            setImageVersion(version => version + 1);
                        }
                        if (['failed', 'skipped'].includes(status.state)) setLocalThumbnail(null);
                        if (
                            ['failed', 'skipped', 'succeeded', 'unavailable'].includes(status.state)
                        ) {
                            clearPendingRegeneration(identifier);
                        }
                        observedGeneration = status.generation;
                        observedState = status.state;
                    });
                });
                const stopReady = subscribeThumbnailReady(identifier, (_identifier, path) => {
                    if (properties.id !== identifier || !path) return;
                    readyForAttempt = true;
                    batch(() => {
                        setLocalThumbnail(path);
                        setLocalError(false);
                        setLoaded(false);
                        setImageVersion(version => version + 1);
                    });
                });
                onCleanup(() => {
                    stopStatus();
                    stopReady();
                });
            }
        )
    );

    createEffect(() => {
        if (
            localError() &&
            snapshot()?.state === 'succeeded' &&
            !isPendingRegeneration(properties.id)
        ) {
            const identifier = properties.id;
            if (repairRequests.has(identifier)) return;
            remember(repairRequests, identifier);
            markPendingRegeneration(identifier);
            setLocalThumbnail(null);
            readyForAttempt = false;
            void invoke('request_thumbnail_regenerate', { assetId: identifier })
                .then(() => invalidateThumbnailProcessing(identifier))
                .catch(error => {
                    clearPendingRegeneration(identifier);
                    invalidateThumbnailProcessing(identifier);
                    console.warn('Thumbnail cache repair was rejected:', error);
                });
        }
    });

    const output = createMemo(
        () => localThumbnail() || snapshot()?.thumbnail_path || properties.thumbnail
    );
    const waitingForOutput = createMemo(
        () =>
            isPendingRegeneration(properties.id) ||
            ['pending', 'retry_wait', 'unknown'].includes(snapshot()?.state || 'pending')
    );
    const fallback = createMemo(() => {
        const status = snapshot()?.state;
        if (['failed', 'skipped'].includes(status || '')) return true;
        if (localError()) return !waitingForOutput();
        if (output()) return false;
        if (status === 'unavailable') return true;
        return ['offline', 'trashed'].includes(properties.state?.toLowerCase() || '');
    });
    const source = createMemo(() => {
        if (fallback() || localError() || !output() || isPendingRegeneration(properties.id))
            return undefined;
        return `${getAssetUrl(properties.id, 'thumb')}&generation=${generation()}&refresh=${imageVersion()}`;
    });
    const isLoaded = createMemo(() => loaded() || loadedUrls.has(source() || ''));
    const spinner = createMemo(() => {
        if (fallback()) return false;
        if (isPendingRegeneration(properties.id)) return true;
        if (!output() || localError()) return waitingForOutput();
        return Boolean(source()) && !isLoaded();
    });
    const label = createMemo(
        () =>
            stateLabel(snapshot()) || (localError() ? 'Thumbnail image is unavailable.' : undefined)
    );
    const extension = createMemo(() => {
        const dot = properties.alt.lastIndexOf('.');
        return dot < 0 ? '' : properties.alt.slice(dot).toLowerCase();
    });

    return {
        /** Image protocol URL with generation-aware cache identity. */
        source,
        /** Whether a real pending request or image load is in progress. */
        spinner,
        /** File icon state for absent or failed output. */
        fallback,
        /** Accessible explanation of extraction or transport failure. */
        label,
        /** Current persisted worker status, independent of AssetState. */
        snapshot,
        /** Cached image load state. */
        isLoaded,
        /** File extension for the fallback icon. */
        extension,
        /** Records successful image loads across virtualized mounts. */
        handleLoad: (): void => {
            const url = source();
            if (url) remember(loadedUrls, url);
            setLoaded(true);
        },
        /** Records image failure; only confirmed successful output can request cache repair. */
        handleError: (): void => {
            const url = source();
            if (url) loadedUrls.delete(url);
            setLocalError(true);
            setLoaded(false);
        }
    };
}
