/**
 * SectionGroup Component
 * A reusable container for grouping related settings or items with a header.
 */

import { Component, JSX, splitProps } from 'solid-js';
import { cn } from '../../lib/utils';
import './section-group.css';

export interface SectionGroupProps extends JSX.HTMLAttributes<HTMLElement> {
  title: string;
  description?: string;
}

export const SectionGroup: Component<SectionGroupProps> = (props) => {
  const [local, others] = splitProps(props, ['class', 'title', 'children', 'description']);

  return (
    <section class={cn("section-group", local.class)} {...others}>
      <div class="section-group-header">
        <h3 class="section-group-title">{local.title}</h3>
        {local.description && (
            <p class="section-group-description">{local.description}</p>
        )}
      </div>
      <div class="section-group-content">
        {local.children}
      </div>
    </section>
  );
};
