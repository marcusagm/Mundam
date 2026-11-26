/**
 * Simple, typesafe Event Bus for domain-level communication.
 * Allows decoupling stores and components by providing a central
 * pub/sub mechanism.
 *
 * @module EventBus
 */

type EventCallback<T = unknown> = (data: T) => void;

/**
 * Registry of available domain events and their payload types.
 */
export interface DomainEvents {
    'plugins:presentation-changed': undefined;
    'metadata:changed': {
        type: 'tag' | 'location' | 'smart-folder' | 'stats';
        ids?: (string | number)[];
    };
    'assets:metadata-updated': {
        assetIds: string[];
        fields: string[];
    };
    'search:executed': {
        query: string;
        isAdvanced: boolean;
    };
}

class EventBus {
    private listeners: Map<keyof DomainEvents, Set<EventCallback<unknown>>> = new Map();

    /**
     * Subscribes to a specific domain event.
     *
     * @param event - The event name.
     * @param callback - Function to execute when the event is emitted.
     * @returns A function to unsubscribe.
     */
    on<K extends keyof DomainEvents>(
        event: K,
        callback: EventCallback<DomainEvents[K]>
    ): () => void {
        if (!this.listeners.has(event)) {
            this.listeners.set(event, new Set());
        }

        const eventListeners = this.listeners.get(event)!;
        const untypedCallback = callback as EventCallback<unknown>;
        eventListeners.add(untypedCallback);

        return () => {
            const currentListeners = this.listeners.get(event);
            if (currentListeners) {
                currentListeners.delete(untypedCallback);
            }
        };
    }

    /**
     * Emits a domain event with the specified data.
     *
     * @param event - The event name.
     * @param data - The payload for the event.
     */
    emit<K extends keyof DomainEvents>(event: K, data: DomainEvents[K]): void {
        this.listeners.get(event)?.forEach(callback => {
            try {
                callback(data);
            } catch (error) {
                console.error(`Error in event listener for ${event}:`, error);
            }
        });
    }
}

/**
 * Global singleton instance of the Event Bus.
 */
export const eventBus = new EventBus();
