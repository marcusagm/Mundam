import { Component, createSignal } from "solid-js";
import "./renderers.css";

interface VideoPlayerProps {
    src: string;
    type?: string; 
}

export const VideoPlayer: Component<VideoPlayerProps> = (props) => {
    let videoRef: HTMLVideoElement | undefined;
    const [error, setError] = createSignal<string | null>(null);

    return (
        <div class="video-player-container">
            {error() ? (
                <div class="video-error">{error()}</div>
            ) : props.type === 'audio' ? (
                 <audio 
                    src={props.src} 
                    controls 
                    autoplay
                    class="audio-element"
                    onError={() => setError("Failed to load audio")}
                >
                    Your browser does not support the audio tag.
                </audio>
            ) : (
                <video 
                    ref={videoRef}
                    src={props.src} 
                    controls 
                    autoplay
                    class="video-element"
                    onError={() => setError("Failed to load video")}
                >
                    Your browser does not support the video tag.
                </video>
            )}
        </div>
    );
};
