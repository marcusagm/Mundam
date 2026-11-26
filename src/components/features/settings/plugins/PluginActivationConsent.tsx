import { createSignal, createUniqueId, For, onMount, Show, type Component } from 'solid-js';
import type { PluginActivationReview, PluginPermission } from '../../../../lib/plugins';
import { Button } from '../../../ui/Button';
import { Checkbox } from '../../../ui/Checkbox';
import { permissionLabels, PermissionList } from './PluginPackageReview';

/** Presents required and optional permissions before enabling a plugin for this catalog. */
export const PluginActivationConsent: Component<{
    review: PluginActivationReview;
    busy: boolean;
    onEnable(reviewId: string, optionalPermissions: PluginPermission[]): void;
    onCancel(): void;
}> = properties => {
    const [optional, setOptional] = createSignal<PluginPermission[]>([]);
    const heading = createUniqueId();
    let title: HTMLHeadingElement | undefined;
    onMount(() => title?.focus());

    return (
        <div class="plugin-settings-entry" role="region" aria-labelledby={heading}>
            <h4 id={heading} tabIndex={-1} ref={title}>
                Enable {properties.review.name} · {properties.review.version}
            </h4>
            <p>
                {properties.review.requiredPermissions.length ||
                properties.review.optionalPermissions.length
                    ? 'Approve these permissions for the current catalog.'
                    : 'Enable this plugin for the current catalog. It requests no permissions.'}
            </p>
            <PermissionList
                label="Required permissions"
                permissions={properties.review.requiredPermissions}
            />
            <Show when={properties.review.optionalPermissions.length}>
                <div class="plugin-settings-fields" role="group" aria-label="Optional permissions">
                    <strong>Optional permissions</strong>
                    <For each={properties.review.optionalPermissions}>
                        {permission => (
                            <Checkbox
                                size="sm"
                                label={permissionLabels[permission]}
                                checked={optional().includes(permission)}
                                disabled={properties.busy}
                                onCheckedChange={checked =>
                                    setOptional(current =>
                                        checked
                                            ? [...current, permission]
                                            : current.filter(selected => selected !== permission)
                                    )
                                }
                            />
                        )}
                    </For>
                </div>
            </Show>
            <div class="plugin-settings-actions">
                <Button
                    size="sm"
                    disabled={properties.busy}
                    onClick={() => properties.onEnable(properties.review.reviewId, optional())}
                >
                    Approve and enable
                </Button>
                <Button
                    size="sm"
                    variant="outline"
                    disabled={properties.busy}
                    onClick={properties.onCancel}
                >
                    Cancel activation
                </Button>
            </div>
        </div>
    );
};
