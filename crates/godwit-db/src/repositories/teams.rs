use crate::models::Team;
use godwit_core::PasteurError;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

pub struct TeamRepository {
    pool: PgPool,
}

impl TeamRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        organization_id: Uuid,
        name: &str,
        budget_usd: Option<Decimal>,
        max_budget_usd: Option<Decimal>,
    ) -> Result<Team, PasteurError> {
        sqlx::query_as::<_, Team>(
            "INSERT INTO teams (organization_id, name, budget_usd, max_budget_usd) VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(organization_id)
        .bind(name)
        .bind(budget_usd)
        .bind(max_budget_usd)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Team, PasteurError> {
        sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn list_for_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<Team>, PasteurError> {
        sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE organization_id = $1 ORDER BY name")
            .bind(organization_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn list_all(&self) -> Result<Vec<Team>, PasteurError> {
        sqlx::query_as::<_, Team>("SELECT * FROM teams ORDER BY organization_id, name")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn update(&self, id: Uuid, name: &str) -> Result<Team, PasteurError> {
        sqlx::query_as::<_, Team>("UPDATE teams SET name = $2 WHERE id = $1 RETURNING *")
            .bind(id)
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn update_with_budget(
        &self,
        id: Uuid,
        name: &str,
        budget_usd: Option<Decimal>,
        max_budget_usd: Option<Decimal>,
    ) -> Result<Team, PasteurError> {
        sqlx::query_as::<_, Team>(
            "UPDATE teams SET name = $2, budget_usd = $3, max_budget_usd = $4 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(name)
        .bind(budget_usd)
        .bind(max_budget_usd)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PasteurError::NotFound,
            _ => PasteurError::Database(e.to_string()),
        })
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM teams WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::organizations::OrganizationRepository;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn create_list_and_get_team(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("acme", None).await.expect("create org");

        let repo = TeamRepository::new(pool);
        let team = repo.create(org.id, "engineering", None, None).await.expect("create team");
        assert_eq!(team.organization_id, org.id);
        assert_eq!(team.name, "engineering");

        let fetched = repo.get_by_id(team.id).await.expect("get by id");
        assert_eq!(fetched.id, team.id);

        let listed = repo.list_for_organization(org.id).await.expect("list");
        assert_eq!(listed.len(), 1);
    }

    #[sqlx::test]
    async fn list_all_spans_organizations(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org_a = orgs.create("acme-a", None).await.expect("create org a");
        let org_b = orgs.create("acme-b", None).await.expect("create org b");

        let repo = TeamRepository::new(pool);
        repo.create(org_a.id, "team-a", None, None).await.expect("create team a");
        repo.create(org_b.id, "team-b", None, None).await.expect("create team b");

        let all = repo.list_all().await.expect("list all");
        assert_eq!(all.len(), 2);
    }

    #[sqlx::test]
    async fn update_renames_team(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("acme", None).await.expect("create org");
        let repo = TeamRepository::new(pool);
        let team = repo.create(org.id, "old-name", None, None).await.expect("create team");

        let updated = repo.update(team.id, "new-name").await.expect("update team");
        assert_eq!(updated.name, "new-name");
    }
}
