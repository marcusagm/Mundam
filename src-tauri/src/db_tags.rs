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
        sort_by: Option<String>,
        sort_order: Option<String>,
        advanced_query: Option<String>,
        search_query: Option<String>,
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
             query_builder.push(" -1 "); 
        }

        query_builder.push(") SELECT DISTINCT i.id, i.path, i.filename, i.width, i.height, i.size, i.thumbnail_path, i.format, i.rating, i.notes, i.created_at, i.modified_at, i.added_at FROM images i ");

        if !tag_ids.is_empty() {
            query_builder.push(" JOIN image_tags it ON i.id = it.image_id ");
        }

        query_builder.push(" WHERE 1=1 ");

        let _group_storage: Option<crate::search_logic::SearchGroup>;
        _group_storage = if let Some(filter) = advanced_query {
            match serde_json::from_str::<crate::search_logic::SearchGroup>(&filter) {
                Ok(group) => Some(group),
                Err(_) => None,
            }
        } else {
            None
        };

        if let Some(group) = &_group_storage {
            query_builder.push(" AND ");
            crate::search_logic::build_where_clause(group, &mut query_builder);
        }

        if let Some(search) = search_query {
            if !search.is_empty() {
                query_builder.push(" AND (i.filename LIKE ");
                query_builder.push_bind(format!("%{}%", search));
                query_builder.push(" OR i.notes LIKE ");
                query_builder.push_bind(format!("%{}%", search));
                query_builder.push(") ");
            }
        }

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

        // Sorting Logic
        let allowed_cols = ["filename", "created_at", "modified_at", "added_at", "size", "format", "rating"];
        let final_sort_by = sort_by.as_deref().filter(|c| allowed_cols.contains(c)).unwrap_or("id");
        let final_order = sort_order.as_deref().filter(|o| *o == "asc" || *o == "desc").unwrap_or("desc");

        query_builder.push(" ORDER BY ");
        
        // Show NULLS last: (column IS NULL) ASC, column [ASC|DESC]
        query_builder.push(" (");
        query_builder.push(final_sort_by);
        query_builder.push(" IS NULL) ASC, ");
        
        query_builder.push(final_sort_by);
        
        // Case-insensitive sorting for strings
        if ["filename", "format"].contains(&final_sort_by) {
            query_builder.push(" COLLATE NOCASE ");
        }

        query_builder.push(" ");
        query_builder.push(final_order);

        // Secondary sorting by filename for stability when primary values are equal
        if final_sort_by != "filename" {
            query_builder.push(", filename COLLATE NOCASE ASC");
            // query_builder.push(final_order);
        }

        query_builder.push(" LIMIT ");
        query_builder.push_bind(limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);

        let query = query_builder.build_query_as::<crate::indexer::metadata::ImageMetadata>();
        let images = query.fetch_all(&self.pool).await?;
        Ok(images)
    }
    pub async fn get_image_count_filtered(
        &self,
        tag_ids: Vec<i64>,
        match_all: bool,
        untagged: Option<bool>,
        folder_id: Option<i64>,
        recursive: bool,
        advanced_query: Option<String>,
        search_query: Option<String>,
    ) -> Result<i64, sqlx::Error> {
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
             query_builder.push(" -1 "); 
        }

        query_builder.push(") SELECT DISTINCT i.id FROM images i ");

        if !tag_ids.is_empty() {
            query_builder.push(" JOIN image_tags it ON i.id = it.image_id ");
        }

        query_builder.push(" WHERE 1=1 ");

        let _group_storage: Option<crate::search_logic::SearchGroup>;
        _group_storage = if let Some(filter) = advanced_query {
            match serde_json::from_str::<crate::search_logic::SearchGroup>(&filter) {
                Ok(group) => Some(group),
                Err(_) => None,
            }
        } else {
            None
        };

        if let Some(group) = &_group_storage {
            query_builder.push(" AND ");
            crate::search_logic::build_where_clause(group, &mut query_builder);
        }

        if let Some(search) = search_query {
            if !search.is_empty() {
                query_builder.push(" AND (i.filename LIKE ");
                query_builder.push_bind(format!("%{}%", search));
                query_builder.push(" OR i.notes LIKE ");
                query_builder.push_bind(format!("%{}%", search));
                query_builder.push(") ");
            }
        }

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
                // For count, we need a slightly different logic if using GROUP BY
                // SELECT COUNT(*) FROM (SELECT i.id ... GROUP BY i.id HAVING ...)
                // But let's append the HAVING clause directly if possible. 
                // However, COUNT(DISTINCT i.id) with GROUP BY returns multiple rows (one count per group).
                // We need to wrap it or use a subquery. 
                // Easier: Use the exact same query structure as `get_images` but select i.id, then wrap in `SELECT COUNT(*) FROM (...)`
                // But QueryBuilder doesn't easily support wrapping *after* building.
                
                // ADJUSTMENT: We push the GROUP BY at the end.
                // The query becomes: ... WHERE ... GROUP BY i.id HAVING ... 
                // The result of this is a list of IDs. 
                // sqlx::query_scalar can't count the rows directly from that output.
                
                // RE-STRATEGY: Use a subquery for the main logic.
                // "SELECT COUNT(*) FROM (SELECT i.id FROM ... WHERE ... GROUP BY ... HAVING ...)"
                // But constructing that with the builder is messy.
                // Alternative: The filter `match_all` is the only one adding GROUP BY.
                // If match_all is false, we don't group, we just join.
            }
        }
        
        // RE-WRITING THE BUILDER INITIALIZATION TO SUPPORT WRAPPING FOR MATCH_ALL
        // Since we are already deep in the copy-paste, let's just solve the `match_all` case.
        // If match_all is TRUE, we need to count the returned Groups.
        // It's robust to always wrap: SELECT COUNT(*) FROM ( <Original Logic returning IDs> )
        
        // Let's Restart the builder with the wrapper if we are doing complex aggregation?
        // Actually, let's keep it simple. If `match_all` is true, we simply fetch all IDs and count in Rust (less performant but accurate and reuses logic).
        // NO, that defeats the purpose of "Total" for massive libraries.
        
        // Correct SQL for "Count matching groups":
        // SELECT COUNT(*) FROM (SELECT i.id FROM ... GROUP BY i.id HAVING ...) as sub
        
        // Let's assume for this specific method, we handle the wrapping logic manually by pushing the prefix first.
        
        // BUT, `query_builder` is linear.
        // I will use `query_builder` to build the inner query, then wrap it? No.
        
        // Let's stick to the simpler implementation:
        // `get_image_count_filtered` will be a near-duplicate, but handled carefully.
        
        // If `match_all` is active:
        // We can't easily validly utilize `COUNT(DISTINCT i.id)` combined with `HAVING`.
        // So we will change the SELECT to `SELECT COUNT(*) FROM (SELECT i.id ...` at the start? No.
        
        // Hack: If match_all is true, we just return `query.fetch_all().len()`. 
        // We select `i.id`. It's only IDs, so it's lightweight.
        // Ideally we want `SELECT COUNT(*)`.
        
        // Let's use `SELECT COUNT(DISTINCT i.id)` generally.
        // If `match_all` is set, we unfortunately must use a subquery or CTE.
        
        // Let's stick to the implementation I started but fix the headers.
        
        // ... (Continuing the replace block)
         if !tag_ids.is_empty() && match_all {
             query_builder.push(" GROUP BY i.id HAVING COUNT(DISTINCT it.tag_id) = ");
             query_builder.push_bind(tag_ids.len() as i32);
         }
         
         // If we added GROUP BY, `SELECT COUNT(...)` returns a row for each group. 
         // We need to count the rows.
         // Effectively `query.fetch_all().await?.len()` is the Count.
         // Since we are selecting `COUNT(DISTINCT i.id)`, if we Group By `i.id`, we get "1" for every row.
         // So `fetch_all` gives a list of "1"s. The length is the count.
         // If we don't group by, `fetch_one` gives the count.
         
         // So:
         if !tag_ids.is_empty() && match_all {
            // We are grouping. We must select something simple.
             // modify the SELECT at the top? No, it's already pushed.
             
             // This method signature is getting trapped by the specific SQL requirement.
             // Let's rely on fetching the Scalar.
             
             // Wait, if I change the top SELECT to `SELECT i.id`, then:
             // - If match_all (GROUP BY): returns N rows of IDs. Count = N.
             // - If !match_all: returns N rows of IDs (distinct). Count = N.
             // So actually, just selecting `i.id` and returning `rows.len()` is a valid "Count" strategy that works for both, and is relatively cheap (only transferring I64s).
             // It is O(N) memory but N is "filtered set", which is usually fine, but for 100k library it's 800KB. acceptable.
             
             // Let's DO THAT. It simplifies the logic to EXACTLY match `get_images_filtered` structure without the "paging" and "sorting".
         }
         
         // ...
         
        let query = query_builder.build_query_as::<(i64,)>(); // Just fetch IDs
        let rows = query.fetch_all(&self.pool).await?;
        Ok(rows.len() as i64)
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
