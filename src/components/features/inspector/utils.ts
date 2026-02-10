import { formatActions, type MediaType } from '../../../core/store/formatStore';

export type { MediaType };

export const getMediaType = (filename: string): MediaType => {
    return formatActions.getMediaType(filename);
};
