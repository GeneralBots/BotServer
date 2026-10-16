use botcore::shared::DbPool;
use chrono::{DateTime, Utc};
use diesel::dsl::{max, sql};
use diesel::prelude::*;

pub use botschema::drive_files;

pub mod dsl {
    pub use super::drive_files::*;
}

pub use botcore::shared::schema::drive::DriveFile;

pub struct FileUpsertParams {
    pub file_path: String,
    pub file_type: String,
    pub etag: Option<String>,
    pub last_modified: Option<DateTime<Utc>>,
    pub indexed: bool,
    pub fail_count: i32,
    pub last_failed_at: Option<DateTime<Utc>>,
}

pub struct DriveFileRepository {
    pool: DbPool,
}

impl std::fmt::Debug for DriveFileRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DriveFileRepository").finish()
    }
}

impl DriveFileRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn get_file_state(&self, file_path: &str) -> Option<DriveFile> {
        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(_) => return None,
        };

        drive_files::table
            .filter(drive_files::file_path.eq(file_path))
            .first(&mut conn)
            .ok()
    }

    pub fn upsert_file(
        &self,
        file_path: &str,
        file_type: &str,
        etag: Option<String>,
        last_modified: Option<DateTime<Utc>>,
        branch_id: Option<uuid::Uuid>,
    ) -> Result<(), String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        let branch_uuid = branch_id.unwrap_or_else(uuid::Uuid::nil);
        let name = file_path.rsplit('/').next().unwrap_or(file_path).to_string();

        // First try UPDATE (unique key is branch_id + path)
        let updated = diesel::sql_query(
            "UPDATE drive_files SET name = $3, mime_type = $4, etag = $5, \
             last_modified = $6, updated_at = NOW() \
             WHERE branch_id = $1 AND path = $2"
        )
        .bind::<diesel::sql_types::Uuid, _>(&branch_uuid)
        .bind::<diesel::sql_types::Text, _>(file_path)
        .bind::<diesel::sql_types::Text, _>(&name)
        .bind::<diesel::sql_types::Text, _>(file_type)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&etag)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(last_modified)
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;

        if updated == 0 {
            diesel::sql_query(
                "INSERT INTO drive_files (branch_id, name, path, file_path, mime_type, file_type, etag, last_modified, indexed, fail_count, created_at, updated_at) \
                 VALUES ($1, $2, $3, $3, $4, $4, $5, $6, false, 0, NOW(), NOW())"
            )
            .bind::<diesel::sql_types::Uuid, _>(&branch_uuid)
            .bind::<diesel::sql_types::Text, _>(&name)
            .bind::<diesel::sql_types::Text, _>(file_path)
            .bind::<diesel::sql_types::Text, _>(file_type)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&etag)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(last_modified)
            .execute(&mut conn)
            .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub fn mark_indexed(&self, file_path: &str, etag: Option<String>) -> Result<(), String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        diesel::update(drive_files::table)
            .filter(drive_files::file_path.eq(file_path))
            .set((
                drive_files::indexed.eq(true),
                drive_files::etag.eq(etag),
                drive_files::fail_count.eq(0),
                drive_files::last_failed_at.eq(None::<DateTime<Utc>>),
                drive_files::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn mark_failed(&self, file_path: &str) -> Result<(), String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        diesel::update(drive_files::table)
            .filter(drive_files::file_path.eq(file_path))
            .set((
                drive_files::fail_count.eq(sql("fail_count + 1")),
                drive_files::last_failed_at.eq(Some(Utc::now())),
                drive_files::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn get_max_fail_count(&self) -> i32 {
        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(_) => return 0,
        };

        drive_files::table
            .select(max(drive_files::fail_count))
            .first::<Option<i32>>(&mut conn)
            .unwrap_or(Some(0))
            .unwrap_or(0)
    }

    pub fn get_files_to_index(&self) -> Vec<DriveFile> {
        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(_) => return vec![],
        };

        drive_files::table
            .filter(drive_files::indexed.eq(false))
            .load(&mut conn)
            .unwrap_or_default()
    }

    pub fn delete_file(&self, file_path: &str) -> Result<(), String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        diesel::delete(drive_files::table)
            .filter(drive_files::file_path.eq(file_path))
            .execute(&mut conn)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn get_all_files_for_bot(&self) -> Vec<DriveFile> {
        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(_) => return vec![],
        };

        drive_files::table
            .load(&mut conn)
            .unwrap_or_default()
    }

    pub fn get_files_by_type(&self, file_type: &str) -> Vec<DriveFile> {
        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(_) => return vec![],
        };

        drive_files::table
            .filter(drive_files::file_type.eq(file_type))
            .load(&mut conn)
            .unwrap_or_default()
    }

    /// Check if a file exists for the given bot and path
    pub fn has_file(&self, file_path: &str) -> bool {
        self.get_file_state(file_path).is_some()
    }

    pub fn upsert_file_full(
        &self,
        params: FileUpsertParams,
        branch_id: Option<uuid::Uuid>,
    ) -> Result<(), String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        let branch_uuid = branch_id.unwrap_or_else(uuid::Uuid::nil);
        let name = params.file_path.rsplit('/').next().unwrap_or(&params.file_path).to_string();

        let updated = diesel::sql_query(
            "UPDATE drive_files SET name = $3, mime_type = $4, etag = $5, last_modified = $6, \
             indexed = $7, fail_count = $8, last_failed_at = $9, updated_at = NOW() \
             WHERE branch_id = $1 AND path = $2"
        )
        .bind::<diesel::sql_types::Uuid, _>(&branch_uuid)
        .bind::<diesel::sql_types::Text, _>(&params.file_path)
        .bind::<diesel::sql_types::Text, _>(&name)
        .bind::<diesel::sql_types::Text, _>(&params.file_type)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&params.etag)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(params.last_modified)
        .bind::<diesel::sql_types::Bool, _>(params.indexed)
        .bind::<diesel::sql_types::Int4, _>(params.fail_count)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(params.last_failed_at)
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;

        if updated == 0 {
            diesel::sql_query(
                "INSERT INTO drive_files (branch_id, name, path, file_path, mime_type, file_type, etag, last_modified, indexed, fail_count, last_failed_at, created_at, updated_at) \
                 VALUES ($1, $2, $3, $3, $4, $4, $5, $6, $7, $8, $9, NOW(), NOW())"
            )
            .bind::<diesel::sql_types::Uuid, _>(&branch_uuid)
            .bind::<diesel::sql_types::Text, _>(&name)
            .bind::<diesel::sql_types::Text, _>(&params.file_path)
            .bind::<diesel::sql_types::Text, _>(&params.file_type)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&params.etag)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(params.last_modified)
            .bind::<diesel::sql_types::Bool, _>(params.indexed)
            .bind::<diesel::sql_types::Int4, _>(params.fail_count)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(params.last_failed_at)
            .execute(&mut conn)
            .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    /// Mark all files matching a path pattern as indexed (for KB folder indexing)
    /// Does NOT update ETag — individual file ETags are already set by upsert_file during scan
    pub fn mark_indexed_by_pattern(&self, pattern: &str) -> Result<(), String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        diesel::update(drive_files::table)
            .filter(drive_files::file_path.like(format!("%{pattern}%")))
            .set((
                drive_files::indexed.eq(true),
                drive_files::fail_count.eq(0),
                drive_files::last_failed_at.eq(None::<DateTime<Utc>>),
                drive_files::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Mark all files matching a path pattern as failed (increment fail_count)
    pub fn mark_failed_by_pattern(&self, pattern: &str) -> Result<(), String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        diesel::update(drive_files::table)
            .filter(drive_files::file_path.like(format!("%{pattern}%")))
            .set((
                drive_files::fail_count.eq(sql("fail_count + 1")),
                drive_files::last_failed_at.eq(Some(Utc::now())),
                drive_files::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Get all files for a bot whose path starts with the given prefix
    pub fn get_files_by_prefix(&self, prefix: &str) -> Vec<DriveFile> {
        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(_) => return vec![],
        };

        drive_files::table
            .filter(drive_files::file_path.like(format!("{prefix}%")))
            .load(&mut conn)
            .unwrap_or_default()
    }

    /// Delete all files for a bot whose path starts with the given prefix
    pub fn delete_by_prefix(&self, prefix: &str) -> Result<usize, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        diesel::delete(drive_files::table)
            .filter(drive_files::file_path.like(format!("{prefix}%")))
            .execute(&mut conn)
            .map_err(|e| e.to_string())
    }

    /// Check if any files exist with the given prefix
    pub fn has_files_with_prefix(&self, prefix: &str) -> bool {
        !self.get_files_by_prefix(prefix).is_empty()
    }

    /// Reset fail_count to 0 for files that failed >= max_fail_count times
    /// and whose last_failed_at is older than the given cooldown duration.
    /// Returns the number of files reset.
    pub fn reset_failed_files(
        &self,
        max_fail_count: i32,
        cooldown: chrono::Duration,
    ) -> Result<usize, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        let cutoff = Utc::now() - cooldown;

        diesel::update(drive_files::table)
            .filter(
                drive_files::fail_count.ge(max_fail_count)
                    .and(drive_files::last_failed_at.is_not_null())
                    .and(drive_files::last_failed_at.lt(cutoff)),
            )
            .set((
                drive_files::fail_count.eq(0),
                drive_files::indexed.eq(false),
                drive_files::last_failed_at.eq(None::<DateTime<Utc>>),
                drive_files::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .map_err(|e| e.to_string())
    }

    /// Reset fail_count to 0 for a specific file path.
    pub fn reset_file_fail_count(&self, file_path: &str) -> Result<(), String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        diesel::update(drive_files::table)
            .filter(drive_files::file_path.eq(file_path))
            .set((
                drive_files::fail_count.eq(0),
                drive_files::indexed.eq(false),
                drive_files::last_failed_at.eq(None::<DateTime<Utc>>),
                drive_files::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
