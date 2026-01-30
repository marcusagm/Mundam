/**
 * createGesture Primitive
 * Subscribe to gesture events
 */

import { createEffect, onCleanup } from 'solid-js';
import { onCommand } from '../dispatcher';
import type { GesturePayload, CreateGestureOptions } from '../types';

/**
 * Subscribe to a specific gesture type
 * 
 * @example
 * createGesture({
 *   type: 'pinch',
 *   handler: (payload) => {
 *     if (payload.meta.final) {
 *       console.log('Pinch ended with scale:', payload.meta.scale);
 *     }
 *   },
 * });
 */
export function createGesture(options: CreateGestureOptions): void {
  const commandName = `gesture:${options.type}`;
  
  createEffect(() => {
    // Check if enabled
    if (options.enabled && !options.enabled()) {
      return;
    }
    
    const unsub = onCommand(commandName, (payload) => {
      // Check scope if specified
      // Note: This is handled by the dispatcher, but we could add extra filtering here
      
      options.handler(payload as unknown as GesturePayload);
    });
    
    onCleanup(unsub);
  });
}

/**
 * Subscribe to pinch gestures
 * 
 * @example
 * usePinchGesture((scale, isFinal) => {
 *   setZoom(prev => prev * scale);
 * });
 */
export function usePinchGesture(
  handler: (scale: number, isFinal: boolean, payload: GesturePayload) => void,
  enabled?: () => boolean
): void {
  createGesture({
    type: 'pinch',
    handler: (payload) => {
      handler(
        payload.meta.scale ?? 1,
        payload.meta.final ?? false,
        payload
      );
    },
    enabled,
  });
}

/**
 * Subscribe to swipe gestures
 * 
 * @example
 * useSwipeGesture((direction, fingers) => {
 *   if (direction === 'left') navigateNext();
 *   if (direction === 'right') navigatePrev();
 * });
 */
export function useSwipeGesture(
  handler: (direction: string, fingers: number, payload: GesturePayload) => void,
  enabled?: () => boolean
): void {
  createGesture({
    type: 'swipe',
    handler: (payload) => {
      handler(
        payload.meta.direction ?? 'right',
        payload.meta.fingers ?? 1,
        payload
      );
    },
    enabled,
  });
}

/**
 * Subscribe to rotate gestures
 * 
 * @example
 * useRotateGesture((angle, isFinal) => {
 *   setRotation(prev => prev + angle);
 * });
 */
export function useRotateGesture(
  handler: (angle: number, isFinal: boolean, payload: GesturePayload) => void,
  enabled?: () => boolean
): void {
  createGesture({
    type: 'rotate',
    handler: (payload) => {
      handler(
        payload.meta.angle ?? 0,
        payload.meta.final ?? false,
        payload
      );
    },
    enabled,
  });
}
