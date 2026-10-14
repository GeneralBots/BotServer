use crate::models::*;
use crate::requests::*;
use crate::schema::*;

use chrono::Utc;
use diesel::prelude::*;

use uuid::Uuid;

pub struct AnalyticsEngine {
    db: crate::routes::DbPool,
}

impl AnalyticsEngine {
    pub fn new(db: crate::routes::DbPool) -> Self {
        Self { db }
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

    pub async fn get_or_create_analytics(
        &self,
        project_id: Uuid,
        export_id: Option<Uuid>,
    ) -> Result<VideoAnalytics, diesel::result::Error> {
        let mut conn = self.get_conn()?;

        let existing: Result<VideoAnalytics, _> = video_analytics::table
            .filter(video_analytics::project_id.eq(project_id))
            .filter(video_analytics::export_id.eq(export_id))
            .first(&mut conn);

        match existing {
            Ok(analytics) => Ok(analytics),
            Err(diesel::result::Error::NotFound) => {
                let analytics = VideoAnalytics {
                    id: Uuid::new_v4(),
                    project_id,
                    export_id,
                    views: 0,
                    unique_viewers: 0,
                    total_watch_time_ms: 0,
                    avg_watch_percent: 0.0,
                    completions: 0,
                    shares: 0,
                    likes: 0,
                    engagement_score: 0.0,
                    viewer_retention_json: Some(serde_json::json!([])),
                    geography_json: Some(serde_json::json!({})),
                    device_json: Some(serde_json::json!({
                        "desktop": 0,
                        "mobile": 0,
                        "tablet": 0,
                        "tv": 0
                    })),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };

                diesel::insert_into(video_analytics::table)
                    .values(&analytics)
                    .execute(&mut conn)?;

                Ok(analytics)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn record_view(
        &self,
        req: RecordViewRequest,
    ) -> Result<VideoAnalytics, diesel::result::Error> {
        let mut conn = self.get_conn()?;

        let analytics: VideoAnalytics = video_analytics::table
            .filter(video_analytics::export_id.eq(Some(req.export_id)))
            .first(&mut conn)?;

        let new_views = analytics.views + 1;
        let new_watch_time = analytics.total_watch_time_ms + req.watch_time_ms;
        let new_completions = if req.completed {
            analytics.completions + 1
        } else {
            analytics.completions
        };

        let mut geo_json = analytics
            .geography_json
            .clone()
            .unwrap_or(serde_json::json!({}));
        if let Some(country) = &req.country {
            if let Some(obj) = geo_json.as_object_mut() {
                let count = obj.get(country).and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert(country.clone(), serde_json::json!(count + 1));
            }
        }

        let mut device_json = analytics
            .device_json
            .clone()
            .unwrap_or(serde_json::json!({}));
        if let Some(device) = &req.device {
            if let Some(obj) = device_json.as_object_mut() {
                let count = obj.get(device).and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert(device.clone(), serde_json::json!(count + 1));
            }
        }

        let engagement_score = calculate_engagement_score(
            new_views,
            new_completions,
            analytics.shares,
            analytics.likes,
        );

        diesel::update(video_analytics::table.find(analytics.id))
            .set((
                video_analytics::views.eq(new_views),
                video_analytics::total_watch_time_ms.eq(new_watch_time),
                video_analytics::completions.eq(new_completions),
                video_analytics::engagement_score.eq(engagement_score),
                video_analytics::geography_json.eq(&geo_json),
                video_analytics::device_json.eq(&device_json),
                video_analytics::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)?;

        video_analytics::table.find(analytics.id).first(&mut conn)
    }

    pub async fn get_analytics(
        &self,
        project_id: Uuid,
    ) -> Result<crate::responses::AnalyticsResponse, diesel::result::Error> {
        let mut conn = self.get_conn()?;

        let analytics: VideoAnalytics = video_analytics::table
            .filter(video_analytics::project_id.eq(project_id))
            .first(&mut conn)?;

        let viewer_retention = parse_retention(&analytics.viewer_retention_json);
        let top_countries = parse_geography(&analytics.geography_json);
        let devices = parse_devices(&analytics.device_json);

        Ok(crate::responses::AnalyticsResponse {
            views: analytics.views,
            unique_viewers: analytics.unique_viewers,
            total_watch_time_ms: analytics.total_watch_time_ms,
            avg_watch_percent: analytics.avg_watch_percent,
            completions: analytics.completions,
            shares: analytics.shares,
            likes: analytics.likes,
            engagement_score: analytics.engagement_score,
            viewer_retention,
            top_countries,
            devices,
        })
    }

    pub async fn increment_shares(&self, project_id: Uuid) -> Result<(), diesel::result::Error> {
        let mut conn = self.get_conn()?;

        diesel::update(
            video_analytics::table.filter(video_analytics::project_id.eq(project_id)),
        )
        .set((
            video_analytics::shares.eq(video_analytics::shares + 1),
            video_analytics::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)?;

        Ok(())
    }

    pub async fn increment_likes(&self, project_id: Uuid) -> Result<(), diesel::result::Error> {
        let mut conn = self.get_conn()?;

        diesel::update(
            video_analytics::table.filter(video_analytics::project_id.eq(project_id)),
        )
        .set((
            video_analytics::likes.eq(video_analytics::likes + 1),
            video_analytics::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)?;

        Ok(())
    }
}

fn calculate_engagement_score(views: i64, completions: i64, shares: i64, likes: i64) -> f32 {
    if views == 0 {
        return 0.0;
    }

    let completion_rate = completions as f32 / views as f32;
    let share_rate = shares as f32 / views as f32;
    let like_rate = likes as f32 / views as f32;

    (completion_rate * 0.5 + share_rate * 0.3 + like_rate * 0.2) * 100.0
}

fn parse_retention(json: &Option<serde_json::Value>) -> Vec<crate::responses::RetentionPoint> {
    json.as_ref()
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    Some(crate::responses::RetentionPoint {
                        percent: item.get("percent")?.as_f64()? as f32,
                        viewers: item.get("viewers")?.as_i64()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_geography(json: &Option<serde_json::Value>) -> Vec<crate::responses::GeoData> {
    let obj = match json.as_ref().and_then(|v| v.as_object()) {
        Some(o) => o,
        None => return vec![],
    };

    let total: i64 = obj.values().filter_map(|v| v.as_i64()).sum();
    if total == 0 {
        return vec![];
    }

    let mut data: Vec<crate::responses::GeoData> = obj
        .iter()
        .filter_map(|(country, views)| {
            let v = views.as_i64()?;
            Some(crate::responses::GeoData {
                country: country.clone(),
                views: v,
                percent: (v as f32 / total as f32) * 100.0,
            })
        })
        .collect();

    data.sort_by_key(|a| std::cmp::Reverse(a.views.clone()));
    data.truncate(10);
    data
}

fn parse_devices(json: &Option<serde_json::Value>) -> crate::responses::DeviceBreakdown {
    let obj = match json.as_ref().and_then(|v| v.as_object()) {
        Some(o) => o,
        None => {
            return crate::responses::DeviceBreakdown {
                desktop: 0.0,
                mobile: 0.0,
                tablet: 0.0,
                tv: 0.0,
            }
        }
    };

    let desktop = obj.get("desktop").and_then(|v| v.as_i64()).unwrap_or(0) as f32;
    let mobile = obj.get("mobile").and_then(|v| v.as_i64()).unwrap_or(0) as f32;
    let tablet = obj.get("tablet").and_then(|v| v.as_i64()).unwrap_or(0) as f32;
    let tv = obj.get("tv").and_then(|v| v.as_i64()).unwrap_or(0) as f32;

    let total = desktop + mobile + tablet + tv;
    if total == 0.0 {
        return crate::responses::DeviceBreakdown {
            desktop: 0.0,
            mobile: 0.0,
            tablet: 0.0,
            tv: 0.0,
        };
    }

    crate::responses::DeviceBreakdown {
        desktop: (desktop / total) * 100.0,
        mobile: (mobile / total) * 100.0,
        tablet: (tablet / total) * 100.0,
        tv: (tv / total) * 100.0,
    }
}

pub async fn get_analytics_handler(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<crate::routes::AppState>>,
    axum::extract::Path(project_id): axum::extract::Path<Uuid>,
) -> impl axum::response::IntoResponse {
    let engine = AnalyticsEngine::new(state.conn.clone());

    let _ = engine.get_or_create_analytics(project_id, None).await;

    match engine.get_analytics(project_id).await {
        Ok(analytics) => (
            axum::http::StatusCode::OK,
            axum::Json(serde_json::json!(analytics)),
        ),
        Err(e) => {
            log::error!("Failed to get analytics: {e}");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!(SafeErrorResponse::internal_error())),
            )
        }
    }
}

pub async fn record_view_handler(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<crate::routes::AppState>>,
    axum::Json(req): axum::Json<RecordViewRequest>,
) -> impl axum::response::IntoResponse {
    let engine = AnalyticsEngine::new(state.conn.clone());

    match engine.record_view(req).await {
        Ok(_) => (
            axum::http::StatusCode::OK,
            axum::Json(serde_json::json!({ "success": true })),
        ),
        Err(e) => {
            log::error!("Failed to record view: {e}");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!(SafeErrorResponse::internal_error())),
            )
        }
    }
}

use crate::safe_error::SafeErrorResponse;
