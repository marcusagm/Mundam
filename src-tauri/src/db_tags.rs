use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub color: Option<String>,
    #[sqlx(default)]
    pub order_index: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagCount {
    pub tag_id: i64,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FolderCount {
    pub folder_id: i64,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryStats {
    pub total_images: i64,
    pub untagged_images: i64,
    pub tag_counts: Vec<TagCount>,
    pub folder_counts: Vec<FolderCount>, // Direct counts
    pub folder_counts_recursive: Vec<FolderCount>, // Recursive counts
}

use crate::database::Db;

impl Db {
    pub async fn create_tag(
        &self,
        name: &str,
        parent_id: Option<i64>,
        color: Option<String>,
    ) -> Result<i64, sqlx::Error> {
        let res = sqlx::query("INSERT INTO tags (name, parent_id, color) VALUES (?, ?, ?)")
            .bind(name)
            .bind(parent_id)
            .bind(color)
            .execute(&self.pool)
            .await?;

        Ok(res.last_insert_rowid())
    }

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

        if name.is_some() {
            updates.push("name = ?");
        }
        if color.is_some() {
            updates.push("color = ?");
        }
        if parent_id.is_some() {
            updates.push("parent_id = ?");
        }
        if order_index.is_some() {
            updates.push("order_index = ?");
        }

        if updates.is_empty() {
            return Ok(());
        }

        query.push_str(&updates.join(", "));
        query.push_str(" WHERE id = ?");

        let mut q = sqlx::query(&query);

        if let Some(n) = name {
            q = q.bind(n);
        }
        if let Some(c) = color {
            q = q.bind(c);
        }
        if let Some(p) = parent_id {
            if p == 0 {
                q = q.bind(None::<i64>);
            } else {
                q = q.bind(p);
            }
        }
        if let Some(o) = order_index {
            q = q.bind(o);
        }

        q = q.bind(id);

        q.execute(&self.pool).await?;
        Ok(())
    }

    pub async fn delete_tag(&self, id: i64) -> Result<(), sqlx::Error> {
        // ON DELETE SET NULL is in schema for parent_id
        // ON DELETE CASCADE is in schema for image_tags
        sqlx::query("DELETE FROM tags WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_all_tags(&self) -> Result<Vec<Tag>, sqlx::Error> {
        let tags = sqlx::query_as::<_, Tag>(
            "SELECT id, name, parent_id, color, order_index FROM tags ORDER BY order_index ASC, name ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(tags)
    }

    pub async fn add_tag_to_image(&self, image_id: i64, tag_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO image_tags (image_id, tag_id) VALUES (?, ?) ON CONFLICT DO NOTHING",
        )
        .bind(image_id)
        .bind(tag_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn remove_tag_from_image(
        &self,
        image_id: i64,
        tag_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM image_tags WHERE image_id = ? AND tag_id = ?")
            .bind(image_id)
            .bind(tag_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_tags_for_image(&self, image_id: i64) -> Result<Vec<Tag>, sqlx::Error> {
        let tags = sqlx::query_as::<_, Tag>(
            "SELECT t.id, t.name, t.parent_id, t.color 
             FROM tags t
             JOIN image_tags it ON t.id = it.tag_id
             WHERE it.image_id = ?",
        )
        .bind(image_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(tags)
    }

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
                sqlx::query("INSERT INTO image_tags (image_id, tag_id) VALUES (?, ?) ON CONFLICT DO NOTHING")
                    .bind(img_id)
                    .bind(tag_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_images_filtered(
        &self,
        limit: i32,
        offset: i32,
        tag_ids: Vec<i64>,
        match_all: bool,
        untagged: Option<bool>,
        folder_id: Option<i64>,
        recursive: bool,
    ) -> Result<Vec<crate::indexer::metadata::ImageMetadata>, sqlx::Error> {
        let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> = sqlx::QueryBuilder::new(
            "WITH RECURSIVE target_folders AS (
               SELECT id FROM folders WHERE id = "
        );

        if let Some(fid) = folder_id {
            query_builder.push_bind(fid);
            if recursive {
                query_builder.push(" UNION ALL SELECT f.id FROM folders f JOIN target_folders tf ON f.parent_id = tf.id");
            }
        } else {
             // If no folder selected, effectively selecting all folders? 
             // Or maybe we just don't use the CTE if folder_id is None.
             // But for cleaner building, let's just put a dummy condition if None or handle it below.
             query_builder.push(" -1 "); // Dummy ID if none provided, handled in WHERE clause
        }

        query_builder.push(") SELECT DISTINCT i.id, i.path, i.filename, i.width, i.height, i.size, i.thumbnail_path, i.format, i.rating, i.notes, i.created_at, i.modified_at FROM images i ");

        if !tag_ids.is_empty() {
            query_builder.push(" JOIN image_tags it ON i.id = it.image_id ");
        }

        query_builder.push(" WHERE 1=1 ");

        if let Some(_) = folder_id {
            if recursive {
                query_builder.push(" AND i.folder_id IN target_folders ");
            } else {
                query_builder.push(" AND i.folder_id = ");
                query_builder.push_bind(folder_id.unwrap());
            }
        }

        if untagged == Some(true) {
            query_builder.push(" AND i.id NOT IN (SELECT DISTINCT image_id FROM image_tags) ");
        }

        if !tag_ids.is_empty() {
            query_builder.push(" AND it.tag_id IN (");
            let mut separated = query_builder.separated(", ");
            for id in &tag_ids {
                separated.push_bind(id);
            }
            separated.push_unseparated(") ");

            if match_all {
                query_builder.push(" GROUP BY i.id HAVING COUNT(DISTINCT it.tag_id) = ");
                query_builder.push_bind(tag_ids.len() as i32);
            }
        }

        query_builder.push(" ORDER BY i.id ASC LIMIT ");
        query_builder.push_bind(limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);

        let query = query_builder.build_query_as::<crate::indexer::metadata::ImageMetadata>();
        let images = query.fetch_all(&self.pool).await?;
        Ok(images)
    }
    pub async fn get_library_stats(&self) -> Result<LibraryStats, sqlx::Error> {
        let total_images: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM images")
            .fetch_one(&self.pool)
            .await?;

        let untagged_images: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM images WHERE id NOT IN (SELECT DISTINCT image_id FROM image_tags)"
        )
        .fetch_one(&self.pool)
        .await?;

        let tag_counts = sqlx::query_as::<_, (i64, i64)>(
            "SELECT tag_id, COUNT(*) FROM image_tags GROUP BY tag_id",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(tag_id, count)| TagCount { tag_id, count })
        .collect();

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
            total_images: total_images.0,
            untagged_images: untagged_images.0,
            tag_counts,
            folder_counts,
            folder_counts_recursive,
        })
    }
}
