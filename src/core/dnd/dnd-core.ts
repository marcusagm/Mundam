import { ActionResult } from '../types/actions';

// --- Result Types for DnD ---

export interface DndSuccessData {
    tagName?: string;
    count?: number;
}

export type DndActionResult = ActionResult<DndSuccessData | void>;

// --- Discriminated Union for Dragged Items ---

export interface AssetDragPayload {
    id: string;
    ids: string[]; // For batch operations
    filename: string;
    path: string;
    thumbnail_path?: string | null;
}

export interface TagDragPayload {
    id: string | number;
    name: string;
    parent_id?: string | number | null;
    color?: string | null;
}

/** An indexed folder dragged as a whole subtree. */
export interface FolderDragPayload {
    id: string;
    name: string;
    path: string;
}

export type DragItem =
    | { type: 'ASSET'; payload: AssetDragPayload }
    | { type: 'TAG'; payload: TagDragPayload }
    | { type: 'FOLDER'; payload: FolderDragPayload };

// --- Strategy Interface ---

export interface DropStrategy {
    /** Determines if this strategy can handle the given dragged item. */
    accepts(item: DragItem): boolean;

    /**
     * Performs the actual drop operation logic.
     * Strategies should avoid direct side-effects like toasing and instead
     * trigger store actions that return ActionResults.
     */
    onDrop(
        item: DragItem,
        targetId: number | string,
        position?: 'before' | 'inside' | 'after'
    ): Promise<DndActionResult>;

    /** Optional hook to determine if the current drag position is a valid drop target. */
    onDragOver?(item: DragItem): boolean;
}

// Registry to hold strategies
class DndStrategyRegistry {
    private strategies: Map<string, DropStrategy> = new Map();

    register(targetType: string, strategy: DropStrategy) {
        this.strategies.set(targetType, strategy);
    }

    get(targetType: string): DropStrategy | undefined {
        return this.strategies.get(targetType);
    }
}

export const dndRegistry = new DndStrategyRegistry();

// --- Global Drag State ---
// Using imports from solid-js since this file is part of the core logic
import { createSignal } from 'solid-js';
export const [currentDragItem, setDragItem] = createSignal<DragItem | null>(null);
export const [currentDropTargetId, setDropTargetId] = createSignal<number | string | null>(null);
