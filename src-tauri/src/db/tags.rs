//! Tag management and image-tag relationship queries.

use crate::db::models::{Tag, TagCount, LibraryStats, FolderCount};
use super::Db;

impl Db {
    /// Creates a new tag in the database.
    ///
    /// # Arguments
    ///
    /// * `name` - The unique name of the tag.
    /// * `parent_id` - Optional ID of a parent tag for hierarchical organization.
    /// * `color` - Optional hex color code for the tag.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the database operation fails.
    pub async fn create_tag(
        &self,
        name: &str,
        parent_id: Option<i64>,
        color: Option<String>,
    ) -> Result<i64, sqlx::Error> {
        let res = sqlx::query!("INSERT INTO tags (name, parent_id, color) VALUES (?, ?, ?)", name, parent_id, color)
            .execute(&self.pool)
            .await?;

        Ok(res.last_insert_rowid())
    }

    /// Updates an existing tag's properties.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the tag doesn't exist or database fails.
    pub async fn update_tag(
        &self,
        id: i64,
        name: Option<String>,
        color: Option<String>,
        parent_id: Option<i64>,
        order_index: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        let mut query = "UPDATE tags SET ".to_string();
        let mut updates = Vec::new();

        if name.is_some() { updates.push("name = ?"); }
        if color.is_some() { updates.push("color = ?"); }
        if parent_id.is_some() { updates.push("parent_id = ?"); }
        if order_index.is_some() { updates.push("order_index = ?"); }

        if updates.is_empty() {
            return Ok(());
        }

        query.push_str(&updates.join(", "));
        query.push_str(" WHERE id = ?");

        let mut q = sqlx::query(&query);

        if let Some(n) = name { q = q.bind(n); }
        if let Some(c) = color { q = q.bind(c); }
        if let Some(p) = parent_id {
            if p == 0 { q = q.bind(None::<i64>); } else { q = q.bind(p); }
        }
        if let Some(o) = order_index { q = q.bind(o); }

        q = q.bind(id);

        q.execute(&self.pool).await?;
        Ok(())
    }

    /// Deletes a tag and removes all its associations with images.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the database fails.
    pub async fn delete_tag(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM tags WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Retrieves all tags from the database, ordered by their index and name.
    pub async fn get_all_tags(&self) -> Result<Vec<Tag>, sqlx::Error> {
        let tags = sqlx::query_as!(
            Tag,
            "SELECT id as \"id!\", name, parent_id, color, order_index as \"order_index!\" FROM tags ORDER BY order_index ASC, name ASC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(tags)
    }

    /// Associates a tag with an image.
    pub async fn add_tag_to_image(&self, image_id: i64, tag_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO image_tags (image_id, tag_id) VALUES (?, ?) ON CONFLICT DO NOTHING",
            image_id,
            tag_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Removes an association between a tag and an image.
    pub async fn remove_tag_from_image(&self, image_id: i64, tag_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM image_tags WHERE image_id = ? AND tag_id = ?",
            image_id,
            tag_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Gets all tags associated with a specific image.
    pub async fn get_tags_for_image(&self, image_id: i64) -> Result<Vec<Tag>, sqlx::Error> {
        let tags = sqlx::query_as!(
            Tag,
            r#"SELECT t.id as "id!", t.name, t.parent_id, t.color, t.order_index as "order_index!"
               FROM tags t
               JOIN image_tags it ON t.id = it.tag_id
               WHERE it.image_id = ?
               ORDER BY t.order_index ASC, t.name ASC"#,
            image_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(tags)
    }

    /// Batch associates multiple tags with multiple images in a single transaction.
    pub async fn add_tags_to_images_batch(
        &self,
        image_ids: Vec<i64>,
        tag_ids: Vec<i64>,
    ) -> Result<(), sqlx::Error> {
        if image_ids.is_empty() || tag_ids.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        for img_id in &image_ids {
            for tag_id in &tag_ids {
                sqlx::query!(
                    "INSERT INTO image_tags (image_id, tag_id) VALUES (?, ?) ON CONFLICT DO NOTHING",
                    img_id,
                    tag_id
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    /// Calculates high-level library statistics.
    pub async fn get_library_stats(&self) -> Result<LibraryStats, sqlx::Error> {
        let total_images = sqlx::query_scalar!("SELECT COUNT(*) FROM images")
            .fetch_one(&self.pool)
            .await? as i64;

        let untagged_images = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM images WHERE id NOT IN (SELECT DISTINCT image_id FROM image_tags)"
        )
        .fetch_one(&self.pool)
        .await? as i64;

        let tag_counts = sqlx::query_as!(
            TagCount,
            "SELECT tag_id, COUNT(*) as count FROM image_tags GROUP BY tag_id"
        )
        .fetch_all(&self.pool)
        .await?;

        let folder_counts = self.get_folder_counts_direct()
            .await?
            .into_iter()
            .map(|(folder_id, count)| FolderCount { folder_id, count })
            .collect();

        let folder_counts_recursive = self.get_folder_counts_recursive()
            .await?
            .into_iter()
            .map(|(folder_id, count)| FolderCount { folder_id, count })
            .collect();

        Ok(LibraryStats {
            total_images,
            untagged_images,
            tag_counts,
            folder_counts,
            folder_counts_recursive,
        })
    }
}
