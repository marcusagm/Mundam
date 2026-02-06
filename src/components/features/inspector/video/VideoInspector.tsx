import { Component } from 'solid-js';
import { type ImageItem } from '../../../../types';
import { Accordion, VideoPlayer as UIVideoPlayer } from '../../../ui';
import { InspectorTags } from '../base/InspectorTags';
import { CommonMetadata } from '../base/CommonMetadata';
import { getVideoUrl } from '../../../../lib/stream-utils';
import './VideoInspector.css';

interface VideoInspectorProps {
    item: ImageItem;
}

export const VideoInspector: Component<VideoInspectorProps> = props => {
    const assetUrl = () => getVideoUrl(props.item.path);

    return (
        <div class="inspector-content">
            <div class="inspector-preview video-preview">
                <UIVideoPlayer
                    src={assetUrl()}
                    variant="compact"
                    class="inspector-video-player-ui"
                />
            </div>

            <Accordion>
                <CommonMetadata item={props.item} />
                <InspectorTags itemId={props.item.id} />
            </Accordion>
        </div>
    );
};
