//! CalDAV 1.0 surface for the calendar collections (issue #1335).
//!
//! Native calendar clients — Thunderbird, Apple Calendar, DAVx5 on Android —
//! talk to a calendar server in three steps: `PROPFIND` to discover the
//! principal and its calendar collections, `REPORT` to fetch the events of a
//! collection for a time range, and `GET`/`PUT`/`DELETE` on the individual
//! `.ics` resources. The router previously answered only `GET` and `PUT`, so
//! discovery failed on the very first request and no client could sync.
//!
//! Authentication is HTTP Basic because that is the only scheme these clients
//! implement. The password field carries a GeneralBots token: the global auth
//! middleware verifies the signature before any handler runs, and the handlers
//! here repeat the presence check so a client that is not authenticated
//! receives a `WWW-Authenticate` challenge instead of an unexplained failure.
//!
//! The responsibilities are split across three modules: request bodies and
//! response documents in [`crate::caldav_xml`], HTTP responses in
//! [`crate::caldav_http`], queries and calendar-object documents in
//! [`crate::caldav_store`]. This module is the dispatcher.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, Method},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use std::sync::Arc;
use uuid::Uuid;

use super::DbPool;
use crate::caldav_http::{
    build_response, challenge, forbidden, method_not_allowed, multi_status, not_found,
    options_response,
};
use crate::caldav_store::{delete_event, load_calendar, load_calendars, load_events};
use crate::caldav_xml;

pub fn create_caldav_router() -> Router<Arc<DbPool>> {
    Router::new()
        .route("/.well-known/caldav", any(well_known))
        .route("/caldav", any(root_resource))
        .route("/caldav/", any(root_resource))
        .route("/caldav/principals", any(principals_resource))
        .route("/caldav/principals/", any(principals_resource))
        .route("/caldav/calendars", any(calendars_resource))
        .route("/caldav/calendars/", any(calendars_resource))
        .route("/caldav/calendars/:calendar_id", any(calendar_resource))
        .route("/caldav/calendars/:calendar_id/", any(calendar_resource))
        .route(
            "/caldav/calendars/:calendar_id/*resource",
            any(event_resource),
        )
}

/// Clients probe `/.well-known/caldav` before anything else and follow the
/// redirect to the DAV root.
async fn well_known() -> Response {
    build_response(
        axum::http::StatusCode::MOVED_PERMANENTLY,
        &[("Location", "/caldav")],
        Body::empty(),
    )
}

/// The DAV root: the principal and the calendar home collection.
async fn root_resource(method: Method, headers: HeaderMap) -> Response {
    // `http::Method` only has constants for the standard verbs, so the WebDAV
    // verbs are matched by name.
    match method.as_str() {
        "OPTIONS" => options_response(),
        "GET" | "HEAD" => super::caldav_root().await.into_response(),
        "PROPFIND" => {
            if !authenticated(&headers) {
                return challenge();
            }
            let mut responses = caldav_xml::response(
                "/caldav/",
                "<D:resourcetype><D:collection/></D:resourcetype>\n\
                 <D:displayname>GeneralBots Calendar</D:displayname>\n\
                 <D:current-user-principal><D:href>/caldav/principals/</D:href></D:current-user-principal>",
            );
            if depth_of(&headers) != 0 {
                responses.push_str(&caldav_xml::response(
                    "/caldav/principals/",
                    "<D:resourcetype><D:collection/><D:principal/></D:resourcetype>\n\
                     <D:displayname>Principal</D:displayname>",
                ));
                responses.push_str(&caldav_xml::response(
                    "/caldav/calendars/",
                    "<D:resourcetype><D:collection/></D:resourcetype>\n\
                     <D:displayname>Calendars</D:displayname>",
                ));
            }
            multi_status(caldav_xml::multistatus(&responses))
        }
        _ => method_not_allowed(),
    }
}

/// The principal the caller authenticates as, with its calendar home.
async fn principals_resource(method: Method, headers: HeaderMap) -> Response {
    match method.as_str() {
        "OPTIONS" => options_response(),
        "GET" | "HEAD" => super::caldav_principals().await.into_response(),
        "PROPFIND" => {
            if !authenticated(&headers) {
                return challenge();
            }
            let responses = caldav_xml::response(
                "/caldav/principals/",
                "<D:resourcetype><D:collection/><D:principal/></D:resourcetype>\n\
                 <D:displayname>Principal</D:displayname>\n\
                 <D:current-user-principal><D:href>/caldav/principals/</D:href></D:current-user-principal>\n\
                 <C:calendar-home-set><D:href>/caldav/calendars/</D:href></C:calendar-home-set>",
            );
            multi_status(caldav_xml::multistatus(&responses))
        }
        _ => method_not_allowed(),
    }
}

/// The calendar home collection, listing every calendar of the caller's scope.
async fn calendars_resource(
    State(state): State<Arc<DbPool>>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    match method.as_str() {
        "OPTIONS" => options_response(),
        "GET" | "HEAD" => super::caldav_calendars(State(state)).await.into_response(),
        "PROPFIND" => {
            if !authenticated(&headers) {
                return challenge();
            }
            let calendars = match load_calendars(&state, &headers).await {
                Ok(calendars) => calendars,
                Err(error) => return error.into_response(),
            };
            let mut responses = caldav_xml::response(
                "/caldav/calendars/",
                "<D:resourcetype><D:collection/></D:resourcetype>\n\
                 <D:displayname>Calendars</D:displayname>",
            );
            if depth_of(&headers) != 0 {
                for calendar in &calendars {
                    responses.push_str(&crate::caldav_store::calendar_response(calendar));
                }
            }
            multi_status(caldav_xml::multistatus(&responses))
        }
        _ => method_not_allowed(),
    }
}

/// A single calendar collection. `REPORT` is what a client uses to fetch the
/// events of this calendar for a time range.
async fn calendar_resource(
    State(state): State<Arc<DbPool>>,
    method: Method,
    headers: HeaderMap,
    Path(calendar_id): Path<String>,
    body: String,
) -> Response {
    let Some(calendar_uuid) = Uuid::parse_str(&calendar_id).ok() else {
        return not_found("Unknown calendar");
    };

    match method.as_str() {
        "OPTIONS" => options_response(),
        "GET" | "HEAD" => {
            super::caldav_calendar(State(state), headers, Path(calendar_id))
                .await
                .into_response()
        }
        "PROPFIND" => {
            if !authenticated(&headers) {
                return challenge();
            }
            let calendar = match load_calendar(&state, &headers, calendar_uuid).await {
                Ok(Some(calendar)) => calendar,
                Ok(None) => return not_found("Unknown calendar"),
                Err(error) => return error.into_response(),
            };
            let mut responses = crate::caldav_store::calendar_response(&calendar);
            if depth_of(&headers) != 0 {
                let events =
                    match load_events(&state, &headers, calendar_uuid, None, None, Vec::new()).await {
                        Ok(events) => events,
                        Err(error) => return error.into_response(),
                    };
                for event in &events {
                    responses.push_str(&crate::caldav_store::event_response(
                        calendar_uuid,
                        event,
                        false,
                    ));
                }
            }
            multi_status(caldav_xml::multistatus(&responses))
        }
        "REPORT" => {
            if !authenticated(&headers) {
                return challenge();
            }
            if let Err(error) = load_calendar(&state, &headers, calendar_uuid).await {
                return error.into_response();
            }
            let (time_min, time_max) = caldav_xml::parse_time_range(&body);
            let hrefs = caldav_xml::parse_hrefs(&body);
            let wanted: Vec<Uuid> = hrefs
                .iter()
                .filter_map(|href| caldav_xml::event_id_from_href(href))
                .collect();
            let events =
                match load_events(&state, &headers, calendar_uuid, time_min, time_max, wanted).await
                {
                    Ok(events) => events,
                    Err(error) => return error.into_response(),
                };
            let mut responses = String::new();
            for event in &events {
                responses.push_str(&crate::caldav_store::event_response(
                    calendar_uuid,
                    event,
                    true,
                ));
            }
            // A multiget names the resources it wants, so an href that resolved
            // to nothing has to be reported as missing.
            for href in &hrefs {
                let wanted_id = caldav_xml::event_id_from_href(href);
                if !events.iter().any(|event| Some(event.id) == wanted_id) {
                    responses.push_str(&caldav_xml::missing_response(href));
                }
            }
            multi_status(caldav_xml::multistatus(&responses))
        }
        _ => method_not_allowed(),
    }
}

/// An individual calendar object resource (`…/<event>.ics`).
async fn event_resource(
    State(state): State<Arc<DbPool>>,
    method: Method,
    headers: HeaderMap,
    Path((calendar_id, resource)): Path<(String, String)>,
    body: String,
) -> Response {
    match method.as_str() {
        "OPTIONS" => options_response(),
        "GET" | "HEAD" => {
            super::caldav_event(State(state), headers, Path((calendar_id, resource)))
                .await
                .into_response()
        }
        "PROPFIND" => {
            if !authenticated(&headers) {
                return challenge();
            }
            let Some(event_id) = caldav_xml::event_id_from_href(&resource) else {
                return not_found("Unknown event");
            };
            let Ok(calendar_uuid) = Uuid::parse_str(&calendar_id) else {
                return not_found("Unknown calendar");
            };
            match load_events(&state, &headers, calendar_uuid, None, None, vec![event_id]).await {
                Ok(events) => match events.into_iter().find(|event| event.id == event_id) {
                    Some(event) => multi_status(caldav_xml::multistatus(
                        &crate::caldav_store::event_response(calendar_uuid, &event, false),
                    )),
                    None => not_found("Unknown event"),
                },
                Err(error) => error.into_response(),
            }
        }
        "PUT" => {
            if !authenticated(&headers) {
                return challenge();
            }
            // The href carries the resource name; the stored event id has to be
            // the same one, otherwise a client update would create a duplicate.
            let event_id = caldav_xml::event_id_from_href(&resource)
                .unwrap_or_else(Uuid::new_v4)
                .to_string();
            super::caldav_put_event(State(state), headers, Path((calendar_id, event_id)), body)
                .await
                .into_response()
        }
        "DELETE" => {
            if !authenticated(&headers) {
                return challenge();
            }
            let Some(event_id) = caldav_xml::event_id_from_href(&resource) else {
                return not_found("Unknown event");
            };
            delete_event(&state, &headers, event_id).await
        }
        // Apple Calendar and Thunderbird write display name and colour with
        // PROPPATCH. Those properties are not editable here, and 403 is the
        // answer they accept for an unchangeable property.
        "PROPPATCH" => forbidden("Calendar properties are not editable"),
        _ => method_not_allowed(),
    }
}

/// Whether the request carries a credential the middleware can verify. The
/// verification itself happens before the handler runs; the check here exists
/// so an unauthenticated client is told which scheme to use.
fn authenticated(headers: &HeaderMap) -> bool {
    let Some(authorization) = headers.get("authorization").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    botsecurity_core::tenant::token_from_authorization(authorization).is_some()
}

/// Depth requested by a `PROPFIND`. A missing header means the collection and
/// its children, which is what clients expect to receive.
fn depth_of(headers: &HeaderMap) -> u64 {
    headers
        .get("depth")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(1)
}
