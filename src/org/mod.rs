use actix_web::{put, web, HttpResponse, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
    pub success: bool,
}

// Helper functions

/// Create a new organization in database
pub async fn create_organization_db(
    db_pool: &PgPool,
    name: &str,
    slug: &str,
) -> Result<Organization, sqlx::Error> {
    let org = sqlx::query_as!(
        Organization,
        r#"
        INSERT INTO organizations (org_id, name, slug, created_at)
        VALUES ($1, $2, $3, $4)
        RETURNING org_id, name, slug, created_at
        "#,
        Uuid::new_v4(),
        name,
        slug,
        Utc::now()
    )
    .fetch_one(db_pool)
    .await?;

    Ok(org)
}

/// Get organization by ID from database
pub async fn get_organization_by_id_db(
    db_pool: &PgPool,
    org_id: Uuid,
) -> Result<Option<Organization>, sqlx::Error> {
    let org = sqlx::query_as!(
        Organization,
        r#"
        SELECT org_id, name, slug, created_at
        FROM organizations
        WHERE org_id = $1
        "#,
        org_id
    )
    .fetch_optional(db_pool)
    .await?;

    Ok(org)
}

#[post("/organizations/create")]
pub async fn create_organization(
    state: web::Data<AppState>,
    payload: web::Json<CreateOrganizationRequest>,
) -> Result<HttpResponse> {
    let org = create_organization_db(&state.db_pool, &payload.name, &payload.slug)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Failed to create organization: {}",
                e
            ))
        })?;

    let response = ApiResponse {
        data: org,
        success: true,
    };

    Ok(HttpResponse::Ok().json(response))
}

#[get("/organizations/{org_id}")]
pub async fn get_organization(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let org_id = path.into_inner();

    let org = get_organization_by_id_db(&state.db_pool, org_id)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Database error: {}", e))
        })?;

    match org {
        Some(org) => {
            let response = ApiResponse {
                data: org,
                success: true,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        None => Ok(HttpResponse::NotFound().json(ApiResponse {
            data: "Organization not found",
            success: false,
        })),
    }
}

#[get("/organizations")]
pub async fn list_organizations(
    state: web::Data<AppState>,
    query: web::Query<PaginationQuery>,
) -> Result<HttpResponse> {
    let orgs = get_organizations_db(&state.db_pool, query.page, query.page_size)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Database error: {}", e))
        })?;

    let response = ApiResponse {
        data: orgs,
        success: true,
    };

    Ok(HttpResponse::Ok().json(response))
}

#[put("/organizations/{org_id}")]
pub async fn update_organization(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    payload: web::Json<CreateOrganizationRequest>,
) -> Result<HttpResponse> {
    let org_id = path.into_inner();

    // Implementation for update operation
    // Use spawn_blocking for CPU-intensive operations if needed
    let updated_org = web::block(move || {
        // Blocking database operation would go here
        // For async, use direct SQLx calls
        Ok::<_, actix_web::Error>(Organization {
            org_id,
            name: payload.name.clone(),
            slug: payload.slug.clone(),
            created_at: Utc::now(),
        })
    })
    .await?
    .map_err(|e: actix_web::Error| e)?;

    let response = ApiResponse {
        data: updated_org,
        success: true,
    };

    Ok(HttpResponse::Ok().json(response))
}
