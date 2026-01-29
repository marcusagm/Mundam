export interface ImageItem {
  id: number;
  path: string;
  filename: string;
  width: number | null;
  height: number | null;
  thumbnail_path: string | null;
  rating: number;
  notes: string | null;
  size: number;
  format: string;
  created_at: string;
  modified_at: string;
  added_at: string;
  folder_id: number;
}
