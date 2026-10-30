import { render } from 'solid-js/web';
import { SplashScreen } from './components/features/startup/SplashScreen';

const root = document.getElementById('root');
if (root) render(() => <SplashScreen />, root);
