/**
 * Shortcut Dispatcher
 * Matches input tokens against registered shortcuts and dispatches actions
 */

import type { InputToken, RegisteredShortcut, ShortcutPayload, InputScope } from './types';
import { inputStore } from './store/inputStore';
import { shortcutStore } from './store/shortcutStore';
import { tokensEqual } from './normalizer';
import { emitCommand } from './commandBus';

// Re-export for backward API compatibility
export { onCommand, clearCommandHandlers } from './commandBus';

const INPUT_ELEMENTS = ['INPUT', 'TEXTAREA', 'SELECT'];
const ARROW_KEYS = ['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'];

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

    return (
        INPUT_ELEMENTS.includes(active.tagName) || active.getAttribute('contenteditable') === 'true'
    );
}

function isInteractiveFocused(target?: EventTarget | null): boolean {
    if (typeof document === 'undefined') return false;

    const active = target instanceof HTMLElement ? target : document.activeElement;
    if (!active || active === document.body) return false;

    if (!document.contains(active)) return false;

    // HTML semantic interactive elements
    const INTERACTIVE_ELEMENTS = ['BUTTON', 'A', 'SUMMARY', 'SELECT', 'DETAILS'];
    if (INTERACTIVE_ELEMENTS.includes(active.tagName)) return true;

    // Elements explicitly acting as interactive via ARIA
    if (active.hasAttribute('role')) {
        const role = active.getAttribute('role');
        const interactiveRoles = [
            'button',
            'link',
            'menuitem',
            'checkbox',
            'radio',
            'tab',
            'switch',
            'treeitem',
            'combobox',
            'listbox',
            'option',
            'searchbox',
            'slider',
            'spinbutton',
            'textbox'
        ];
        if (role && interactiveRoles.includes(role)) return true;
    }

    return false;
}

function isArrowNavigationWidgetFocused(target?: EventTarget | null): boolean {
    if (typeof document === 'undefined') return false;

    const focusedElement =
        target instanceof HTMLElement ? target : (document.activeElement as HTMLElement | null);
    const role = focusedElement?.getAttribute('role');
    const navigationWidgetRoles = [
        'combobox',
        'listbox',
        'option',
        'searchbox',
        'slider',
        'spinbutton',
        'textbox'
    ];

    return navigationWidgetRoles.includes(role ?? '');
}

interface MatchResult {
    shortcut: RegisteredShortcut;
    matchType: 'single' | 'sequence' | 'chord';
}

/**
 * Calculates the priority threshold from blocking scopes.
 */
function calculateCutoffPriority(scopeStack: InputScope[]): number {
    let cutoffPriority = -Infinity;
    for (const scope of scopeStack) {
        if (scope.blockLowerScopes && scope.priority > cutoffPriority) {
            cutoffPriority = scope.priority;
        }
    }
    return cutoffPriority;
}

/**
 * Checks if a shortcut is blocked by higher priority scopes.
 */
function isShortcutBlocked(
    shortcut: RegisteredShortcut,
    scopeStack: InputScope[],
    cutoffPriority: number
): boolean {
    const activeScopeNames = scopeStack.map(scope => scope.name);

    // 1. Check if scope is active
    if (shortcut.scope && !activeScopeNames.includes(shortcut.scope)) {
        return true;
    }

    // 2. Check scope blocking
    let shortcutScopePriority = 0;
    if (shortcut.scope) {
        const scopeDefinition = scopeStack.find(scope => scope.name === shortcut.scope);
        if (scopeDefinition) {
            shortcutScopePriority = scopeDefinition.priority;
        }
    }

    // Shortcuts with modifiers typically bypass level-blocking
    const hasModifiers = shortcut.tokens.some(token => {
        const modifiers = token.meta?.modifiers;
        return Array.isArray(modifiers) && modifiers.length > 0;
    });

    if (shortcutScopePriority < cutoffPriority && !hasModifiers) {
        return true;
    }

    // Check enabledWhen condition
    if (shortcut.enabledWhen) {
        try {
            if (!shortcut.enabledWhen()) return true;
        } catch {
            return true;
        }
    }

    return false;
}

/**
 * Matches a specific token against a shortcut's token sequence.
 */
function matchShortcutTokens(
    shortcut: RegisteredShortcut,
    currentToken: InputToken,
    sequenceBuffer: InputToken[]
): MatchResult | null {
    const tokens = shortcut.tokens;
    if (tokens.length === 0) return null;

    // Single key match
    if (tokens.length === 1 && tokensEqual(tokens[0], currentToken)) {
        return { shortcut, matchType: 'single' };
    }

    // Sequence match (e.g., "g g" for go)
    if (tokens.length > 1) {
        const bufferWithCurrent = [...sequenceBuffer, currentToken];
        const tail = bufferWithCurrent.slice(-tokens.length);

        if (tail.length === tokens.length) {
            const sequenceMatches = tokens.every((token, index) => tokensEqual(token, tail[index]));
            if (sequenceMatches) {
                return { shortcut, matchType: 'sequence' };
            }
        }
    }

    return null;
}

/**
 * Find matching shortcuts for the current input state
 */
function findMatches(token: InputToken): MatchResult[] {
    const scopeStack = inputStore.scopeStack();
    const sequenceBuffer = inputStore.sequenceBuffer();
    const allShortcuts = shortcutStore.list();
    const cutoffPriority = calculateCutoffPriority(scopeStack);

    const matches: MatchResult[] = [];

    for (const shortcut of allShortcuts) {
        if (isShortcutBlocked(shortcut, scopeStack, cutoffPriority)) {
            continue;
        }

        const match = matchShortcutTokens(shortcut, token, sequenceBuffer);
        if (match) {
            matches.push(match);
        }
    }

    return matches;
}

/**
 * Compares two shortcut matches based on scope priority.
 */
function compareScopePriority(
    matchA: MatchResult,
    matchB: MatchResult,
    scopeStack: InputScope[]
): number {
    const scopeA = scopeStack.find(scope => scope.name === matchA.shortcut.scope);
    const scopeB = scopeStack.find(scope => scope.name === matchB.shortcut.scope);
    const priorityA = scopeA?.priority ?? 0;
    const priorityB = scopeB?.priority ?? 0;

    return priorityB - priorityA;
}

/**
 * Compares two shortcut matches based on shortcut-specific priority.
 */
function compareShortcutPriority(matchA: MatchResult, matchB: MatchResult): number {
    const priorityA = matchA.shortcut.priority ?? 0;
    const priorityB = matchB.shortcut.priority ?? 0;

    return priorityB - priorityA;
}

/**
 * Compares two shortcut matches based on sequence length/specificity.
 */
function compareSpecificity(matchA: MatchResult, matchB: MatchResult): number {
    const lengthA = matchA.shortcut.tokens.length;
    const lengthB = matchB.shortcut.tokens.length;

    return lengthB - lengthA;
}

/**
 * Comparison logic for sorting shortcuts by priority and specificity
 */
function compareShortcutMatches(
    matchA: MatchResult,
    matchB: MatchResult,
    scopeStack: InputScope[]
): number {
    const scopePriorityDiff = compareScopePriority(matchA, matchB, scopeStack);
    if (scopePriorityDiff !== 0) return scopePriorityDiff;

    const shortcutPriorityDiff = compareShortcutPriority(matchA, matchB);
    if (shortcutPriorityDiff !== 0) return shortcutPriorityDiff;

    const specificityDiff = compareSpecificity(matchA, matchB);
    if (specificityDiff !== 0) return specificityDiff;

    const isDefaultA = matchA.shortcut.isDefault ?? true;
    const isDefaultB = matchB.shortcut.isDefault ?? true;

    if (isDefaultA !== isDefaultB) return isDefaultA ? 1 : -1;

    return 0;
}

/**
 * Sort matches by priority and specificity
 */
function sortMatches(matches: MatchResult[]): MatchResult[] {
    const scopeStack = inputStore.scopeStack();

    return [...matches].sort((matchA, matchB) =>
        compareShortcutMatches(matchA, matchB, scopeStack)
    );
}

/**
 * Handle an incoming token and dispatch matching shortcuts
 * Returns true if a shortcut was dispatched
 */
export function dispatchToken(token: InputToken, event: Event | null): boolean {
    if (!inputStore.enabled()) return false;

    if (token.kind === 'keyboard') inputStore.keyDown(token);

    const inputFocused = isInputFocused(event?.target);
    const matches = findMatches(token);
    if (matches.length === 0) return false;

    const sortedMatches = sortMatches(matches);

    for (const match of sortedMatches) {
        if (executeMatchIfValid(match, token, event, inputFocused)) {
            return true;
        }
    }

    return false;
}

function shouldYieldToInteractive(
    shortcut: RegisteredShortcut,
    token: InputToken,
    target?: EventTarget | null
): boolean {
    if (shortcut.ignoreInputs === false || shortcut.allowOnInteractive) return false;
    if (shortcut.yieldToInteractive && isInteractiveFocused(target)) return true;

    const key = token.id.split('+').pop();
    if (key === 'Enter' || key === 'Space') return isInteractiveFocused(target);

    return (
        shortcut.yieldToNavigationWidgets === true &&
        ARROW_KEYS.includes(key ?? '') &&
        isArrowNavigationWidgetFocused(target)
    );
}

/**
 * Validates and executes a shortcut match.
 */
function executeMatchIfValid(
    match: MatchResult,
    token: InputToken,
    event: Event | null,
    inputFocused: boolean
): boolean {
    const { shortcut } = match;

    // Check ignoreInputs flag for text inputs
    if (shortcut.ignoreInputs && inputFocused && token.id !== 'Escape') {
        return false;
    }

    // Yield to interactive elements for standard interaction keys (Space, Enter)
    if (shouldYieldToInteractive(shortcut, token, event?.target)) {
        return false;
    }

    // Check if chord was already dispatched (for held keys)
    if (match.matchType === 'chord' && inputStore.isChordDispatched(shortcut.id)) {
        return false;
    }

    if (match.matchType === 'chord') {
        inputStore.markChordDispatched(shortcut.id);
    }

    return performDispatch(match, event);
}

/**
 * Performs the actual dispatching of a shortcut.
 */
function performDispatch(match: MatchResult, event: Event | null): boolean {
    const { shortcut } = match;
    try {
        if (shortcut.preventDefault && event && 'preventDefault' in event) {
            event.preventDefault();
        }

        const payload: ShortcutPayload = {
            shortcutDef: shortcut,
            sequence: inputStore.sequenceBuffer(),
            meta: match.shortcut.tokens[0].meta || {} // Simplified meta extraction
        };

        if (shortcut.handler) shortcut.handler(event, payload);
        if (shortcut.command) emitCommand(shortcut.command, payload);

        if (match.matchType === 'sequence') {
            inputStore.clearSequenceBuffer();
        }

        return true;
    } catch (error) {
        console.error(`[InputDispatcher] Error dispatching shortcut ${shortcut.id}:`, error);
        return false;
    }
}

/**
 * Handle key up event
 */
export function handleKeyUp(keyId: string): void {
    inputStore.keyUp(keyId);
}
