import { createContext, useContext, JSX } from 'solid-js';
import { useVideoPlayer } from './useVideoPlayer';
import { VideoPlayerProps } from './types';

type VideoContextValue = ReturnType<typeof useVideoPlayer> & {
    props: VideoPlayerProps;
};

const VideoPlayerContext = createContext<VideoContextValue>();

export const VideoProvider = (props: { children: JSX.Element; playerProps: VideoPlayerProps }) => {
    const logic = useVideoPlayer(props.playerProps);

    return (
        <VideoPlayerContext.Provider value={{ ...logic, props: props.playerProps }}>
            {props.children}
        </VideoPlayerContext.Provider>
    );
};

export const useVideoContext = () => {
    const context = useContext(VideoPlayerContext);
    if (!context) {
        throw new Error('useVideoContext must be used within VideoProvider');
    }
    return context;
};
