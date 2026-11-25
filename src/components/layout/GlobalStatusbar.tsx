import { Component, JSX, Show } from 'solid-js';
import { StatusMessages } from '../features/statusbar/StatusMessages';
import { StatusSystem } from '../features/statusbar/StatusSystem';
import './global-statusbar.css';

/** Displays shared processing status alongside controls supplied by the active view. */
export const GlobalStatusbar: Component<{
    context?: JSX.Element;
    actions?: JSX.Element;
}> = props => (
    <footer class="global-statusbar" aria-label="Application status">
        <Show when={props.context}>{context => <div class="statusbar-left">{context()}</div>}</Show>
        <div class="statusbar-center">
            <StatusMessages />
        </div>
        <div class="statusbar-right">
            <StatusSystem />
            {props.actions}
        </div>
    </footer>
);
