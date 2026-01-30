/**
 * Gesture Provider
 * Recognizes multi-touch gestures (swipe, pinch, rotate)
 */

import { createGestureToken } from '../normalizer';
import { dispatchToken } from '../dispatcher';
import { inputStore } from '../store/inputStore';
import type { GesturePayload, SwipeDirection } from '../types';

// =============================================================================
// Configuration
// =============================================================================

interface GestureConfig {
  throttleMs: number;
  minSwipeDistance: number;
}

const DEFAULT_CONFIG: GestureConfig = {
  throttleMs: 50,
  minSwipeDistance: 30,
};

// =============================================================================
// Provider State
// =============================================================================

let attached = false;
let config = { ...DEFAULT_CONFIG };
let lastGestureTs = 0;

// Touch tracking
interface TouchPoint {
  id: number;
  x: number;
  y: number;
}

interface TouchTracking {
  startTime: number;
  startTouches: TouchPoint[];
  lastTouches: TouchPoint[];
  moved: boolean;
}

interface PinchTracking {
  startDist: number;
  lastDist: number;
}

interface RotateTracking {
  startAngle: number;
  lastAngle: number;
}

let touchTracking: TouchTracking | null = null;
let pinchTracking: PinchTracking | null = null;
let rotateTracking: RotateTracking | null = null;

// Gesture callbacks
type GestureCallback = (payload: GesturePayload) => void;
let gestureCallback: GestureCallback | null = null;

// =============================================================================
// Helpers
// =============================================================================

function shouldDispatch(): boolean {
  if (config.throttleMs <= 0) return true;
  
  const now = Date.now();
  if (now - lastGestureTs >= config.throttleMs) {
    lastGestureTs = now;
    return true;
  }
  return false;
}

function computeAveragePoint(points: TouchPoint[]): { x: number; y: number } {
  let sumX = 0;
  let sumY = 0;
  for (const p of points) {
    sumX += p.x;
    sumY += p.y;
  }
  return { x: sumX / points.length, y: sumY / points.length };
}

function computeDistance(p1: TouchPoint, p2: TouchPoint): number {
  return Math.hypot(p2.x - p1.x, p2.y - p1.y);
}

function computeAngle(p1: TouchPoint, p2: TouchPoint): number {
  return Math.atan2(p2.y - p1.y, p2.x - p1.x);
}

function emitGesture(payload: GesturePayload): void {
  if (gestureCallback) {
    gestureCallback(payload);
  }
  
  // Also dispatch to shortcut system
  const token = createGestureToken(payload.gesture, payload.meta as Record<string, unknown>);
  dispatchToken(token, payload.event);
}

// =============================================================================
// Event Handlers
// =============================================================================

function onTouchStart(event: TouchEvent): void {
  if (!inputStore.enabled()) return;
  if (!event.touches || event.touches.length === 0) return;
  
  const now = Date.now();
  const touches: TouchPoint[] = Array.from(event.touches).map(t => ({
    id: t.identifier,
    x: t.clientX,
    y: t.clientY,
  }));
  
  touchTracking = {
    startTime: now,
    startTouches: touches,
    lastTouches: [...touches],
    moved: false,
  };
  
  // Initialize pinch/rotate tracking for 2-finger gestures
  if (touches.length === 2) {
    const dist = computeDistance(touches[0], touches[1]);
    const angle = computeAngle(touches[0], touches[1]);
    
    pinchTracking = { startDist: dist, lastDist: dist };
    rotateTracking = { startAngle: angle, lastAngle: angle };
  } else {
    pinchTracking = null;
    rotateTracking = null;
  }
}

function onTouchMove(event: TouchEvent): void {
  if (!inputStore.enabled()) return;
  if (!touchTracking || !event.touches) return;
  
  const touches: TouchPoint[] = Array.from(event.touches).map(t => ({
    id: t.identifier,
    x: t.clientX,
    y: t.clientY,
  }));
  
  touchTracking.lastTouches = touches;
  touchTracking.moved = true;
  
  // Handle 2-finger gestures
  if (touches.length === 2 && pinchTracking && shouldDispatch()) {
    const dist = computeDistance(touches[0], touches[1]);
    const scale = dist / (pinchTracking.startDist || 1);
    pinchTracking.lastDist = dist;
    
    const center = {
      x: (touches[0].x + touches[1].x) / 2,
      y: (touches[0].y + touches[1].y) / 2,
    };
    
    emitGesture({
      gesture: 'pinch',
      meta: { scale, center, incremental: true },
      event,
    });
  }
  
  if (touches.length === 2 && rotateTracking && shouldDispatch()) {
    const angle = computeAngle(touches[0], touches[1]);
    const deltaRad = angle - rotateTracking.startAngle;
    const deltaDeg = deltaRad * (180 / Math.PI);
    rotateTracking.lastAngle = angle;
    
    const center = {
      x: (touches[0].x + touches[1].x) / 2,
      y: (touches[0].y + touches[1].y) / 2,
    };
    
    emitGesture({
      gesture: 'rotate',
      meta: { angle: deltaDeg, center, incremental: true },
      event,
    });
  }
}

function onTouchEnd(event: TouchEvent): void {
  if (!inputStore.enabled()) return;
  if (!touchTracking) return;
  
  const data = touchTracking;
  const startTouches = data.startTouches;
  const lastTouches = data.lastTouches;
  const fingers = Math.min(startTouches.length, lastTouches.length);
  
  if (fingers <= 0) {
    touchTracking = null;
    return;
  }
  
  const startAvg = computeAveragePoint(startTouches);
  const endAvg = computeAveragePoint(lastTouches);
  const deltaX = endAvg.x - startAvg.x;
  const deltaY = endAvg.y - startAvg.y;
  const absDeltaX = Math.abs(deltaX);
  const absDeltaY = Math.abs(deltaY);
  
  // Check if it was a swipe
  if (data.moved && (absDeltaX >= config.minSwipeDistance || absDeltaY >= config.minSwipeDistance)) {
    let direction: SwipeDirection;
    if (absDeltaX >= absDeltaY) {
      direction = deltaX > 0 ? 'right' : 'left';
    } else {
      direction = deltaY > 0 ? 'down' : 'up';
    }
    
    emitGesture({
      gesture: 'swipe',
      meta: {
        fingers,
        direction,
        dx: deltaX,
        dy: deltaY,
        duration: Date.now() - data.startTime,
        final: true,
      },
      event,
    });
  }
  
  // Emit final pinch
  if (pinchTracking && pinchTracking.startDist) {
    const finalScale = pinchTracking.lastDist / pinchTracking.startDist;
    emitGesture({
      gesture: 'pinch',
      meta: { scale: finalScale, final: true },
      event,
    });
  }
  
  // Emit final rotate
  if (rotateTracking && typeof rotateTracking.lastAngle === 'number') {
    const deltaDeg = (rotateTracking.lastAngle - rotateTracking.startAngle) * (180 / Math.PI);
    emitGesture({
      gesture: 'rotate',
      meta: { angle: deltaDeg, final: true },
      event,
    });
  }
  
  // Reset tracking
  touchTracking = null;
  pinchTracking = null;
  rotateTracking = null;
}

// =============================================================================
// Provider Creation
// =============================================================================

export interface GestureProviderOptions {
  throttleMs?: number;
  minSwipeDistance?: number;
  onGesture?: GestureCallback;
}

/**
 * Create and attach the gesture provider
 */
export function createGestureProvider(options?: GestureProviderOptions): () => void {
  if (attached) {
    console.warn('[GestureProvider] Already attached');
    return () => {};
  }
  
  if (typeof window === 'undefined') {
    return () => {};
  }
  
  // Apply options
  if (options?.throttleMs !== undefined) {
    config.throttleMs = options.throttleMs;
  }
  if (options?.minSwipeDistance !== undefined) {
    config.minSwipeDistance = options.minSwipeDistance;
  }
  if (options?.onGesture) {
    gestureCallback = options.onGesture;
  }
  
  // Attach listeners
  window.addEventListener('touchstart', onTouchStart, { passive: true });
  window.addEventListener('touchmove', onTouchMove, { passive: false });
  window.addEventListener('touchend', onTouchEnd, { passive: true });
  
  attached = true;
  
  const cleanup = () => {
    window.removeEventListener('touchstart', onTouchStart);
    window.removeEventListener('touchmove', onTouchMove);
    window.removeEventListener('touchend', onTouchEnd);
    
    touchTracking = null;
    pinchTracking = null;
    rotateTracking = null;
    gestureCallback = null;
    config = { ...DEFAULT_CONFIG };
    attached = false;
  };
  
  return cleanup;
}

/**
 * Check if gesture provider is attached
 */
export function isGestureProviderAttached(): boolean {
  return attached;
}
