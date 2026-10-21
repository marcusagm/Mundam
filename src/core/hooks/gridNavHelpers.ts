/**
 * Grid Navigation Helpers
 * Pure utility functions for grid keyboard navigation scoring and selection.
 */

import type { ShortcutPayload } from '../input/types';
import type { ItemPosition } from '../viewport';

export type NavigationDirection = 'up' | 'down' | 'left' | 'right';

/** Find an adjacent item by linear index when position-based lookup fails */
export function findByIndex(
    currentId: string,
    direction: NavigationDirection,
    allItems: { id: string }[]
): string | null {
    const currentIndex = allItems.findIndex(item => item.id === currentId);
    if (direction === 'up' || direction === 'left') {
        return currentIndex > 0 ? allItems[currentIndex - 1].id : null;
    }
    return currentIndex < allItems.length - 1 ? allItems[currentIndex + 1].id : null;
}

/** Compute directional score for candidate selection; lower is better */
function computeDirectionalScore(
    candidate: ItemPosition,
    centerX: number,
    centerY: number,
    direction: NavigationDirection
): number | null {
    const posCenterX = candidate.x + candidate.width / 2;
    const posCenterY = candidate.y + candidate.height / 2;

    switch (direction) {
        case 'up':
            if (posCenterY < centerY - 10) {
                return Math.abs(posCenterX - centerX) * 2 + Math.abs(centerY - posCenterY);
            }
            return null;
        case 'down':
            if (posCenterY > centerY + 10) {
                return Math.abs(posCenterX - centerX) * 2 + Math.abs(posCenterY - centerY);
            }
            return null;
        case 'left':
            if (posCenterX < centerX - 10) {
                return Math.abs(posCenterY - centerY) * 2 + Math.abs(centerX - posCenterX);
            }
            return null;
        case 'right':
            if (posCenterX > centerX + 10) {
                return Math.abs(posCenterY - centerY) * 2 + Math.abs(posCenterX - centerX);
            }
            return null;
    }
}

/** Find the best positional candidate among visible items */
export function findBestCandidate(
    currentPos: ItemPosition,
    direction: NavigationDirection,
    visibleItems: ItemPosition[]
): ItemPosition | null {
    let bestCandidate: ItemPosition | null = null;
    let bestScore = Infinity;

    const centerX = currentPos.x + currentPos.width / 2;
    const centerY = currentPos.y + currentPos.height / 2;

    for (const candidate of visibleItems) {
        if (candidate.id === currentPos.id) continue;

        const score = computeDirectionalScore(candidate, centerX, centerY, direction);
        if (score !== null && score < bestScore) {
            bestScore = score;
            bestCandidate = candidate;
        }
    }

    return bestCandidate;
}

/**
 * Extract selection modifiers from a DOM Event or ShortcutPayload.
 * Distinguishes between multi-select (CMD/CTRL) and range-select (SHIFT).
 */
export function extractSelectionModifiers(argument?: Event | ShortcutPayload): {
    multi: boolean;
    shift: boolean;
} {
    if (!argument) return { multi: false, shift: false };

    if ('getModifierState' in argument) {
        const event = argument as MouseEvent | KeyboardEvent;
        return {
            multi: event.ctrlKey || event.metaKey,
            shift: event.shiftKey
        };
    }

    const payload = argument as ShortcutPayload;
    if (payload.meta && Array.isArray(payload.meta.modifiers)) {
        const modifiers = payload.meta.modifiers as string[];
        return {
            multi:
                modifiers.includes('Ctrl') ||
                modifiers.includes('Control') ||
                modifiers.includes('Meta'),
            shift: modifiers.includes('Shift')
        };
    }

    return { multi: false, shift: false };
}

/**
 * Extract the multi-select flag (backward compatibility).
 * @deprecated Use extractSelectionModifiers instead.
 */
export function extractMultiFlag(argument?: Event | ShortcutPayload): boolean {
    const modifiers = extractSelectionModifiers(argument);
    return modifiers.multi || modifiers.shift;
}
