use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::Response,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;

use crate::schema::email_tracking;
use crate::state::AppState;

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let chars: Vec<u8> = input
        .chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=' {
                Some(c as u8)
            } else {
                None
            }
        })
        .collect();

    const DECODE_TABLE: [i8; 128] = [
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, 62, -1, -1, -1, 63, 52, 53,
        54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, -1, -1, 0,
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
        17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1, -1, -1, -1, 26,
        27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42,
        43, 44, 45, 46, 47, 48, 49, 50, 51, -1, -1, -1, -1, -1, -1, -1,
    ];

    let mut output = Vec::with_capacity(chars.len() * 3 / 4);
    let mut buf = [0u8; 4];
    let mut count = 0;

    for &byte in chars.iter() {
        if byte >= 128 {
            return None;
        }
        let val = DECODE_TABLE[byte as usize];
        if val < 0 {
            continue;
        }
        buf[count] = val as u8;
        count += 1;
        if count == 4 {
            output.push((buf[0] << 2) | (buf[1] >> 4));
            output.push((buf[1] << 4) | (buf[2] >> 2));
            output.push((buf[2] << 6) | buf[3]);
            count = 0;
        }
    }

    if count >= 2 {
        output.push((buf[0] << 2) | (buf[1] >> 4));
        if count > 2 {
            output.push((buf[1] << 4) | (buf[2] >> 2));
        }
    }

    Some(output)
}

pub async fn track_email_open_pixel(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Response {
    let pixel = base64_decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==")
        .unwrap_or_else(|| vec![0u8; 1]);

    if let Ok(mut conn) = state.conn.get() {
        if let Ok(token_uuid) = uuid::Uuid::parse_str(&token) {
            let now = Utc::now();
            let _ = diesel::update(
                email_tracking::table.filter(email_tracking::open_token.eq(Some(token_uuid))),
            )
            .set((
                email_tracking::opened.eq(true),
                email_tracking::opened_at.eq(Some(now)),
            ))
            .execute(&mut conn);

            log::info!("Email open tracked via pixel: token={}", token);
        }
    }

    let mut response = Response::new(Body::from(pixel));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("image/png"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
    response
}

pub async fn track_email_click(
    State(state): State<Arc<AppState>>,
    Path((id, destination)): Path<(String, String)>,
) -> Response {
    if let Ok(mut conn) = state.conn.get() {
        if let Ok(tracking_id) = uuid::Uuid::parse_str(&id) {
            let now = Utc::now();
            let _ = diesel::update(
                email_tracking::table.filter(email_tracking::id.eq(tracking_id)),
            )
            .set((
                email_tracking::clicked.eq(true),
                email_tracking::clicked_at.eq(Some(now)),
            ))
            .execute(&mut conn);

            log::info!("Email click tracked: tracking_id={}", tracking_id);
        }
    }

    let destination = if destination.starts_with("http") {
        destination
    } else {
        format!("/{}", destination)
    };

    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::FOUND;
    response.headers_mut().insert(
        header::LOCATION,
        destination.parse::<header::HeaderValue>().unwrap_or_else(|_| {
            header::HeaderValue::from_static("/")
        }),
    );
    response
}

pub fn configure_marketing_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/crm/campaigns", get(crate::campaigns::list_campaigns).post(crate::campaigns::create_campaign))
        .route("/api/crm/campaigns/:id", get(crate::campaigns::get_campaign).put(crate::campaigns::update_campaign).delete(crate::campaigns::delete_campaign))
        .route("/api/crm/campaigns/:id/send", post(crate::campaigns::send_campaign))
        .route("/api/crm/campaigns/:id/publish", post(crate::publish::publish_campaign))
        .route("/api/crm/campaigns/:id/control/:action", post(crate::campaign::control::control_campaign))
        .route("/api/crm/campaigns/worker/:action", post(crate::campaign::control::control_worker))
        .route("/api/crm/campaigns/:id/events", get(crate::campaign::control::campaign_events))

        .route("/api/crm/lists", get(crate::lists::list_lists).post(crate::lists::create_list))
        .route("/api/crm/lists/:id", get(crate::lists::get_list).put(crate::lists::update_list).delete(crate::lists::delete_list))
        .route("/api/crm/lists/:id/refresh", post(crate::lists::refresh_marketing_list))

        .route("/api/crm/templates", get(crate::templates::list_templates).post(crate::templates::create_template))
        .route("/api/crm/templates/:id", get(crate::templates::get_template).put(crate::templates::update_template).delete(crate::templates::delete_template))

        .route("/api/crm/email/track/open", post(crate::triggers::track_email_open))
        .route("/api/marketing/track/open/:token", get(track_email_open_pixel))
        .route("/api/marketing/track/click/:id/*destination", get(track_email_click))
        .route("/api/crm/email/send", post(crate::email::send_email_api))

        .route("/api/crm/whatsapp/send", post(crate::whatsapp::send_whatsapp_api))

        .route("/api/crm/metrics/campaign/:id", get(crate::metrics_api::get_campaign_metrics_api))
        .route("/api/crm/metrics/campaign/:id/channels", get(crate::metrics_api::get_campaign_channel_breakdown_api))
        .route("/api/crm/metrics/campaign/:id/timeseries/:interval", get(crate::metrics_api::get_campaign_timeseries_api))
        .route("/api/crm/metrics/aggregate", get(crate::metrics_api::get_aggregate_metrics_api))

        .route("/api/crm/ai/generate", post(crate::ai::generate_content_api))
        .route("/api/crm/ai/image", post(crate::ai::generate_studio_image))
        .route("/api/crm/ai/personalize", post(crate::ai::personalize_api))
}
