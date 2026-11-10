import { render } from 'solid-js/web';
import { ShutdownScreen } from './components/features/shutdown/ShutdownScreen';

const root = document.getElementById('root');
if (root) render(() => <ShutdownScreen />, root);
