
/**
 * Global application constants.
 * These values are used throughout the application to maintain consistency.
 */

export const APP_CONFIG = {
  /**
   * Number of images to load per batch in the library view (infinite scroll).
   */
  BATCH_SIZE: 100,

  /**
   * Debounce delay (in ms) for search inputs.
   */
  SEARCH_DEBOUNCE_MS: 300,

  /**
   * Default thumbnail size (in px) requested from the backend.
   */
  THUMBNAIL_SIZE: 300,

  /**
   * Maximum depth for recursive folder operations to prevent infinite loops.
   */
  MAX_FOLDER_DEPTH: 50,
};
