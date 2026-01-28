import { invoke } from "@tauri-apps/api/core";

export interface Tag {
  id: number;
  name: string;
  parent_id: number | null;
  color: string | null;
  order_index: number;
}

export interface LibraryStats {
  total_images: number;
  untagged_images: number;
  tag_counts: { tag_id: number; count: number }[];
  folder_counts: { location_id: number; count: number }[];
}

export const tagService = {
  createTag: async (name: string, parent_id?: number | null, color?: string | null): Promise<number> => {
    return await invoke("create_tag", { name, parentId: parent_id, color });
  },

  updateTag: async (id: number, name?: string | null, color?: string | null, parent_id?: number | null, order_index?: number | null): Promise<void> => {
    return await invoke("update_tag", { id, name, color, parentId: parent_id, orderIndex: order_index });
  },

  deleteTag: async (id: number): Promise<void> => {
    return await invoke("delete_tag", { id });
  },

  getAllTags: async (): Promise<Tag[]> => {
    return await invoke("get_all_tags");
  },

  getLibraryStats: async (): Promise<LibraryStats> => {
    return await invoke("get_library_stats");
  },

  addTagsToImagesBatch: async (imageIds: number[], tagIds: number[]): Promise<void> => {
    return await invoke("add_tags_to_images_batch", { imageIds, tagIds });
  },

  getTagsForImage: async (imageId: number): Promise<Tag[]> => {
    return await invoke("get_tags_for_image", { imageId });
  },

  removeTagFromImage: async (imageId: number, tagId: number): Promise<void> => {
    return await invoke("remove_tag_from_image", { imageId, tagId });
  },

  getImagesFiltered: async (
    limit: number, 
    offset: number, 
    tagIds: number[], 
    matchAll: boolean = true,
    untagged?: boolean,
    locationId?: number,
    subfolderId?: number
  ): Promise<any[]> => {
    return await invoke("get_images_filtered", { 
      limit, 
      offset, 
      tagIds, 
      matchAll,
      untagged,
      locationId,
      subfolderId
    });
  },

  updateImageRating: async (id: number, rating: number): Promise<void> => {
    return await invoke("update_image_rating", { id, rating });
  },

  updateImageNotes: async (id: number, notes: string): Promise<void> => {
    return await invoke("update_image_notes", { id, notes });
  },

  getImageExif: async (path: string): Promise<Record<string, string>> => {
    return await invoke("get_image_exif", { path });
  }
};
