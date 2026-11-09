interface OklchComponents {
    lightness: number;
    chroma: number;
    hueInRadians: number;
}

/**
 * Parses an OKLCH expression into normalized components.
 *
 * @param {string} colorExpression - An oklch() expression.
 * @returns {OklchComponents | null} The parsed components, or null when invalid.
 */
function parseOklchComponents(colorExpression: string): OklchComponents | null {
    const functionMatch = colorExpression.match(
        /^oklch\(\s*([+-]?(?:\d+\.?\d*|\.\d+)%?)\s+([+-]?(?:\d+\.?\d*|\.\d+)%?)\s+([+-]?(?:\d+\.?\d*|\.\d+)(?:deg|rad|turn)?)\s*(?:\/\s*[^)]*)?\)$/i
    );
    if (!functionMatch) return null;

    const lightness = parseCssNumericComponent(functionMatch[1], 1);
    const chroma = parseCssNumericComponent(functionMatch[2], 0.4);
    if (lightness === null || chroma === null) return null;

    const hueExpression = functionMatch[3];
    const hueUnitMatch = hueExpression.match(/(deg|rad|turn)$/i);
    const hueValue = Number(
        hueUnitMatch ? hueExpression.slice(0, -hueUnitMatch[0].length) : hueExpression
    );
    if (!Number.isFinite(hueValue)) return null;

    const hueUnit = hueUnitMatch?.[0].toLowerCase();
    const hueInDegrees =
        hueUnit === 'rad'
            ? (hueValue * 180) / Math.PI
            : hueUnit === 'turn'
              ? hueValue * 360
              : hueValue;

    return {
        lightness,
        chroma,
        hueInRadians: (hueInDegrees * Math.PI) / 180
    };
}

/**
 * Parses a CSS numeric component, including percentage values.
 *
 * @param {string} component - The CSS component.
 * @param {number} percentageScale - The multiplier used to normalize percentages.
 * @returns {number | null} The parsed value, or null when invalid.
 */
function parseCssNumericComponent(component: string, percentageScale: number): number | null {
    const isPercentage = component.endsWith('%');
    const numericValue = Number(isPercentage ? component.slice(0, -1) : component);
    if (!Number.isFinite(numericValue)) return null;
    return isPercentage ? (numericValue / 100) * percentageScale : numericValue;
}

/**
 * Converts OKLab coordinates to linear sRGB channels.
 *
 * @param {OklchComponents} components - The OKLCH color components.
 * @param {number} chroma - The chroma used for this conversion.
 * @returns {[number, number, number]} The linear red, green, and blue channels.
 */
function convertOklchToLinearSrgb(
    components: OklchComponents,
    chroma: number
): [number, number, number] {
    const a = chroma * Math.cos(components.hueInRadians);
    const b = chroma * Math.sin(components.hueInRadians);
    const lightnessComponent = components.lightness + 0.3963377774 * a + 0.2158037573 * b;
    const greenComponent = components.lightness - 0.1055613458 * a - 0.0638541728 * b;
    const blueComponent = components.lightness - 0.0894841775 * a - 1.291485548 * b;
    const lightnessCubed = lightnessComponent ** 3;
    const greenCubed = greenComponent ** 3;
    const blueCubed = blueComponent ** 3;

    return [
        4.0767416621 * lightnessCubed - 3.3077115913 * greenCubed + 0.2309699292 * blueCubed,
        -1.2684380046 * lightnessCubed + 2.6097574011 * greenCubed - 0.3413193965 * blueCubed,
        -0.0041960863 * lightnessCubed - 0.7034186147 * greenCubed + 1.707614701 * blueCubed
    ];
}

/**
 * Checks whether linear RGB channels are inside the sRGB gamut.
 *
 * @param {[number, number, number]} channels - The linear RGB channels.
 * @returns {boolean} True if every channel is in gamut.
 */
function isWithinSrgbGamut(channels: [number, number, number]): boolean {
    return channels.every(channel => channel >= 0 && channel <= 1);
}

/**
 * Reduces chroma until an OKLCH color fits inside the sRGB gamut.
 *
 * @param {OklchComponents} components - The OKLCH color components.
 * @returns {[number, number, number]} The in-gamut linear RGB channels.
 */
function mapOklchToSrgbGamut(components: OklchComponents): [number, number, number] {
    let minimumChroma = 0;
    let maximumChroma = components.chroma;
    let linearChannels = convertOklchToLinearSrgb(components, maximumChroma);

    if (isWithinSrgbGamut(linearChannels)) return linearChannels;

    for (let iteration = 0; iteration < 16; iteration += 1) {
        const chroma = (minimumChroma + maximumChroma) / 2;
        linearChannels = convertOklchToLinearSrgb(components, chroma);
        if (isWithinSrgbGamut(linearChannels)) {
            minimumChroma = chroma;
        } else {
            maximumChroma = chroma;
        }
    }

    return convertOklchToLinearSrgb(components, minimumChroma);
}

/**
 * Converts a linear sRGB channel to its gamma-encoded value.
 *
 * @param {number} channel - The linear channel.
 * @returns {number} The gamma-encoded channel.
 */
function convertLinearChannelToSrgb(channel: number): number {
    return channel <= 0.0031308 ? channel * 12.92 : 1.055 * channel ** (1 / 2.4) - 0.055;
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
 * Converts an OKLCH color to hexadecimal, mapping out-of-gamut colors to sRGB.
 *
 * @param {string} colorExpression - An oklch() expression.
 * @returns {string | null} The hexadecimal color, or null when the expression is invalid.
 */
export function convertOklchColorToHexadecimal(colorExpression: string): string | null {
    const components = parseOklchComponents(colorExpression);
    if (!components) return null;

    const linearChannels = mapOklchToSrgbGamut(components);
    const [redChannel, greenChannel, blueChannel] = linearChannels.map(convertLinearChannelToSrgb);
    return formatSrgbHexadecimal(redChannel, greenChannel, blueChannel);
}
