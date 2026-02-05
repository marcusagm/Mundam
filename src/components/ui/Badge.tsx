import { Component, JSX, splitProps } from 'solid-js';
import { cn } from '../../lib/utils';
import './badge.css';

export type BadgeVariant =
    | 'default'
    | 'outline'
    | 'secondary'
    | 'success'
    | 'warning'
    | 'error'
    | 'info';
export type BadgeSize = 'sm' | 'md';

export interface BadgeProps extends JSX.HTMLAttributes<HTMLSpanElement> {
    /** Visual variant */
    variant?: BadgeVariant;
    /** Size variant */
    size?: BadgeSize;
    /** Additional content (e.g., icon) */
    children?: JSX.Element;
}

/**
 * Badge component for highlighting status or metadata.
 *
 * @example
 * <Badge>New</Badge>
 *
 * @example
 * <Badge variant="success">Active</Badge>
 *
 * @example
 * <Badge variant="error" size="sm">3 errors</Badge>
 */
export const Badge: Component<BadgeProps> = props => {
    const [local, others] = splitProps(props, ['variant', 'size', 'class', 'children']);

    const variant = () => local.variant || 'default';
    const size = () => local.size || 'md';

    return (
        <span
            class={cn('ui-badge', `ui-badge-${variant()}`, `ui-badge-${size()}`, local.class)}
            {...others}
        >
            {local.children}
        </span>
    );
};
