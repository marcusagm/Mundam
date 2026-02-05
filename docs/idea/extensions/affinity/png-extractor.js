import { CONFIG } from './config.js';
import { bytesMatch } from './utils.js';

export class PNGExtractor {
    static extractAll(data) {
        const images = [];
        const dataLength = data.length;
        const signatureLength = CONFIG.PNG_SIGNATURE.length;
        
        for (let i = 0; i <= dataLength - signatureLength; i++) {
            if (this.isPNGSignature(data, i)) {
                const pngData = this.extractPNGAt(data, i);
                if (pngData) {
                    images.push(pngData);
                    i += pngData.length - 1;
                }
            }
        }
        
        return images;
    }
    
    static isPNGSignature(data, offset) {
        return bytesMatch(data, CONFIG.PNG_SIGNATURE, offset);
    }
    
    static extractPNGAt(data, startOffset) {
        const endOffset = this.findPNGEnd(data, startOffset);
        
        if (endOffset === -1) {
            return null;
        }
        
        const pngLength = endOffset - startOffset + 12;
        
        if (pngLength < 57) {
            return null;
        }
        
        return data.slice(startOffset, startOffset + pngLength);
    }
    
    static findPNGEnd(data, startOffset) {
        const iendSignature = CONFIG.PNG_IEND;
        const searchLimit = Math.min(startOffset + 50 * 1024 * 1024, data.length);
        
        for (let i = startOffset + 8; i <= searchLimit - iendSignature.length; i++) {
            if (bytesMatch(data, iendSignature, i)) {
                return i;
            }
        }
        
        return -1;
    }
    
    static selectBest(images, strategy = CONFIG.SELECTION_STRATEGY) {
        if (images.length === 0) return null;
        if (images.length === 1) return images[0];
        
        switch (strategy) {
            case 'smallest':
                return images.reduce((a, b) => a.byteLength < b.byteLength ? a : b);
            case 'largest':
                return images.reduce((a, b) => a.byteLength > b.byteLength ? a : b);
            case 'first':
            default:
                return images[0];
        }
    }
}
