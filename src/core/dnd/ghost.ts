import { AssetItem } from '../../types';
import { getAssetUrl } from '../utils/assetUrl';

/**
 * Creates a "ghost" element for drag-and-drop feedback.
 * Renders a stack of up to 3 thumbnails with a count badge.
 */
export const createDragGhost = (items: AssetItem[]) => {
    const container = document.createElement('div');
    const count = items.length;

    Object.assign(container.style, {
        position: 'absolute',
        top: '-1000px',
        left: '-1000px',
        width: '120px',
        height: 'auto',
        zIndex: '10000',
        pointerEvents: 'none'
    });

    // Deck Effect: render up to 3 items stacked
    const previewItems = items.slice(0, 3).reverse();

    previewItems.forEach((item, index) => {
        const card = document.createElement('div');
        const isTop = index === previewItems.length - 1;

        Object.assign(card.style, {
            position: isTop ? 'relative' : 'absolute',
            top: isTop ? '0' : `${(previewItems.length - 1 - index) * 4}px`,
            left: isTop ? '0' : `${(previewItems.length - 1 - index) * 4}px`,
            width: '100%',
            height: '80px',
            background: 'var(--bg-surface)',
            border: '1px solid var(--border-active)',
            borderRadius: '4px',
            boxShadow: '0 4px 8px rgba(0,0,0,0.4)',
            overflow: 'hidden',
            transform:
                count > 1 ? `rotate(${(previewItems.length - 1 - index) * 2 - 2}deg)` : 'none',
            zIndex: index
        });

        if (item.thumbnail_path) {
            const img = document.createElement('img');
            img.src = getAssetUrl(item.id, 'thumb');

            Object.assign(img.style, {
                width: '100%',
                height: '100%',
                objectFit: 'cover',
                display: 'block'
            });
            card.appendChild(img);
        }

        container.appendChild(card);
    });

    // Badge if more than 1 item is dragging
    if (count > 1) {
        const badge = document.createElement('div');
        Object.assign(badge.style, {
            position: 'absolute',
            top: '-6px',
            right: '-6px',
            background: 'var(--accent-color)',
            color: '#fff',
            fontSize: '10px',
            fontWeight: 'bold',
            padding: '2px 6px',
            borderRadius: '10px',
            zIndex: '100',
            boxShadow: '0 2px 4px rgba(0,0,0,0.3)'
        });
        badge.innerText = String(count);
        container.appendChild(badge);
    }

    document.body.appendChild(container);
    return container;
};
