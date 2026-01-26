use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub color: Option<String>,
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
    ) -> Result<(), sqlx::Error> {
        // Dynamic update is tricky in SQLx without a builder, but we can do individual updates
        // or just assume we send all fields. For simplicity, let's assume specific update methods
        // or a comprehensive one.
        // Let's implement specific setters for now to be safe, or a full update if we expect full object.
        // User story mentions renaming, color, moving (parent_id).

        // We'll use a COALESCE approach or just dynamic query construction if needed.
        // But cleaner to just update what's passed if we use Option in arguments.
        // However, standard SQL update:

        // Let's do separate methods for atomic operations or a big one.
        // A big one is flexible.

        let mut query = "UPDATE tags SET ".to_string();
        let mut updates = Vec::new();

        // This is a naive query builder, but sufficient.
        if name.is_some() {
            updates.push("name = ?");
        }
        if color.is_some() {
            updates.push("color = ?");
        }
        if parent_id.is_some() {
            updates.push("parent_id = ?");
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
            q = q.bind(p);
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
            "SELECT id, name, parent_id, color FROM tags ORDER BY name ASC",
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
        match_all: bool, // true=AND, false=OR
    ) -> Result<Vec<crate::indexer::metadata::ImageMetadata>, sqlx::Error> {
        if tag_ids.is_empty() {
            return Ok(vec![]);
        }

        let placeholders: Vec<String> = tag_ids.iter().map(|_| "?".to_string()).collect();
        let query_str = if match_all {
            format!(
                "SELECT i.id, i.path, i.filename, i.width, i.height, i.size, i.thumbnail_path, i.created_at, i.modified_at
                 FROM images i
                 JOIN image_tags it ON i.id = it.image_id
                 WHERE it.tag_id IN ({})
                 GROUP BY i.id
                 HAVING COUNT(DISTINCT it.tag_id) = ?
                 ORDER BY i.id ASC
                 LIMIT ? OFFSET ?",
                placeholders.join(",")
            )
        } else {
            format!(
                "SELECT DISTINCT i.id, i.path, i.filename, i.width, i.height, i.size, i.thumbnail_path, i.created_at, i.modified_at
                 FROM images i
                 JOIN image_tags it ON i.id = it.image_id
                 WHERE it.tag_id IN ({})
                 ORDER BY i.id ASC
                 LIMIT ? OFFSET ?",
                placeholders.join(",")
            )
        };

        let mut q = sqlx::query_as::<_, crate::indexer::metadata::ImageMetadata>(&query_str);

        for id in &tag_ids {
            q = q.bind(id);
        }

        if match_all {
            q = q.bind(tag_ids.len() as i32);
        }

        q = q.bind(limit).bind(offset);

        let images = q.fetch_all(&self.pool).await?;
        Ok(images)
    }
}
