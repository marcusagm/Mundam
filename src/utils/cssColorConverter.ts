import { validateHexadecimalColor } from './color';
import { convertOklchColorToHexadecimal } from './oklchColorConverter';

/**
 * Measures the computed CSS color of an expression using an ephemeral probe element.
 *
 * @param {string} colorExpression - The CSS color expression.
 * @param {Element} hostElement - The parent container element.
 * @returns {string} The computed color string.
 */
function measureElementComputedColor(colorExpression: string, hostElement: Element): string {
    const probeElement = document.createElement('span');
    probeElement.style.color = colorExpression;
    probeElement.style.position = 'fixed';
    probeElement.style.pointerEvents = 'none';
    probeElement.style.opacity = '0';
    probeElement.style.visibility = 'hidden';

    hostElement.appendChild(probeElement);
    const computedColor = getComputedStyle(probeElement).color;
    hostElement.removeChild(probeElement);
    return computedColor;
}

/**
 * Iteratively resolves CSS variable chains to retrieve their underlying values.
 *
 * @param {string} initialExpression - The variable expression or token name.
 * @returns {string} The resolved property value.
 */
function resolveCssVariableHierarchy(initialExpression: string): string {
    let currentExpression = initialExpression;
    let iterationCounter = 0;
    const maximumIterations = 10;

    while (
        iterationCounter < maximumIterations &&
        (currentExpression.startsWith('--') || currentExpression.includes('var('))
    ) {
        iterationCounter += 1;
        const variableNameMatch =
            currentExpression.match(/var\(\s*(--[\w-]+)/) ||
            currentExpression.match(/^(--[\w-]+)$/);
        if (!variableNameMatch) {
            break;
        }

        const variableName = variableNameMatch[1];
        const propertyValue =
            getComputedStyle(document.documentElement).getPropertyValue(variableName).trim() ||
            (document.body
                ? getComputedStyle(document.body).getPropertyValue(variableName).trim()
                : '');

        if (!propertyValue || propertyValue === currentExpression) {
            break;
        }
        currentExpression = propertyValue;
    }

    return currentExpression;
}

/**
 * Parses a CSS numeric component, including percentage values.
 *
 * @param {string} component - The CSS component.
 * @param {number} percentageScale - The multiplier used to normalize percentages.
 * @returns {number | null} The parsed value, or null when the component is invalid.
 */
function parseCssNumericComponent(component: string, percentageScale: number): number | null {
    const isPercentage = component.endsWith('%');
    const numericValue = Number(isPercentage ? component.slice(0, -1) : component);
    if (!Number.isFinite(numericValue)) return null;
    return isPercentage ? (numericValue / 100) * percentageScale : numericValue;
}

/**
 * Formats normalized sRGB channels as a hexadecimal color.
 *
 * @param {number} red - The normalized red channel.
 * @param {number} green - The normalized green channel.
 * @param {number} blue - The normalized blue channel.
 * @returns {string} A six-digit hexadecimal color.
 */
function formatSrgbHexadecimal(red: number, green: number, blue: number): string {
    const redHexadecimal = Math.round(Math.min(1, Math.max(0, red)) * 255)
        .toString(16)
        .padStart(2, '0');
    const greenHexadecimal = Math.round(Math.min(1, Math.max(0, green)) * 255)
        .toString(16)
        .padStart(2, '0');
    const blueHexadecimal = Math.round(Math.min(1, Math.max(0, blue)) * 255)
        .toString(16)
        .padStart(2, '0');
    return `#${redHexadecimal}${greenHexadecimal}${blueHexadecimal}`;
}

/**
 * Converts an sRGB functional color to hexadecimal.
 *
 * @param {string} colorExpression - An rgb() or rgba() expression.
 * @returns {string | null} The hexadecimal color, or null when the expression is invalid.
 */
function convertRgbColorToHexadecimal(colorExpression: string): string | null {
    const functionMatch = colorExpression.match(/^rgba?\((.*)\)$/i);
    if (!functionMatch) return null;

    const components = functionMatch[1].replace(/,/g, ' ').replace('/', ' ').trim().split(/\s+/);
    if (components.length < 3) return null;

    const [redChannel, greenChannel, blueChannel] = components
        .slice(0, 3)
        .map(component => parseCssNumericComponent(component, 255));
    if (redChannel === null || greenChannel === null || blueChannel === null) return null;

    return formatSrgbHexadecimal(redChannel / 255, greenChannel / 255, blueChannel / 255);
}

/**
 * Resolves a CSS custom property or color string by measuring a probe element attached to the DOM.
 * This guarantees that CSS variables like `var(--p-neutral-900)` and custom property names
 * are resolved through the active cascade before conversion.
 *
 * @param {string} cssColorOrToken - The CSS color expression or token name.
 * @returns {string} The computed color expression (for example, 'rgb(...)' or 'oklch(...)').
 */
export function resolveComputedCssColor(cssColorOrToken: string): string {
    if (typeof document === 'undefined') {
        return cssColorOrToken;
    }

    const trimmedValue = cssColorOrToken.trim();
    if (validateHexadecimalColor(trimmedValue)) {
        return trimmedValue;
    }

    const hostElement = document.body || document.documentElement;
    if (!hostElement) {
        return trimmedValue;
    }

    const colorExpression = trimmedValue.startsWith('--') ? `var(${trimmedValue})` : trimmedValue;
    const computedColor = measureElementComputedColor(colorExpression, hostElement);
    const resolvedColor = resolveCssVariableHierarchy(computedColor || trimmedValue);

    return resolvedColor || trimmedValue;
}

/**
 * Normalizes a short or full hexadecimal color.
 *
 * @param {string} colorExpression - The hexadecimal color expression.
 * @returns {string | null} The normalized color, or null when invalid.
 */
function normalizeHexadecimalColor(colorExpression: string): string | null {
    if (!validateHexadecimalColor(colorExpression)) return null;
    if (colorExpression.length === 4) {
        const [redCharacter, greenCharacter, blueCharacter] = colorExpression.slice(1);
        return `#${redCharacter}${redCharacter}${greenCharacter}${greenCharacter}${blueCharacter}${blueCharacter}`.toLowerCase();
    }
    return colorExpression.toLowerCase();
}

/**
 * Rasterizes a browser-supported CSS color while detecting rejected fillStyle values.
 *
 * @param {string} colorExpression - The resolved CSS color expression.
 * @returns {string | null} The hexadecimal color, or null when not supported.
 */
function rasterizeCssColor(colorExpression: string): string | null {
    const canvas = document.createElement('canvas');
    canvas.width = 1;
    canvas.height = 1;
    const context = canvas.getContext('2d', { willReadFrequently: true });
    if (!context) return null;

    context.fillStyle = '#010203';
    const firstSentinel = context.fillStyle;
    context.fillStyle = colorExpression;
    const firstAttempt = context.fillStyle;
    context.fillStyle = '#040506';
    const secondSentinel = context.fillStyle;
    context.fillStyle = colorExpression;
    const secondAttempt = context.fillStyle;
    if (firstAttempt === firstSentinel && secondAttempt === secondSentinel) return null;

    context.fillRect(0, 0, 1, 1);
    const [redChannel, greenChannel, blueChannel, alphaChannel] = context.getImageData(
        0,
        0,
        1,
        1
    ).data;
    if (alphaChannel === 0) return null;

    const redHexadecimal = redChannel.toString(16).padStart(2, '0');
    const greenHexadecimal = greenChannel.toString(16).padStart(2, '0');
    const blueHexadecimal = blueChannel.toString(16).padStart(2, '0');
    return `#${redHexadecimal}${greenHexadecimal}${blueHexadecimal}`;
}

/**
 * Converts a resolved CSS color through direct parsing or the browser canvas.
 *
 * @param {string} colorExpression - The resolved CSS color expression.
 * @returns {string | null} The hexadecimal color, or null when it cannot be converted.
 */
function convertResolvedColorToHexadecimal(colorExpression: string): string | null {
    const hexadecimalColor = normalizeHexadecimalColor(colorExpression);
    if (hexadecimalColor) return hexadecimalColor;

    const rgbHexadecimal = convertRgbColorToHexadecimal(colorExpression);
    if (rgbHexadecimal) return rgbHexadecimal;

    const oklchHexadecimal = convertOklchColorToHexadecimal(colorExpression);
    if (oklchHexadecimal) return oklchHexadecimal;

    return rasterizeCssColor(colorExpression);
}

/**
 * Converts any CSS color understood by the browser (including oklch and CSS custom properties) to `#rrggbb`
 * by resolving computed variables and rasterizing on a 1x1 canvas.
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
    if (typeof document === 'undefined') return null;

    const trimmedCssColor = cssColor.trim();
    if (!trimmedCssColor) return null;

    const directColor = convertResolvedColorToHexadecimal(trimmedCssColor);
    if (directColor) return directColor;

    const resolvedColor = resolveComputedCssColor(trimmedCssColor);
    return convertResolvedColorToHexadecimal(resolvedColor);
}
