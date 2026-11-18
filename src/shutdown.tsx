import { render } from 'solid-js/web';
import { ShutdownScreen } from './components/features/shutdown/ShutdownScreen';
import { hasShutdownTestFault } from './core/shutdown/testFaults';

const root = document.getElementById('root');
if (root && !hasShutdownTestFault('presentation-unavailable'))
    render(() => <ShutdownScreen />, root);
