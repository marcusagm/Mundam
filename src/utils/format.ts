/**
 * Formats a file size in bytes to a human-readable string (e.g., "1.5 MB").
 *
 * @param {number} bytes - The size of the file in bytes.
 * @returns {string} The formatted file size with its appropriate unit.
 *
 * @example
 * const readableSize = formatFileSize(1024); // Returns "1 KB"
 */
export function formatFileSize(bytes: number): string {
    if (bytes === 0) return '0 B';
    const bytesPerKilobyte = 1024;
    const sizeUnitList = ['B', 'KB', 'MB', 'GB', 'TB'];
    const unitIndex = Math.floor(Math.log(bytes) / Math.log(bytesPerKilobyte));
    return (
        parseFloat((bytes / Math.pow(bytesPerKilobyte, unitIndex)).toFixed(2)) +
        ' ' +
        sizeUnitList[unitIndex]
    );
}

/**
 * Formats a date string or Date object to a localized alphanumeric string including time.
 *
 * @param {string | Date} dateSource - The date string or object to format.
 * @returns {string} The formatted date string in localized format, or "-" if input is empty.
 */
export function formatDate(dateSource: string | Date): string {
    if (!dateSource) return '-';
    const dateObject = typeof dateSource === 'string' ? new Date(dateSource) : dateSource;
    return new Intl.DateTimeFormat(navigator.language, {
        year: 'numeric',
        month: 'short',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit'
    }).format(dateObject);
}

/**
 * Formats a date to a short localized numeric representation (e.g., "DD/MM/YY").
 *
 * @param {string | Date} dateSource - The date source string or object to format.
 * @returns {string} The short formatted date string.
 */
export function formatShortDate(dateSource: string | Date): string {
    if (!dateSource) return '-';
    const dateObject = typeof dateSource === 'string' ? new Date(dateSource) : dateSource;
    return new Intl.DateTimeFormat(navigator.language, {
        year: '2-digit',
        month: '2-digit',
        day: '2-digit'
    }).format(dateObject);
}

/**
 * Formats a Date object or string to an ISO standard date string (YYYY-MM-DD).
 *
 * @param {Date | string} dateSource - The date to convert to ISO string.
 * @returns {string} The formatted ISO date string or the original string if conversion is not applicable.
 */
export const formatToISO = (dateSource: Date | string): string => {
    if (dateSource instanceof Date) {
        const year = dateSource.getFullYear();
        const month = (dateSource.getMonth() + 1).toString().padStart(2, '0');
        const day = dateSource.getDate().toString().padStart(2, '0');
        return `${year}-${month}-${day}`;
    }
    return String(dateSource);
};

/**
 * Parses an ISO standard date string (YYYY-MM-DD) to a Date object.
 *
 * @param {string} isoString - The ISO standard date string to parse.
 * @returns {Date | null} The corresponding Date object, or null if the string is invalid or cannot be parsed.
 */
export const fromISO = (isoString: string): Date | null => {
    if (!isoString || typeof isoString !== 'string') return null;
    const dateParts = isoString.split('-');
    if (dateParts.length === 3) {
        const year = parseInt(dateParts[0], 10);
        const month = parseInt(dateParts[1], 10) - 1;
        const day = parseInt(dateParts[2], 10);
        const dateObject = new Date(year, month, day);
        return isNaN(dateObject.getTime()) ? null : dateObject;
    }
    return null;
};

/**
 * Formats a Date object to a display-ready string (DD/MM/YYYY).
 *
 * @param {Date} date - The date object to format.
 * @returns {string} The formatted display date string.
 */
export const formatDateToDisplay = (date: Date): string => {
    if (!(date instanceof Date) || isNaN(date.getTime())) return '';
    const day = date.getDate().toString().padStart(2, '0');
    const month = (date.getMonth() + 1).toString().padStart(2, '0');
    const year = date.getFullYear().toString();
    return `${day}/${month}/${year}`;
};

/**
 * Parses a display date string (DD/MM/YYYY) to a Date object.
 *
 * @param {string} displayString - The string in DD/MM/YYYY format.
 * @returns {Date | null} The corresponding date object, or null if invalid.
 */
export const parseDisplayDate = (displayString: string): Date | null => {
    if (!displayString || displayString.length < 10) return null;
    const parts = displayString.split('/');
    if (parts.length !== 3) return null;

    const day = parseInt(parts[0], 10);
    const month = parseInt(parts[1], 10) - 1; // Month is 0-indexed
    const year = parseInt(parts[2], 10);

    const date = new Date(year, month, day);

    // Validation to ensure it's a real date (e.g. not 31/02/2021)
    if (date.getFullYear() === year && date.getMonth() === month && date.getDate() === day) {
        return date;
    }

    return null;
};

/**
 * Formats a date source (ISO string or Date object) to a localized display format (DD/MM/YYYY).
 *
 * @param {string | Date} dateSource - The date source to format.
 * @returns {string} The formatted display date string, or an empty string if invalid.
 */
export const formatToDisplay = (dateSource: string | Date): string => {
    if (dateSource instanceof Date) {
        return formatDateToDisplay(dateSource);
    }
    if (typeof dateSource !== 'string') return '';

    const dateObject = fromISO(dateSource);
    return dateObject ? formatDateToDisplay(dateObject) : dateSource;
};

/**
 * Formats a given time in seconds into a traditional HH:MM:SS or MM:SS format.
 *
 * @param {number} seconds - The time duration in seconds.
 * @returns {string} The formatted time string.
 *
 * @example
 * const formatted = formatTime(125); // "02:05"
 */
export const formatTime = (seconds: number): string => {
    if (!Number.isFinite(seconds) || Number.isNaN(seconds)) return '--:--';
    const totalHours = Math.floor(seconds / 3600);
    const totalMinutes = Math.floor((seconds % 3600) / 60);
    const totalSeconds = Math.floor(seconds % 60);
    const formattedParts = [totalMinutes, totalSeconds].map(value =>
        value.toString().padStart(2, '0')
    );
    if (totalHours > 0) formattedParts.unshift(totalHours.toString());
    return formattedParts.join(':');
};

/**
 * Formats a numeric count into a shorter string representation (e.g., 1.2k, 1M).
 *
 * @param {number} numericCount - The count to abbreviate.
 * @param {number} maximumValue - Optional. The maximum value to show before appending a '+' sign.
 * @returns {string} The formatted abbreviated count string.
 */
export const formatCompactNumber = (numericCount: number, maximumValue: number = 9999): string => {
    if (numericCount > maximumValue) {
        return `${formatCompactNumber(maximumValue, maximumValue)}+`;
    }
    if (numericCount >= 1000000) {
        return (numericCount / 1000000).toFixed(1).replace(/\.0$/, '') + 'M';
    }
    if (numericCount >= 1000) {
        return (numericCount / 1000).toFixed(1).replace(/\.0$/, '') + 'k';
    }
    return numericCount.toString();
};

interface RelativeTimeInterval {
    readonly secondsInUnit: number;
    readonly singularLabel: string;
    readonly pluralLabel: string;
}

const RELATIVE_TIME_INTERVAL_LIST: readonly RelativeTimeInterval[] = [
    { secondsInUnit: 31536000, singularLabel: '1 year ago', pluralLabel: 'years ago' },
    { secondsInUnit: 2592000, singularLabel: '1 month ago', pluralLabel: 'months ago' },
    { secondsInUnit: 86400, singularLabel: 'Yesterday', pluralLabel: 'days ago' },
    { secondsInUnit: 3600, singularLabel: '1 hour ago', pluralLabel: 'hours ago' },
    { secondsInUnit: 60, singularLabel: '1 minute ago', pluralLabel: 'minutes ago' }
];

/**
 * Formats a date string or Date object to a relative human-readable time string (e.g., "Just now", "5 minutes ago", "2 days ago").
 *
 * @param {string | Date} dateSource - The date string or Date object to format.
 * @returns {string} The relative formatted time string.
 *
 * @example
 * const relativeTime = formatRelativeTime("2026-10-20T01:00:00Z");
 */
export function formatRelativeTime(dateSource: string | Date): string {
    if (!dateSource) return '-';
    const dateObject = typeof dateSource === 'string' ? new Date(dateSource) : dateSource;
    const targetTimestampMilliseconds = dateObject.getTime();
    if (isNaN(targetTimestampMilliseconds)) return '-';

    const elapsedSeconds = Math.floor((Date.now() - targetTimestampMilliseconds) / 1000);

    if (elapsedSeconds < 60) {
        return 'Just now';
    }

    for (const timeInterval of RELATIVE_TIME_INTERVAL_LIST) {
        const unitsCount = Math.floor(elapsedSeconds / timeInterval.secondsInUnit);
        if (unitsCount >= 1) {
            return unitsCount === 1
                ? timeInterval.singularLabel
                : `${unitsCount} ${timeInterval.pluralLabel}`;
        }
    }

    return 'Just now';
}

/**
 * Converts free text into a lowercase, dash-separated slug suitable for file names.
 *
 * @param {string} text - The text to convert.
 * @param {string} [fallback] - Value returned when the slug would be empty.
 * @returns {string} The slug.
 *
 * @example
 * ```ts
 * formatSlug('My Ocean Theme!'); // 'my-ocean-theme'
 * ```
 */
export const formatSlug = (text: string, fallback: string = 'untitled'): string => {
    const slug = text
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '-')
        .replace(/^-|-$/g, '');
    return slug || fallback;
};
