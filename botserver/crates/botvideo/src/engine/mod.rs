mod audio_export;
mod chat_ai;
mod clips_layers;
mod effects;

pub use effects::CaptionRequest;

use crate::schema::*;
use crate::models::*;
use crate::requests::*;
use crate::responses::*;
use crate::routes::DbPool;

use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info};
use uuid::Uuid;

pub struct VideoEngine {
    pub db: DbPool,
    pub cache: Option<Arc<redis::Client>>,
    pub progress_tx: Option<broadcast::Sender<ExportProgressEvent>>,
}

impl VideoEngine {
    pub fn new(db: DbPool) -> Self {
        Self {
            db,
            cache: None,
            progress_tx: None,
        }
    }

    pub fn with_cache(db: DbPool, cache: Arc<redis::Client>) -> Self {
        Self {
            db,
            cache: Some(cache),
            progress_tx: None,
        }
    }

    fn get_conn(
        &self,
    ) -> Result<
        diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::PgConnection>>,
        diesel::result::Error,
    > {
        self.db.get().map_err(|e| {
            log::error!("DB connection error: {e}");
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::Unknown,
                Box::new(e.to_string()),
            )
        })
    }

    pub async fn create_project(
        &self,
        organization_id: Option<Uuid>,
        created_by: Option<Uuid>,
        req: CreateProjectRequest,
    ) -> Result<VideoProject, diesel::result::Error> {
        let mut conn = self.get_conn()?;

        let project = VideoProject {
            id: Uuid::new_v4(),
            organization_id,
            created_by,
            name: req.name,
            description: req.description,
            resolution_width: req.resolution_width.unwrap_or(1920),
            resolution_height: req.resolution_height.unwrap_or(1080),
            fps: req.fps.unwrap_or(30),
            total_duration_ms: 0,
            timeline_json: serde_json::json!({"clips": []}),
            layers_json: serde_json::json!([]),
            audio_tracks_json: serde_json::json!([]),
            playhead_ms: 0,
            selection_json: serde_json::json!({"type": "None"}),
            zoom_level: 1.0,
            thumbnail_url: None,
            status: ProjectStatus::Draft.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        diesel::insert_into(video_projects::table)
            .values(&project)
            .execute(&mut conn)?;

        info!("Created video project: {} ({})", project.name, project.id);
        Ok(project)
    }

    pub async fn get_project(&self, project_id: Uuid) -> Result<VideoProject, diesel::result::Error> {
        let mut conn = self.get_conn()?;
        video_projects::table.find(project_id).first(&mut conn)
    }

    pub async fn list_projects(
        &self,
        org_id: Option<Uuid>,
        filters: ProjectFilters,
    ) -> Result<Vec<ProjectResponse>, diesel::result::Error> {
        let mut conn = self.get_conn()?;
        let mut query = video_projects::table.into_boxed();

        if let Some(org) = org_id {
            query = query.filter(video_projects::organization_id.eq(org));
        }
        if let Some(status) = filters.status {
            query = query.filter(video_projects::status.eq(status));
        }
        if let Some(search) = filters.search {
            let pattern = format!("%{search}%");
            query = query.filter(video_projects::name.ilike(pattern));
        }

        query = query.order(video_projects::updated_at.desc());

        if let Some(limit) = filters.limit {
            query = query.limit(limit);
        }
        if let Some(offset) = filters.offset {
            query = query.offset(offset);
        }

        let projects: Vec<VideoProject> = query.load(&mut conn)?;

        let responses: Vec<ProjectResponse> = projects
            .into_iter()
            .map(|p| ProjectResponse {
                id: p.id,
                name: p.name,
                description: p.description,
                resolution_width: p.resolution_width,
                resolution_height: p.resolution_height,
                fps: p.fps,
                total_duration_ms: p.total_duration_ms,
                playhead_ms: p.playhead_ms,
                zoom_level: p.zoom_level,
                thumbnail_url: p.thumbnail_url,
                status: p.status,
                clips_count: 0,
                layers_count: 0,
                created_at: p.created_at,
                updated_at: p.updated_at,
            })
            .collect();

        Ok(responses)
    }

    pub async fn update_project(
        &self,
        project_id: Uuid,
        req: UpdateProjectRequest,
    ) -> Result<VideoProject, diesel::result::Error> {
        let mut conn = self.get_conn()?;
        let project: VideoProject = video_projects::table.find(project_id).first(&mut conn)?;

        diesel::update(video_projects::table.find(project_id))
            .set((
                video_projects::name.eq(req.name.unwrap_or(project.name)),
                video_projects::description.eq(req.description.or(project.description)),
                video_projects::playhead_ms.eq(req.playhead_ms.unwrap_or(project.playhead_ms)),
                video_projects::selection_json.eq(req.selection_json.unwrap_or(project.selection_json)),
                video_projects::zoom_level.eq(req.zoom_level.unwrap_or(project.zoom_level)),
                video_projects::status.eq(req.status.unwrap_or(project.status)),
                video_projects::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)?;

        self.get_project(project_id).await
    }

    pub async fn delete_project(&self, project_id: Uuid) -> Result<(), diesel::result::Error> {
        let mut conn = self.get_conn()?;

        diesel::delete(video_clips::table.filter(video_clips::project_id.eq(project_id)))
            .execute(&mut conn)?;
        diesel::delete(video_layers::table.filter(video_layers::project_id.eq(project_id)))
            .execute(&mut conn)?;
        diesel::delete(video_audio_tracks::table.filter(video_audio_tracks::project_id.eq(project_id)))
            .execute(&mut conn)?;
        diesel::delete(video_exports::table.filter(video_exports::project_id.eq(project_id)))
            .execute(&mut conn)?;
        diesel::delete(video_command_history::table.filter(video_command_history::project_id.eq(project_id)))
            .execute(&mut conn)?;
        diesel::delete(video_analytics::table.filter(video_analytics::project_id.eq(project_id)))
            .execute(&mut conn)?;
        diesel::delete(video_projects::table.find(project_id)).execute(&mut conn)?;

        info!("Deleted video project: {project_id}");
        Ok(())
    }
}
