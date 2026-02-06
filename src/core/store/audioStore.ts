import { createSignal } from 'solid-js';

const savedVolume = localStorage.getItem('mundam-audio-volume');
const initialVolume = savedVolume ? parseFloat(savedVolume) : 1;

const [volume, setVolume] = createSignal(initialVolume);
const [isMuted, setIsMuted] = createSignal(false);
const [isLooping, setIsLooping] = createSignal(false);

export const audioState = {
    volume,
    isMuted,
    isLooping
};

export const audioActions = {
    setVolume: (v: number) => {
        setVolume(v);
        localStorage.setItem('mundam-audio-volume', v.toString());
    },
    setIsMuted: (muted: boolean) => {
        setIsMuted(muted);
    },
    setIsLooping: (loop: boolean) => {
        setIsLooping(loop);
    }
};
