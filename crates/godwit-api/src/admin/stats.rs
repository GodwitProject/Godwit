use axum::{
    extract::{Extension, Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use godwit_auth::{jwt::Claims, rbac::Role};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/admin/stats", get(get_stats))
        .route("/admin/recent-activity", get(get_recent_activity))
}

/// Only super_admin and org_admin reach the dashboard (see the (dashboard) layout's own
/// role check on the frontend); this mirrors that gate at the API level.
fn require_dashboard_access(claims: &Claims) -> Result<Role, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if role != Role::SuperAdmin && role != Role::OrgAdmin {
        return Err(ApiError::Forbidden);
    }
    Ok(role)
}

#[derive(Deserialize)]
pub struct StatsQuery {
    organization_id: Option<Uuid>,
}

async fn get_stats(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<StatsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_dashboard_access(&claims)?;
    let org_scope = if role == Role::SuperAdmin {
        query.organization_id
    } else {
        Some(claims.organization_id)
    };

    let (organizations, teams, users, api_keys) = match org_scope {
        None => {
            let orgs = state.org_repo.list().await.map_err(ApiError::Core)?;
            let teams = state.team_repo.list_all().await.map_err(ApiError::Core)?;
            let users = state.user_repo.list_all().await.map_err(ApiError::Core)?;
            let keys = state.api_key_repo.list_all().await.map_err(ApiError::Core)?;
            (orgs.len(), teams.len(), users.len(), keys.len())
        }
        Some(org_id) => {
            let teams = state
                .team_repo
                .list_for_organization(org_id)
                .await
                .map_err(ApiError::Core)?;
            let users = state
                .user_repo
                .list_for_organization(org_id)
                .await
                .map_err(ApiError::Core)?;
            let keys = state
                .api_key_repo
                .list_for_organization(org_id)
                .await
                .map_err(ApiError::Core)?;
            (1, teams.len(), users.len(), keys.len())
        }
    };

    Ok(Json(serde_json::json!({
        "organizations": organizations,
        "teams": teams,
        "users": users,
        "apiKeys": api_keys,
    })))
}

#[derive(Deserialize)]
pub struct RecentActivityQuery {
    limit: Option<i64>,
    organization_id: Option<Uuid>,
}

#[derive(serde::Serialize)]
struct ActivityItem {
    id: Uuid,
    #[serde(rename = "type")]
    kind: &'static str,
    name: String,
    created_at: DateTime<Utc>,
}

async fn get_recent_activity(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<RecentActivityQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_dashboard_access(&claims)?;
    let org_scope = if role == Role::SuperAdmin {
        query.organization_id
    } else {
        Some(claims.organization_id)
    };
    let limit = query.limit.unwrap_or(5).clamp(1, 50) as usize;

    let mut items: Vec<ActivityItem> = Vec::new();

    match org_scope {
        None => {
            for org in state.org_repo.list().await.map_err(ApiError::Core)? {
                items.push(ActivityItem {
                    id: org.id,
                    kind: "organization",
                    name: org.name,
                    created_at: org.created_at,
                });
            }
            for team in state.team_repo.list_all().await.map_err(ApiError::Core)? {
                items.push(ActivityItem {
                    id: team.id,
                    kind: "team",
                    name: team.name,
                    created_at: team.created_at,
                });
            }
            for user in state.user_repo.list_all().await.map_err(ApiError::Core)? {
                items.push(ActivityItem {
                    id: user.id,
                    kind: "user",
                    name: user.email,
                    created_at: user.created_at,
                });
            }
            for key in state.api_key_repo.list_all().await.map_err(ApiError::Core)? {
                items.push(ActivityItem {
                    id: key.id,
                    kind: "api_key",
                    name: key.name,
                    created_at: key.created_at,
                });
            }
        }
        Some(org_id) => {
            if let Ok(org) = state.org_repo.get_by_id(org_id).await {
                items.push(ActivityItem {
                    id: org.id,
                    kind: "organization",
                    name: org.name,
                    created_at: org.created_at,
                });
            }
            for team in state
                .team_repo
                .list_for_organization(org_id)
                .await
                .map_err(ApiError::Core)?
            {
                items.push(ActivityItem {
                    id: team.id,
                    kind: "team",
                    name: team.name,
                    created_at: team.created_at,
                });
            }
            for user in state
                .user_repo
                .list_for_organization(org_id)
                .await
                .map_err(ApiError::Core)?
            {
                items.push(ActivityItem {
                    id: user.id,
                    kind: "user",
                    name: user.email,
                    created_at: user.created_at,
                });
            }
            for key in state
                .api_key_repo
                .list_for_organization(org_id)
                .await
                .map_err(ApiError::Core)?
            {
                items.push(ActivityItem {
                    id: key.id,
                    kind: "api_key",
                    name: key.name,
                    created_at: key.created_at,
                });
            }
        }
    }

    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    items.truncate(limit);

    Ok(Json(serde_json::json!({ "data": items })))
}
