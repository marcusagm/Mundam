import { initializeMotionPreference } from './core/store/motionPreference';
import { render } from 'solid-js/web';
import { SplashScreen } from './components/features/startup/SplashScreen';

initializeMotionPreference();

const root = document.getElementById('root');
if (root) render(() => <SplashScreen />, root);
