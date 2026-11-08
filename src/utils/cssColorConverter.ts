import { validateHexadecimalColor } from './color';

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
 * Resolves a CSS custom property or color string by measuring a probe element attached to the DOM.
 * This guarantees that CSS variables like `var(--p-neutral-900)` and custom property names
 * are fully computed to their canonical `rgb(...)` or `rgba(...)` representation across all browsers
 * (specifically resolving the WebKit canvas limitation on macOS).
 *
 * @param {string} cssColorOrToken - The CSS color expression or token name.
 * @returns {string} The computed color string (e.g., 'rgb(24, 24, 27)').
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

    if (validateHexadecimalColor(trimmedCssColor)) {
        if (trimmedCssColor.length === 4) {
            const [redCharacter, greenCharacter, blueCharacter] = trimmedCssColor.slice(1);
            return `#${redCharacter}${redCharacter}${greenCharacter}${greenCharacter}${blueCharacter}${blueCharacter}`.toLowerCase();
        }
        return trimmedCssColor.toLowerCase();
    }

    const resolvedColor = resolveComputedCssColor(trimmedCssColor);

    if (validateHexadecimalColor(resolvedColor)) {
        if (resolvedColor.length === 4) {
            const [redCharacter, greenCharacter, blueCharacter] = resolvedColor.slice(1);
            return `#${redCharacter}${redCharacter}${greenCharacter}${greenCharacter}${blueCharacter}${blueCharacter}`.toLowerCase();
        }
        return resolvedColor.toLowerCase();
    }

    const redGreenBlueMatch = resolvedColor.match(
        /^rgba?\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})/i
    );
    if (redGreenBlueMatch) {
        const redNumber = Math.min(255, Math.max(0, parseInt(redGreenBlueMatch[1], 10)));
        const greenNumber = Math.min(255, Math.max(0, parseInt(redGreenBlueMatch[2], 10)));
        const blueNumber = Math.min(255, Math.max(0, parseInt(redGreenBlueMatch[3], 10)));

        const redHexadecimal = redNumber.toString(16).padStart(2, '0');
        const greenHexadecimal = greenNumber.toString(16).padStart(2, '0');
        const blueHexadecimal = blueNumber.toString(16).padStart(2, '0');

        return `#${redHexadecimal}${greenHexadecimal}${blueHexadecimal}`;
    }

    const canvas = document.createElement('canvas');
    canvas.width = 1;
    canvas.height = 1;
    const context = canvas.getContext('2d', { willReadFrequently: true });
    if (!context) return null;

    context.clearRect(0, 0, 1, 1);
    context.fillStyle = '#000';
    context.fillStyle = resolvedColor;
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
