/**
 * Pointer Provider
 * Handles mouse/pointer events and wheel events
 */

import { createPointerToken, createWheelToken } from '../normalizer';
import { dispatchToken } from '../dispatcher';
import { inputStore } from '../store/inputStore';

// =============================================================================
// Provider State
// =============================================================================

let attached = false;

// =============================================================================
// Event Handlers
// =============================================================================

function onPointerDown(event: PointerEvent): void {
  if (!inputStore.enabled()) return;
  
  // Only handle non-primary buttons or modified clicks
  // Primary click without modifiers is usually for normal interaction
  if (event.button === 0 && !event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey) {
    return;
  }
  
  const token = createPointerToken(event);
  dispatchToken(token, event);
}

function onWheel(event: WheelEvent): void {
  if (!inputStore.enabled()) return;
  
  const token = createWheelToken(event);
  dispatchToken(token, event);
}

// =============================================================================
// Provider Creation
// =============================================================================

/**
 * Create and attach the pointer provider
 */
export function createPointerProvider(): () => void {
  if (attached) {
    console.warn('[PointerProvider] Already attached');
    return () => {};
  }
  
  if (typeof document === 'undefined') {
    return () => {};
  }
  
  // Attach listeners
  document.addEventListener('pointerdown', onPointerDown, { passive: true });
  window.addEventListener('wheel', onWheel, { passive: false });
  
  attached = true;
  
  const cleanup = () => {
    document.removeEventListener('pointerdown', onPointerDown);
    window.removeEventListener('wheel', onWheel);
    attached = false;
  };
  
  return cleanup;
}

/**
 * Check if pointer provider is attached
 */
export function isPointerProviderAttached(): boolean {
  return attached;
}
