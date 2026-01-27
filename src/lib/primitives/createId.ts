let counter = 0;

/**
 * Creates a unique ID for accessibility attributes.
 * Uses a simple counter for stable IDs during SSR and hydration.
 * 
 * @param prefix - Optional prefix for the ID
 */
export function createId(prefix = "ui"): string {
  counter += 1;
  return `${prefix}-${counter}`;
}
