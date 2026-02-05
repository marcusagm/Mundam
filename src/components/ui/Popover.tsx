import { Component, JSX, Show, createSignal, onCleanup, createEffect } from 'solid-js';
import { Portal } from 'solid-js/web';
import { cn } from '../../lib/utils';
import { createId, createFocusTrap, createClickOutside } from '../../lib/primitives';
import './popover.css';

export interface PopoverProps {
    trigger: JSX.Element;
    children: JSX.Element;
    class?: string;
    isOpen?: boolean;
    onOpenChange?: (open: boolean) => void;
    onClose?: () => void;
    offset?: number;
    autoClose?: boolean;
    align?: 'start' | 'center' | 'end';
}

/**
 * A robust Popover component with viewport-aware positioning and accessibility excellence.
 */
export const Popover: Component<PopoverProps> = props => {
    const [internalOpen, setInternalOpen] = createSignal(false);
    const isOpen = () => props.isOpen ?? internalOpen();

    const contentId = createId();
    let triggerRef: HTMLDivElement | undefined;
    let contentRef: HTMLDivElement | undefined;

    const setOpen = (open: boolean) => {
        if (props.isOpen !== undefined) {
            if (!open) {
                props.onClose?.();
                props.onOpenChange?.(false);
            } else {
                props.onOpenChange?.(true);
            }
        } else {
            setInternalOpen(open);
            if (!open) props.onClose?.();
            props.onOpenChange?.(open);
        }
    };

    const toggle = (e: MouseEvent) => {
        e.stopPropagation();
        setOpen(!isOpen());
    };

    const positionContent = () => {
        if (!triggerRef || !contentRef || !isOpen()) return;

        const rect = triggerRef.getBoundingClientRect();
        const contentRect = contentRef.getBoundingClientRect();
        const offset = props.offset ?? 8;
        const align = props.align ?? 'start';

        const viewportWidth = window.innerWidth;
        const viewportHeight = window.innerHeight;

        let left = rect.left;
        let top = rect.bottom + offset;

        // Apply alignment preference
        if (align === 'center') {
            left = rect.left + rect.width / 2 - contentRect.width / 2;
        } else if (align === 'end') {
            left = rect.right - contentRect.width;
        }

        // Boundary check - Right overflow
        if (left + contentRect.width > viewportWidth - 10) {
            left = viewportWidth - contentRect.width - 10;
        }

        // Boundary check - Left overflow
        if (left < 10) {
            left = 10;
        }

        // Boundary check - Bottom overflow
        if (top + contentRect.height > viewportHeight - 10) {
            // Flip to top if it doesn't fit below
            const topSpace = rect.top - offset;
            if (topSpace > contentRect.height) {
                top = rect.top - contentRect.height - offset;
            } else {
                // If it fits neither top nor bottom, push it up to fit in the viewport
                top = Math.max(10, viewportHeight - contentRect.height - 10);
            }
        }

        contentRef.style.top = `${top}px`;
        contentRef.style.left = `${left}px`;
    };

    // Handle outside clicks
    createClickOutside(
        () => [triggerRef, contentRef].filter(Boolean) as HTMLElement[],
        () => {
            if (isOpen() && props.autoClose !== false) {
                setOpen(false);
            }
        }
    );

    // Keyboard accessibility
    createEffect(() => {
        if (!isOpen()) return;

        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === 'Escape') {
                setOpen(false);
            }
        };

        window.addEventListener('keydown', handleKeyDown);
        onCleanup(() => window.removeEventListener('keydown', handleKeyDown));
    });

    // Position update on events
    createEffect(() => {
        if (isOpen()) {
            // Initial position
            requestAnimationFrame(positionContent);

            const updatePosition = () => {
                requestAnimationFrame(positionContent);
            };

            window.addEventListener('scroll', updatePosition, true);
            window.addEventListener('resize', updatePosition);

            onCleanup(() => {
                window.removeEventListener('scroll', updatePosition, true);
                window.removeEventListener('resize', updatePosition);
            });
        }
    });

    // Focus trapping logic using the primitive
    createFocusTrap(() => contentRef, isOpen);

    return (
        <div
            ref={triggerRef}
            onClick={toggle}
            class="ui-popover-wrapper"
            style={{ display: 'inline-block' }}
            aria-haspopup="true"
            aria-expanded={isOpen()}
            aria-controls={isOpen() ? contentId : undefined}
        >
            {props.trigger}

            <Show when={isOpen()}>
                <Portal>
                    <div
                        ref={contentRef}
                        id={contentId}
                        role="dialog"
                        class={cn('ui-popover-content', props.class)}
                        style={{
                            position: 'fixed',
                            'z-index': 10000,
                            top: '-9999px', // Initially off-screen to avoid flicker
                            left: '-9999px'
                        }}
                    >
                        {props.children}
                    </div>
                </Portal>
            </Show>
        </div>
    );
};
