import { initializeMotionPreference } from './core/store/motionPreference';
import { render } from 'solid-js/web';
import { ShutdownScreen } from './components/features/shutdown/ShutdownScreen';
import { hasShutdownTestFault } from './core/shutdown/testFaults';

initializeMotionPreference();

const root = document.getElementById('root');
if (root && !hasShutdownTestFault('presentation-unavailable'))
    render(() => <ShutdownScreen />, root);
