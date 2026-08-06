use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ModelAlias {
    pub id: Uuid,
    pub alias: String,
    pub target_model_id: Uuid,
    pub created_at: DateTime<Utc>,
}
