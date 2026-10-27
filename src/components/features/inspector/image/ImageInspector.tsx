import { Component, onMount } from 'solid-js';
import { type AssetItem } from '../../../../types';
import { CommonMetadata } from '../base/CommonMetadata';
import { AssetMetadata } from './AssetMetadata.tsx';
import { InspectorTags } from '../base/InspectorTags';
import { AdvancedMetadata } from './AdvancedMetadata.tsx';
import { ColorPaletteSection } from './ColorPaletteSection';
import { Accordion } from '../../../ui';
import { accordionActions, accordionStoreState } from '../../../../core/store/accordionStore';
import { getAssetUrl } from '../../../../core/utils/assetUrl';
import './ImageInspector.css';

interface ImageInspectorProps {
    item: AssetItem;
}

export const ImageInspector: Component<ImageInspectorProps> = props => {
    onMount(() => {
        accordionActions.initializeAccordion('inspector_image', ['common']);
    });

    return (
        <div class="inspector-content">
            <div class="inspector-preview square">
                <img
                    class="preview-image"
                    src={props.item.thumbnail_path ? getAssetUrl(props.item.id, 'thumb') : ''}
                    alt={props.item.filename}
                />
            </div>

            <Accordion
                type="multiple"
                value={accordionStoreState['inspector_image'] || []}
                onValueChange={val => accordionActions.setExpandedItems('inspector_image', val)}
            >
                <CommonMetadata item={props.item} />
                <AssetMetadata item={props.item} />
                <InspectorTags itemId={props.item.id} />
                <AdvancedMetadata item={props.item} />
                <ColorPaletteSection item={props.item} />
            </Accordion>
        </div>
    );
};
