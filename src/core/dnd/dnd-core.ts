// Define types for Draggable Items
export interface DragItem {
    type: "IMAGE" | "TAG";
    payload: any;
}

// Strategy Interface
export interface DropStrategy {
    accepts(item: DragItem): boolean;
    onDrop(item: DragItem, targetId: number | string): Promise<void>;
    onDragOver?(item: DragItem): boolean; // valid drop target?
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

import { createSignal } from "solid-js";

export const dndRegistry = new DndStrategyRegistry();

// Global Drag State (Signal for Reactivity)
export const [currentDragItem, setDragItem] = createSignal<DragItem | null>(null);
