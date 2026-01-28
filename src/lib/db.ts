import { invoke } from "@tauri-apps/api/core";

// We primarily use the Rust backend for DB operations now.
// This file wraps those invocations or provides legacy support where needed.

export async function initDb() {
  console.log("Database management is handled by the Backend (Rust).");
  // No-op or perform specific frontend-only inits if needed
}

export async function addLocation(path: string) {
    return await invoke("add_location", { path });
}

export async function getLocations() {
  return await invoke<any[]>("get_locations");
}

export async function getImages(limit: number = 100, offset: number = 0) {
  // Use the backend command which now handles the unified logic
  return await invoke<any[]>("get_images_filtered", {
    limit,
    offset,
    tagIds: [],
    matchAll: true,
    untagged: false, // optional
    folderId: null,      // was locationId
    recursive: true   // default to recursive or not? context implies simple "get recent"
  });
}
