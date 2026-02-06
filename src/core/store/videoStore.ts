import { createSignal } from 'solid-js';

const savedVolume = localStorage.getItem('mundam-video-volume');
const initialVolume = savedVolume ? parseFloat(savedVolume) : 1;

const [volume, setVolume] = createSignal(initialVolume);
const [isMuted, setIsMuted] = createSignal(false);
const [isLooping, setIsLooping] = createSignal(false);
const [playbackRate, setPlaybackRate] = createSignal(1);

export const videoState = {
    volume,
    isMuted,
    isLooping,
    playbackRate
};

export const videoActions = {
    setVolume: (v: number) => {
        setVolume(v);
        localStorage.setItem('mundam-video-volume', v.toString());
    },
    setIsMuted: (muted: boolean) => {
        setIsMuted(muted);
    },
    setIsLooping: (loop: boolean) => {
        setIsLooping(loop);
    },
    setPlaybackRate: (rate: number) => {
        setPlaybackRate(rate);
    }
};
