import { createUniqueId, For, onMount, Show, type Component } from 'solid-js';
import type {
    PluginPackageReview as PackageReview,
    PluginPermission
} from '../../../../lib/plugins';
import { Button } from '../../../ui/Button';

/** Describes declarative plugin permissions in Settings. */
export const permissionLabels: Record<PluginPermission, string> = {
    'selection.read': 'Read the current asset selection',
    'assetPreview.read': 'Read the generated preview of the selected image',
    'assetData.own.read': 'Read its own fields on the selected asset',
    'assetData.own.write': 'Save its own fields on the selected asset'
};

/** Lists the permissions requested by a package. */
export const PermissionList: Component<{
    label: string;
    permissions: PluginPermission[];
}> = properties => (
    <Show when={properties.permissions.length}>
        <div class="plugin-settings-fields">
            <strong>{properties.label}</strong>
            <ul class="plugin-settings-permissions">
                <For each={properties.permissions}>
                    {permission => <li>{permissionLabels[permission]}</li>}
                </For>
            </ul>
        </div>
    </Show>
);

/** Shows package metadata and the permissions requested before an explicit installation. */
export const PluginPackageReview: Component<{
    review: PackageReview;
    busy: boolean;
    onInstall(reviewId: string): void;
    onCancel(): void;
}> = properties => {
    const heading = createUniqueId();
    const installableReview = () => properties.review.compatible && properties.review.reviewId;
    const actionLabel = () => {
        switch (properties.review.replacement?.kind) {
            case 'update':
                return 'Update and disable';
            case 'rollback':
                return 'Restore and disable';
            default:
                return 'Install disabled';
        }
    };
    let title: HTMLHeadingElement | undefined;
    onMount(() => title?.focus());

    return (
        <div class="plugin-settings-entry" role="region" aria-labelledby={heading}>
            <div class="plugin-settings-package">
                <img src={`data:image/png;base64,${properties.review.iconBase64}`} alt="" />
                <h4 id={heading} tabIndex={-1} ref={title}>
                    {properties.review.name}
                </h4>
            </div>
            <p>{properties.review.description}</p>
            <dl class="plugin-settings-details">
                <dt>Publisher</dt>
                <dd>{properties.review.publisher}</dd>
                <dt>Fingerprint</dt>
                <dd>
                    <code>{properties.review.publisherFingerprint}</code>
                </dd>
                <dt>Plugin ID</dt>
                <dd>{properties.review.pluginId}</dd>
                <dt>Version</dt>
                <dd>{properties.review.version}</dd>
                <dt>License</dt>
                <dd>{properties.review.license}</dd>
                <dt>Supported targets</dt>
                <dd>
                    {properties.review.targets
                        .map(target => `${target.platform}/${target.architecture}`)
                        .join(', ')}
                </dd>
            </dl>
            <Show when={properties.review.experimentalWorkerId}>
                {workerId => <p>Contains experimental worker {workerId()}.</p>}
            </Show>
            <PermissionList
                label="Required permissions"
                permissions={properties.review.requiredPermissions}
            />
            <PermissionList
                label="Optional permissions"
                permissions={properties.review.optionalPermissions}
            />
            <Show when={properties.review.replacement}>
                {replacement => (
                    <>
                        <p>
                            Replace version {replacement().currentVersion} with version{' '}
                            {properties.review.version}.
                        </p>
                        <PermissionList
                            label="New or newly required permissions"
                            permissions={replacement().addedPermissions}
                        />
                        <PermissionList
                            label="Removed permissions"
                            permissions={replacement().removedPermissions}
                        />
                    </>
                )}
            </Show>
            <Show
                when={installableReview()}
                fallback={<p role="status">This package is incompatible with this host.</p>}
            >
                <p>
                    {properties.review.replacement
                        ? 'Changing the version clears approvals in all catalogs. Enable the package again to approve its permissions.'
                        : 'Installation keeps this plugin disabled. Enabling it requires separate permission approval.'}
                </p>
            </Show>
            <div class="plugin-settings-actions">
                <Show when={installableReview()}>
                    {reviewId => (
                        <Button
                            size="sm"
                            disabled={properties.busy}
                            onClick={() => properties.onInstall(reviewId())}
                        >
                            {actionLabel()}
                        </Button>
                    )}
                </Show>
                <Button
                    size="sm"
                    variant="outline"
                    disabled={properties.busy}
                    onClick={properties.onCancel}
                >
                    Cancel review
                </Button>
            </div>
        </div>
    );
};
