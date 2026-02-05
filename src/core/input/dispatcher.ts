/**
 * Shortcut Dispatcher
 * Matches input tokens against registered shortcuts and dispatches actions
 */

import type { 
  InputToken, 
  RegisteredShortcut, 
  ShortcutPayload,
} from './types';
import { inputStore } from './store/inputStore';
import { shortcutStore } from './store/shortcutStore';
import { tokensEqual } from './normalizer';

// =============================================================================
// Focus Detection
// =============================================================================

const INPUT_ELEMENTS = ['INPUT', 'TEXTAREA', 'SELECT'];

function isInputFocused(target?: EventTarget | null): boolean {
  if (typeof document === 'undefined') return false;
  
  // If target is provided and is an input, it's focused (or was when event fired)
  if (target instanceof HTMLElement) {
    if (INPUT_ELEMENTS.includes(target.tagName)) return true;
    if (target.getAttribute('contenteditable') === 'true') return true;
  }

  const active = document.activeElement;
  if (!active || active === document.body) return false;
  
  // If active element is no longer in the document, ignore it
  if (!document.contains(active)) return false;
  
  // Check tag name
  if (INPUT_ELEMENTS.includes(active.tagName)) {
    return true;
  }
  
  // Check contenteditable
  if (active.getAttribute('contenteditable') === 'true') {
    return true;
  }
  
  return false;
}

// =============================================================================
// Shortcut Matching
// =============================================================================

interface MatchResult {
  shortcut: RegisteredShortcut;
  matchType: 'single' | 'sequence' | 'chord';
}

/**
 * Find matching shortcuts for the current input state
 */
function findMatches(token: InputToken): MatchResult[] {
  // Determine active scopes and blocking threshold
  const scopeStack = inputStore.scopeStack();
  const activeScopeNames = scopeStack.map(s => s.name);
  const sequenceBuffer = inputStore.sequenceBuffer();
  const allShortcuts = shortcutStore.list();
  
  // Calculate priority threshold from blocking scopes
  // Any scope with blockLowerScopes=true will block shortcuts from scopes with lower priority
  let cutoffPriority = -Infinity;
  for (const scope of scopeStack) {
    if (scope.blockLowerScopes && scope.priority > cutoffPriority) {
      cutoffPriority = scope.priority;
    }
  }
  
  const candidates: RegisteredShortcut[] = [];
  
  // Filter by scope and blocking
  for (const shortcut of allShortcuts) {
    // 1. Check if scope is active
    if (shortcut.scope && !activeScopeNames.includes(shortcut.scope)) {
      continue;
    }
    
    // 2. Check scope blocking
    // Resolve shortcut's effective scope priority
    // Global shortcuts have priority 0 (SCOPE_PRIORITIES.global)
    let shortcutScopePriority = 0;
    if (shortcut.scope) {
      const scopeDef = scopeStack.find(s => s.name === shortcut.scope);
      if (scopeDef) {
        shortcutScopePriority = scopeDef.priority;
      }
    }
    
    // If shortcut's scope priority is lower than the blocking threshold, ignore it
    // EXCEPT for shortcuts with modifiers (Meta, Alt, Ctrl) which should typically 
    // bypass input-level blocking unless they are in the blocking scope themselves.
    const hasModifiers = shortcut.tokens.some(t => {
      const mods = t.meta?.modifiers;
      return Array.isArray(mods) && mods.length > 0;
    });
    
    if (shortcutScopePriority < cutoffPriority && !hasModifiers) {
      continue;
    }
    
    // Check enabledWhen condition
    if (shortcut.enabledWhen) {
      try {
        if (!shortcut.enabledWhen()) continue;
      } catch {
        continue;
      }
    }
    
    candidates.push(shortcut);
  }
  
  const matches: MatchResult[] = [];
  
  for (const shortcut of candidates) {
    const tokens = shortcut.tokens;
    
    if (tokens.length === 0) continue;
    
    // Single key match
    if (tokens.length === 1) {
      if (tokensEqual(tokens[0], token)) {
        matches.push({ shortcut, matchType: 'single' });
        continue;
      }
    }
    
    // Sequence match (e.g., "g g" for go)
    if (tokens.length > 1) {
      // Check if sequence buffer ends with this sequence
      const bufferWithCurrent = [...sequenceBuffer, token];
      const tail = bufferWithCurrent.slice(-tokens.length);
      
      if (tail.length === tokens.length) {
        let sequenceMatch = true;
        for (let i = 0; i < tokens.length; i++) {
          if (!tokensEqual(tokens[i], tail[i])) {
            sequenceMatch = false;
            break;
          }
        }
        
        if (sequenceMatch) {
          matches.push({ shortcut, matchType: 'sequence' });
          continue;
        }
      }
    }
    
    // Chord match (multiple keys pressed simultaneously)
    // This requires all keys in the chord to be currently pressed
    // Not typically used for keyboard shortcuts, but supported
  }
  
  return matches;
}

/**
 * Sort matches by priority and specificity
 */
function sortMatches(matches: MatchResult[]): MatchResult[] {
  return matches.sort((a, b) => {
    // 1. Higher scope priority first (MOST IMPORTANT)
    const scopeA = inputStore.scopeStack().find(s => s.name === a.shortcut.scope);
    const scopeB = inputStore.scopeStack().find(s => s.name === b.shortcut.scope);
    const scopePriorityA = scopeA?.priority ?? 0;
    const scopePriorityB = scopeB?.priority ?? 0;
    if (scopePriorityB !== scopePriorityA) return scopePriorityB - scopePriorityA;
    
    // 2. Higher shortcut priority first
    const prioA = a.shortcut.priority ?? 0;
    const prioB = b.shortcut.priority ?? 0;
    if (prioB !== prioA) return prioB - prioA;

    // 3. Longer sequences first (Specificity)
    const lenA = a.shortcut.tokens.length;
    const lenB = b.shortcut.tokens.length;
    if (lenB !== lenA) return lenB - lenA;

    // 4. Non-default shortcuts first (User overrides within same scope/priority)
    const isDefaultA = a.shortcut.isDefault ?? true;
    const isDefaultB = b.shortcut.isDefault ?? true;
    if (isDefaultA !== isDefaultB) {
      return isDefaultA ? 1 : -1; // Non-default (false) comes first
    }
    
    return 0;
  });
}

// =============================================================================
// Dispatcher
// =============================================================================

/**
 * Handle an incoming token and dispatch matching shortcuts
 * Returns true if a shortcut was dispatched
 */
export function dispatchToken(token: InputToken, event: Event | null): boolean {
  if (!inputStore.enabled()) {
    return false;
  }
  
  // Update pressed keys state
  if (token.kind === 'keyboard') {
    inputStore.keyDown(token);
  }
  
  // Check if we should ignore due to input focus
  // Use event target if available for more robust focus detection during blur transitions
  const inputFocused = isInputFocused(event?.target);
  
  // Find matches
  const matches = findMatches(token);
  
  if (matches.length === 0) {
    return false;
  }
  
  // Sort by priority
  const sorted = sortMatches(matches);
  
  // Find first match that should fire
  for (const match of sorted) {
    const { shortcut } = match;
    
    // Check ignoreInputs flag
    if (shortcut.ignoreInputs && inputFocused) {
      // Special case: Escape always works for blur
      if (token.id !== 'Escape') {
        continue;
      }
    }
    
    // Check if chord was already dispatched (for held keys)
    if (match.matchType === 'chord') {
      if (inputStore.isChordDispatched(shortcut.id)) {
        continue;
      }
      inputStore.markChordDispatched(shortcut.id);
    }
    
    // Dispatch!
    try {
      // Prevent default if configured
      if (shortcut.preventDefault && event && typeof (event as any).preventDefault === 'function') {
        (event as any).preventDefault();
      }
      
      const payload: ShortcutPayload = {
        shortcutDef: shortcut,
        sequence: inputStore.sequenceBuffer(),
        meta: token.meta || {},
      };
      
      // Call handler if exists
      if (shortcut.handler) {
        shortcut.handler(event, payload);
      }
      
      // Emit command event if configured
      if (shortcut.command) {
        emitCommand(shortcut.command, payload);
      }
      
      // Clear sequence buffer on successful match
      if (match.matchType === 'sequence') {
        inputStore.clearSequenceBuffer();
      }
      
      return true;
      
    } catch (err) {
      console.error(`[InputDispatcher] Error dispatching shortcut ${shortcut.id}:`, err);
    }
  }
  
  return false;
}

/**
 * Handle key up event
 */
export function handleKeyUp(keyId: string): void {
  inputStore.keyUp(keyId);
}

// =============================================================================
// Command Event System
// =============================================================================

type CommandHandler = (payload: ShortcutPayload) => void;
const commandHandlers = new Map<string, Set<CommandHandler>>();

/**
 * Subscribe to a command
 */
export function onCommand(command: string, handler: CommandHandler): () => void {
  if (!commandHandlers.has(command)) {
    commandHandlers.set(command, new Set());
  }
  
  commandHandlers.get(command)!.add(handler);
  
  // Return unsubscribe function
  return () => {
    commandHandlers.get(command)?.delete(handler);
  };
}

/**
 * Emit a command event
 */
function emitCommand(command: string, payload: ShortcutPayload): void {
  const handlers = commandHandlers.get(command);
  if (!handlers) return;
  
  for (const handler of handlers) {
    try {
      handler(payload);
    } catch (err) {
      console.error(`[InputDispatcher] Error in command handler for ${command}:`, err);
    }
  }
}

/**
 * Clear all command handlers (for cleanup)
 */
export function clearCommandHandlers(): void {
  commandHandlers.clear();
}
