/**
 * Keyboard Provider
 * Attaches keyboard event listeners and normalizes events to tokens
 */

import { createKeyboardToken } from '../normalizer';
import { dispatchToken, handleKeyUp } from '../dispatcher';
import { inputStore } from '../store/inputStore';

// =============================================================================
// Provider State
// =============================================================================

let attached = false;

// =============================================================================
// Event Handlers
// =============================================================================

function onKeyDown(event: KeyboardEvent): void {
  if (!inputStore.enabled()) return;
  
  // Create normalized token
  const token = createKeyboardToken(event);
  
  // Dispatch to shortcut system
  dispatchToken(token, event);
}

function onKeyUp(event: KeyboardEvent): void {
  if (!inputStore.enabled()) return;
  
  // Extract key ID for release tracking
  const keyId = event.code || event.key;
  handleKeyUp(keyId);
}

function onVisibilityChange(): void {
  // Clear pressed keys when window becomes hidden
  if (typeof document !== 'undefined' && document.visibilityState === 'hidden') {
    inputStore.clearPressed();
  }
}

function onWindowBlur(): void {
  // Clear pressed keys when window loses focus
  inputStore.clearPressed();
}

// =============================================================================
// Provider Creation
// =============================================================================

/**
 * Create and attach the keyboard provider
 * Returns a cleanup function that should be called on unmount
 */
export function createKeyboardProvider(): () => void {
  if (attached) {
    console.warn('[KeyboardProvider] Already attached');
    return () => {};
  }
  
  if (typeof document === 'undefined') {
    return () => {};
  }
  
  // Attach listeners
  document.addEventListener('keydown', onKeyDown, { passive: false });
  document.addEventListener('keyup', onKeyUp, { passive: true });
  document.addEventListener('visibilitychange', onVisibilityChange, { passive: true });
  window.addEventListener('blur', onWindowBlur, { passive: true });
  
  attached = true;
  
  // Return cleanup function
  const cleanup = () => {
    document.removeEventListener('keydown', onKeyDown);
    document.removeEventListener('keyup', onKeyUp);
    document.removeEventListener('visibilitychange', onVisibilityChange);
    window.removeEventListener('blur', onWindowBlur);
    attached = false;
  };
  
  return cleanup;
}

/**
 * Check if keyboard provider is attached
 */
export function isKeyboardProviderAttached(): boolean {
  return attached;
}

