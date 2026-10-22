/**
 * Utility functions for color conversion and validation.
 * Supports Hexadecimal, Hue-Saturation-Brightness (HSB/HSV), and Hue-Saturation-Lightness (HSL) formats.
 */

/**
 * Interface representing a color in Hue, Saturation, and Brightness format.
 */
export interface HueSaturationBrightness {
    /**
     * Hue value from 0 to 360
     *
     * @returns {number} The hue value.
     */
    hue: number;

    /**
     * Saturation value from 0 to 100
     *
     * @returns {number} The saturation value.
     */
    saturation: number;

    /**
     * Brightness value from 0 to 100
     *
     * @returns {number} The brightness value.
     */
    brightness: number;
}

/**
 * Interface representing a color in Hue, Saturation, and Lightness format.
 */
export interface HueSaturationLightness {
    /**
     * Hue value from 0 to 360
     *
     * @returns {number} The hue value.
     */
    hue: number;

    /**
     * Saturation value from 0 to 1
     *
     * @returns {number} The saturation value.
     */
    saturation: number;

    /**
     * Lightness value from 0 to 1
     *
     * @returns {number} The lightness value.
     */
    lightness: number;
}

/**
 * Validates if a string is a valid hexadecimal color code.
 * Supports both 3-digit (#RGB) and 6-digit (#RRGGBB) formats.
 *
 * @param {string} hexadecimal - The string to validate.
 * @returns {boolean} True if the string is a valid hexadecimal color.
 */
export function validateHexadecimalColor(hexadecimal: string): boolean {
    return /^#([0-9A-Fa-f]{3}|[0-9A-Fa-f]{6})$/.test(hexadecimal);
}

/**
 * Converts a Hue-Saturation-Brightness color to a Hexadecimal color string.
 *
 * @param {number} hue - Hue value (0-360).
 * @param {number} saturation - Saturation value (0-100).
 * @param {number} brightness - Brightness value (0-100).
 * @returns {string} Hexadecimal color string (e.g., "#FF0000").
 */
export function convertHueSaturationBrightnessToHexadecimal(
    hue: number,
    saturation: number,
    brightness: number
): string {
    /**
     * Normalizes the saturation and brightness values to a range of 0 to 1.
     *
     * @param {number} saturation - The saturation value to normalize.
     * @param {number} brightness - The brightness value to normalize.
     * @returns {{normalizedSaturation: number, normalizedBrightness: number}} The normalized saturation and brightness values.
     */
    const normalizedSaturation = saturation / 100;

    /**
     * Normalizes the brightness value to a range of 0 to 1.
     *
     * @param {number} brightness - The brightness value to normalize.
     * @returns {number} The normalized brightness value.
     */
    const normalizedBrightness = brightness / 100;

    /**
     * Calculates the RGB components of the color.
     *
     * @param {number} index - The index of the RGB component to calculate.
     * @returns {number} The calculated RGB component value.
     */
    const calculateComponent = (index: number) => {
        const factor = (index + hue / 60) % 6;
        return (
            normalizedBrightness *
            (1 - normalizedSaturation * Math.max(0, Math.min(factor, 4 - factor, 1)))
        );
    };

    /**
     * Converts a value to its hexadecimal representation.
     *
     * @param {number} value - The value to convert.
     * @returns {string} The hexadecimal representation of the value.
     */
    const toHexadecimal = (value: number) =>
        Math.round(255 * value)
            .toString(16)
            .padStart(2, '0');

    return `#${toHexadecimal(calculateComponent(5))}${toHexadecimal(
        calculateComponent(3)
    )}${toHexadecimal(calculateComponent(1))}`;
}

/**
 * Converts a Hexadecimal color string to Hue-Saturation-Brightness format.
 *
 * @param {string} hexadecimal - Hexadecimal color string (e.g., "#FF0000" or "#F00").
 * @returns {HueSaturationBrightness} Color in HSB format.
 */
export function convertHexadecimalToHueSaturationBrightness(
    hexadecimal: string
): HueSaturationBrightness {
    let processedHexadecimal = hexadecimal.replace(/^#/, '');

    if (processedHexadecimal.length === 3) {
        processedHexadecimal = processedHexadecimal
            .split('')
            .map(character => character + character)
            .join('');
    }

    const red = parseInt(processedHexadecimal.slice(0, 2), 16) / 255;
    const green = parseInt(processedHexadecimal.slice(2, 4), 16) / 255;
    const blue = parseInt(processedHexadecimal.slice(4, 6), 16) / 255;
    const max = Math.max(red, green, blue);
    const min = Math.min(red, green, blue);
    const delta = max - min;

    let hue = 0;
    if (delta !== 0) {
        if (max === red) {
            hue = (green - blue) / delta;
        } else if (max === green) {
            hue = 2 + (blue - red) / delta;
        } else {
            hue = 4 + (red - green) / delta;
        }
        hue = 60 * (hue < 0 ? hue + 6 : hue);
    }

    const saturation = max ? (delta / max) * 100 : 0;
    const brightness = max * 100;

    return {
        hue,
        saturation,
        brightness
    };
}

/**
 * Normalizes a hexadecimal color string, adding '#' if missing and expanding shorthand format.
 *
 * @param {string} inputValue - The raw string input.
 * @returns {string | null} Normalized hex string or null if invalid.
 */
export function normalizeHexadecimalValue(inputValue: string): string | null {
    let processedValue = inputValue;

    if (!processedValue.startsWith('#') && /^[0-9A-Fa-f]{3,6}$/.test(processedValue)) {
        processedValue = '#' + processedValue;
    }

    if (validateHexadecimalColor(processedValue)) {
        if (processedValue.length === 4) {
            processedValue =
                '#' +
                processedValue[1] +
                processedValue[1] +
                processedValue[2] +
                processedValue[2] +
                processedValue[3] +
                processedValue[3];
        }
        return processedValue;
    }

    return null;
}

/**
 * Converts a hex color string to HSL values.
 *
 * @param {string} hexColor - Hex string like "#FF5733" or "FF5733".
 * @returns {HueSaturationLightness} An HSL object with hue (0-360), saturation (0-1), lightness (0-1).
 */
export function convertHexadecimalToHueSaturationLightness(
    hexColor: string
): HueSaturationLightness {
    const hexTrimmed = hexColor.replace('#', '');
    const red = parseInt(hexTrimmed.substring(0, 2), 16) / 255;
    const green = parseInt(hexTrimmed.substring(2, 4), 16) / 255;
    const blue = parseInt(hexTrimmed.substring(4, 6), 16) / 255;

    const maxChannel = Math.max(red, green, blue);
    const minChannel = Math.min(red, green, blue);
    const channelDelta = maxChannel - minChannel;

    const lightness = (maxChannel + minChannel) / 2;

    if (channelDelta === 0) {
        return { hue: 0, saturation: 0, lightness };
    }

    const saturation =
        lightness > 0.5
            ? channelDelta / (2 - maxChannel - minChannel)
            : channelDelta / (maxChannel + minChannel);

    let hue = 0;
    if (maxChannel === red) {
        hue = ((green - blue) / channelDelta + (green < blue ? 6 : 0)) * 60;
    } else if (maxChannel === green) {
        hue = ((blue - red) / channelDelta + 2) * 60;
    } else {
        hue = ((red - green) / channelDelta + 4) * 60;
    }

    return { hue, saturation, lightness };
}

/**
 * Interface representing a color format option for copying or displaying.
 */
export interface ColorFormatOption {
    label: string;
    value: string;
}

/**
 * Generates an array of color string representations in various formats.
 * Formats include HEX, RGB, RGBA, HSL, HSV, HWB, and CMYK.
 *
 * @param {string} hexColor - The input hexadecimal color string.
 * @returns {ColorFormatOption[]} Array of color formats with labels and string values.
 */
export function generateColorFormats(hexColor: string): ColorFormatOption[] {
    const red = parseInt(hexColor.slice(1, 3), 16);
    const green = parseInt(hexColor.slice(3, 5), 16);
    const blue = parseInt(hexColor.slice(5, 7), 16);

    const redNormalized = red / 255;
    const greenNormalized = green / 255;
    const blueNormalized = blue / 255;

    // HSL Calculation
    const maxChannel = Math.max(redNormalized, greenNormalized, blueNormalized);
    const minChannel = Math.min(redNormalized, greenNormalized, blueNormalized);
    const channelDelta = maxChannel - minChannel;
    const lightness = (maxChannel + minChannel) / 2;
    let saturationHsl = 0;
    if (channelDelta !== 0) {
        saturationHsl =
            lightness > 0.5
                ? channelDelta / (2 - maxChannel - minChannel)
                : channelDelta / (maxChannel + minChannel);
    }

    let hue = 0;
    if (channelDelta !== 0) {
        if (maxChannel === redNormalized) {
            hue =
                ((greenNormalized - blueNormalized) / channelDelta +
                    (greenNormalized < blueNormalized ? 6 : 0)) *
                60;
        } else if (maxChannel === greenNormalized) {
            hue = ((blueNormalized - redNormalized) / channelDelta + 2) * 60;
        } else {
            hue = ((redNormalized - greenNormalized) / channelDelta + 4) * 60;
        }
    }

    // HSV Calculation
    const saturationHsv = maxChannel === 0 ? 0 : channelDelta / maxChannel;
    const brightness = maxChannel;

    // HWB Calculation
    const whiteness = minChannel;
    const blackness = 1 - maxChannel;

    // CMYK Calculation
    const black = 1 - maxChannel;
    let cyan = 0;
    let magenta = 0;
    let yellow = 0;
    if (black < 1) {
        cyan = (1 - redNormalized - black) / (1 - black);
        magenta = (1 - greenNormalized - black) / (1 - black);
        yellow = (1 - blueNormalized - black) / (1 - black);
    }

    return [
        { label: 'HEX', value: hexColor.toUpperCase() },
        { label: 'RGB', value: `rgb(${red}, ${green}, ${blue})` },
        { label: 'RGBA', value: `rgba(${red}, ${green}, ${blue}, 1)` },
        {
            label: 'HSL',
            value: `hsl(${Math.round(hue)}, ${Math.round(saturationHsl * 100)}%, ${Math.round(lightness * 100)}%)`
        },
        {
            label: 'HSV',
            value: `hsv(${Math.round(hue)}, ${Math.round(saturationHsv * 100)}%, ${Math.round(brightness * 100)}%)`
        },
        {
            label: 'HWB',
            value: `hwb(${Math.round(hue)} ${Math.round(whiteness * 100)}% ${Math.round(blackness * 100)}%)`
        },
        {
            label: 'CMYK',
            value: `cmyk(${Math.round(cyan * 100)}%, ${Math.round(magenta * 100)}%, ${Math.round(yellow * 100)}%, ${Math.round(black * 100)}%)`
        }
    ];
}

const SAFE_CSS_COLOR_PATTERN =
    /^(#[0-9a-fA-F]{3,8}|(rgb|rgba|hsl|hsla|oklch|oklab|lab|lch|color)\([0-9a-zA-Z%.,\s/+-]+\))$/;
const MAXIMUM_CSS_COLOR_LENGTH = 100;

/**
 * Checks whether a string is a plain CSS color (hex or functional notation).
 * Rejects `url()`, `var()`, semicolons, braces and any other non-color content,
 * so the value is safe to assign to a custom property.
 *
 * @param {string} value - The CSS value to validate.
 * @returns {boolean} True if the value is a safe CSS color.
 *
 * @example
 * ```ts
 * isSafeCssColor('oklch(50% 0.1 200)'); // true
 * isSafeCssColor('red; background: url(x)'); // false
 * ```
 */
export function isSafeCssColor(value: string): boolean {
    return value.length <= MAXIMUM_CSS_COLOR_LENGTH && SAFE_CSS_COLOR_PATTERN.test(value.trim());
}

/**
 * Converts any CSS color understood by the browser (including oklch) to `#rrggbb`
 * by rasterizing it on a 1x1 canvas.
 *
 * @param {string} cssColor - The CSS color to convert.
 * @returns {string | null} The hexadecimal color, or null if it cannot be resolved.
 *
 * @example
 * ```ts
 * convertCssColorToHexadecimal('rgb(255, 0, 0)'); // '#ff0000'
 * ```
 */
export function convertCssColorToHexadecimal(cssColor: string): string | null {
    const canvas = document.createElement('canvas');
    canvas.width = 1;
    canvas.height = 1;
    const context = canvas.getContext('2d', { willReadFrequently: true });
    if (!context) return null;

    context.clearRect(0, 0, 1, 1);
    context.fillStyle = '#000';
    context.fillStyle = cssColor;
    context.fillRect(0, 0, 1, 1);

    const [red, green, blue, alpha] = context.getImageData(0, 0, 1, 1).data;
    if (alpha === 0) return null;

    return `#${[red, green, blue].map(channel => channel.toString(16).padStart(2, '0')).join('')}`;
}
