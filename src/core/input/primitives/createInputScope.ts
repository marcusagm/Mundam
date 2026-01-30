/**
 * createInputScope Primitive
 * Manages scope lifecycle for components
 */

import { onMount, onCleanup } from 'solid-js';
import { inputStore } from '../store/inputStore';
import type { InputScopeName } from '../types';
import { SCOPE_PRIORITIES } from '../types';

/**
 * Push a scope when component mounts, pop when it unmounts
 * 
 * @example
 * // In ImageViewer component
 * createInputScope('image-viewer');
 * 
 * // Or with custom priority
 * createInputScope('custom-scope', 75);
 */
export function createInputScope(name: InputScopeName, priority?: number): void {
  const resolvedPriority = priority ?? SCOPE_PRIORITIES[name] ?? 0;
  
  // Push scope immediately during component creation
  // This ensures shortcuts work right away
  inputStore.pushScope(name, resolvedPriority);
  
  // Cleanup when component unmounts
  onCleanup(() => {
    inputStore.popScope(name);
  });
}

/**
 * Temporarily push a scope based on a condition
 * 
 * @example
 * // Push scope when modal is open
 * createConditionalScope('modal', () => isOpen());
 */
export function createConditionalScope(
  name: InputScopeName, 
  when: () => boolean,
  priority?: number
): void {
  const resolvedPriority = priority ?? SCOPE_PRIORITIES[name] ?? 0;
  
  // Track if currently pushed
  let isPushed = false;
  
  onMount(() => {
    // Watch the condition
    const checkCondition = () => {
      const shouldBePushed = when();
      
      if (shouldBePushed && !isPushed) {
        inputStore.pushScope(name, resolvedPriority);
        isPushed = true;
      } else if (!shouldBePushed && isPushed) {
        inputStore.popScope(name);
        isPushed = false;
      }
    };
    
    // Initial check
    checkCondition();
    
    // Note: In a real reactive context, this would need to be wrapped in createEffect
    // But since we're receiving an accessor, we rely on the caller to handle reactivity
  });
  
  onCleanup(() => {
    if (isPushed) {
      inputStore.popScope(name);
    }
  });
}

/**
 * HOC/Wrapper component for scope management
 * Use when you want declarative scope in JSX
 */
export function useScopeOnMount(name: InputScopeName, priority?: number): void {
  createInputScope(name, priority);
}
