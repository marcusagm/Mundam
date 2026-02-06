import { Component } from 'solid-js';
import { VideoPlayer as UIVideoPlayer } from '../../../../ui';
import '../renderers.css';

interface VideoPlayerProps {
    src: string;
    type?: string;
}

export const VideoPlayer: Component<VideoPlayerProps> = props => {
    return (
        <div class="video-player-container">
            <UIVideoPlayer src={props.src} variant="full" autoPlay class="video-renderer-player" />
        </div>
    );
};
