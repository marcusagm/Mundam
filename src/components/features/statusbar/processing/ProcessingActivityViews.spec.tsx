import { cleanup, fireEvent, render } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ProcessingActivityPanel } from './ProcessingActivityPanel';
import { createInitialState } from './ProcessingActivityPanelTestUtils';

afterEach(cleanup);

describe('processing activity panel view modes', () => {
    it('supports switching between overview and diagnostics views with developer mode', () => {
        const current = createInitialState();
        current.snapshot!.capacity = {
            limit: 4,
            active: 1,
            waiting: 0,
            thumbnail_limit: 2
        };
        const view = render(() => (
            <ProcessingActivityPanel
                state={current}
                indexing={null}
                onPause={vi.fn()}
                isDeveloperMode={true}
            />
        ));
        const overviewTabButton = view.getByRole('tab', { name: 'Overview' });
        const diagnosticsTabButton = view.getByRole('tab', {
            name: 'Diagnostics'
        });
        expect(overviewTabButton).toBeTruthy();
        expect(diagnosticsTabButton).toBeTruthy();

        fireEvent.click(overviewTabButton);
        expect(view.getByText('Library is up to date')).toBeTruthy();
        expect(view.queryByLabelText('Processing capacity')).toBeNull();

        fireEvent.click(diagnosticsTabButton);
        expect(view.getByLabelText('Processing capacity')).toBeTruthy();
    });
});
