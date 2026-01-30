/**
 * Input Store
 * Reactive state for input system (pressed keys, active scopes, etc.)
 */

import { createSignal } from 'solid-js';
import type { 
  InputScope, 
  InputScopeName, 
  InputToken,
  InputActions,
} from '../types';
import { SCOPE_PRIORITIES } from '../types';

// =============================================================================
// State
// =============================================================================

function createInputStore() {
  // Using signals for fine-grained reactivity
  const [enabled, setEnabled] = createSignal(true);
  const [pressedKeys, setPressedKeys] = createSignal<Set<string>>(new Set());
  const [scopeStack, setScopeStack] = createSignal<InputScope[]>([
    { name: 'global', priority: SCOPE_PRIORITIES.global },
  ]);
  const [sequenceBuffer, setSequenceBuffer] = createSignal<InputToken[]>([]);
  const [dispatchedChords, setDispatchedChords] = createSignal<Set<string>>(new Set());
  
  // Sequence timeout timer
  let sequenceTimerId: number | null = null;
  const SEQUENCE_TIMEOUT = 800;
  
  // =============================================================================
  // Computed
  // =============================================================================
  
  /**
   * Get active scopes sorted by priority (highest first)
   */
  const activeScopes = () => {
    return [...scopeStack()].sort((a, b) => b.priority - a.priority);
  };
  
  /**
   * Get highest priority active scope
   */
  const currentScope = () => {
    const scopes = activeScopes();
    return scopes.length > 0 ? scopes[0] : null;
  };
  
  /**
   * Get array of active scope names
   */
  const activeScopeNames = () => {
    return scopeStack().map(s => s.name);
  };
  
  /**
   * Check if a key is currently pressed
   */
  const isKeyPressed = (keyId: string) => {
    return pressedKeys().has(keyId);
  };
  
  // =============================================================================
  // Actions
  // =============================================================================
  
  const actions: InputActions = {
    enable: () => setEnabled(true),
    
    disable: () => {
      setEnabled(false);
      // Clear all transient state
      setPressedKeys(new Set<string>());
      setSequenceBuffer([]);
      setDispatchedChords(new Set<string>());
      if (sequenceTimerId) {
        clearTimeout(sequenceTimerId);
        sequenceTimerId = null;
      }
    },
    
    pushScope: (name: InputScopeName, priority?: number) => {
      const resolvedPriority = priority ?? SCOPE_PRIORITIES[name] ?? 0;
      
      setScopeStack(prev => {
        // Don't add duplicate scopes
        if (prev.some(s => s.name === name)) {
          return prev;
        }
        return [...prev, { name, priority: resolvedPriority }];
      });
    },
    
    popScope: (name: InputScopeName) => {
      setScopeStack(prev => prev.filter(s => s.name !== name));
    },
    
    keyDown: (token: InputToken) => {
      if (!enabled()) return;
      
      // Add to pressed keys
      setPressedKeys(prev => {
        const next = new Set(prev);
        next.add(token.id);
        return next;
      });
      
      // Add to sequence buffer
      setSequenceBuffer(prev => {
        const next = [...prev, token];
        // Keep buffer manageable (max 10 tokens)
        if (next.length > 10) {
          next.shift();
        }
        return next;
      });
      
      // Reset sequence timeout
      if (sequenceTimerId) {
        clearTimeout(sequenceTimerId);
      }
      sequenceTimerId = window.setTimeout(() => {
        setSequenceBuffer([]);
        sequenceTimerId = null;
      }, SEQUENCE_TIMEOUT);
    },
    
    keyUp: (keyId: string) => {
      if (!enabled()) return;
      
      // Remove from pressed keys
      setPressedKeys(prev => {
        const next = new Set(prev);
        next.delete(keyId);
        return next;
      });
      
      // Clear dispatched chords cache
      setDispatchedChords(new Set<string>());
    },
    
    clearPressed: () => {
      setPressedKeys(new Set<string>());
      setDispatchedChords(new Set<string>());
    },
  };
  
  // =============================================================================
  // Internal helpers for dispatcher
  // =============================================================================
  
  const markChordDispatched = (shortcutId: string) => {
    setDispatchedChords(prev => {
      const next = new Set(prev);
      next.add(shortcutId);
      return next;
    });
  };
  
  const isChordDispatched = (shortcutId: string) => {
    return dispatchedChords().has(shortcutId);
  };
  
  const clearSequenceBuffer = () => {
    setSequenceBuffer([]);
    if (sequenceTimerId) {
      clearTimeout(sequenceTimerId);
      sequenceTimerId = null;
    }
  };
  
  return {
    // State accessors
    enabled,
    pressedKeys,
    scopeStack,
    sequenceBuffer,
    dispatchedChords,
    
    // Computed
    activeScopes,
    currentScope,
    activeScopeNames,
    isKeyPressed,
    
    // Actions
    ...actions,
    
    // Internal helpers
    markChordDispatched,
    isChordDispatched,
    clearSequenceBuffer,
  };
}

// =============================================================================
// Singleton Export
// =============================================================================

// Create store directly - signals work outside component context
export const inputStore = createInputStore();
