import { initializeMotionPreference } from './core/store/motionPreference';
import { initializeApplication } from './core/startup/initializeApplication';

initializeMotionPreference();

void initializeApplication();
