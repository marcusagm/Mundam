import { Component, createSignal, onCleanup, onMount } from "solid-js";

interface VideoPlayerProps {
    src: string;
    type?: string; 
}

export const VideoPlayer: Component<VideoPlayerProps> = (props) => {
    let videoRef: HTMLVideoElement | undefined;
    const [error, setError] = createSignal<string | null>(null);

    return (
        <div class="video-player-container" style={{
            "width": "100%",
            "height": "100%",
            "display": "flex",
            "align-items": "center",
            "justify-content": "center",
            "background": "#000"
        }}>
            {error() ? (
                <div class="video-error">{error()}</div>
            ) : props.type === 'audio' ? (
                 <audio 
                    src={props.src} 
                    controls 
                    autoplay
                    style={{ "max-width": "100%", "width": "500px" }}
                    onError={(e) => setError("Failed to load audio")}
                >
                    Your browser does not support the audio tag.
                </audio>
            ) : (
                <video 
                    ref={videoRef}
                    src={props.src} 
                    controls 
                    autoplay
                    style={{ "max-width": "100%", "max-height": "100%" }}
                    onError={(e) => setError("Failed to load video")}
                >
                    Your browser does not support the video tag.
                </video>
            )}
        </div>
    );
};
