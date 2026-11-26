import { For, Match, onMount, Switch, type Component, type JSX } from 'solid-js';
import { ArrowLeft } from 'lucide-solid';
import { GlobalStatusbar } from '../components/layout/GlobalStatusbar';
import { Button } from '../components/ui/Button';
import type { PluginScreen } from '../lib/pluginPresentation';
import './plugin-screen-view.css';

/** Renders an approved screen with plain text and native application controls. */
export const PluginScreenView: Component<{
    header: JSX.Element;
    screen: PluginScreen;
    focusHeading: boolean;
    onBack: () => void;
}> = properties => {
    let heading: HTMLHeadingElement | undefined;
    onMount(() => {
        if (properties.focusHeading) heading?.focus();
    });

    return (
        <div class="plugin-screen-view">
            {properties.header}
            <main class="plugin-screen-scroll">
                <article class="plugin-screen-content">
                    <div class="plugin-screen-heading">
                        <p>{properties.screen.pluginName}</p>
                        <h1 ref={heading} tabIndex={-1}>
                            {properties.screen.title}
                        </h1>
                    </div>
                    <Button size="sm" variant="ghost" onClick={() => properties.onBack()}>
                        <ArrowLeft size={16} aria-hidden="true" />
                        Back to gallery
                    </Button>
                    <For each={properties.screen.blocks}>
                        {block => (
                            <Switch>
                                <Match when={block.kind === 'paragraph' && block}>
                                    {paragraph => <p>{paragraph().text}</p>}
                                </Match>
                                <Match when={block.kind === 'list' && block}>
                                    {list => (
                                        <ul>
                                            <For each={list().items}>{item => <li>{item}</li>}</For>
                                        </ul>
                                    )}
                                </Match>
                            </Switch>
                        )}
                    </For>
                </article>
            </main>
            <GlobalStatusbar />
        </div>
    );
};
