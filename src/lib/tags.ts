import { invoke } from "@tauri-apps/api/core";

export interface Tag {
  id: number;
  name: string;
  parent_id: number | null;
  color: string | null;
}

export const tagService = {
  createTag: async (name: string, parent_id?: number | null, color?: string | null): Promise<number> => {
    return await invoke("create_tag", { name, parentId: parent_id, color });
  },

  updateTag: async (id: number, name?: string | null, color?: string | null, parent_id?: number | null): Promise<void> => {
    return await invoke("update_tag", { id, name, color, parentId: parent_id });
  },

  deleteTag: async (id: number): Promise<void> => {
    return await invoke("delete_tag", { id });
  },

  getAllTags: async (): Promise<Tag[]> => {
    return await invoke("get_all_tags");
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

  getImagesFiltered: async (limit: number, offset: number, tagIds: number[], matchAll: boolean = true): Promise<any[]> => {
    return await invoke("get_images_filtered", { limit, offset, tagIds, matchAll });
  }
};
