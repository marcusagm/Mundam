import { render } from '@solidjs/testing-library';
import { createSignal, Show } from 'solid-js';
import { expect, it, vi } from 'vitest';
import { AssetItemContainer } from './AssetItemContainer';

vi.mock('../../../../core/dnd', () => ({ assetDragSource: vi.fn() }));
vi.mock('../../../../core/hooks/useAssetDropZone', () => ({
    useAssetDropZone: () => ({ isDropTarget: () => false, dragHandlers: {} })
}));

it('preserves external editing focus on remount and follows keyboard focus within the grid', () => {
    const [visible, setVisible] = createSignal(false);
    const [focused, setFocused] = createSignal(true);
    const view = render(() => (
        <>
            <input aria-label="Search" />
            <div role="grid" tabIndex={0}>
                <Show when={visible()}>
                    <AssetItemContainer
                        id="asset"
                        filename="sample.jpg"
                        path="sample.jpg"
                        thumbnailPath={null}
                        isSelected
                        isFocused={focused()}
                        style={{}}
                        onSelect={vi.fn()}
                        onOpen={vi.fn()}
                        getSelectedIds={() => []}
                        getItemInfo={() => undefined}
                    >
                        {() => null}
                    </AssetItemContainer>
                </Show>
            </div>
        </>
    ));
    const input = view.getByRole('textbox');
    input.focus();
    setVisible(true);
    setVisible(false);
    setVisible(true);
    expect(document.activeElement).toBe(input);
    setFocused(false);
    view.getByRole('grid').focus();
    setFocused(true);
    expect(document.activeElement).toBe(view.getByRole('gridcell'));
});
