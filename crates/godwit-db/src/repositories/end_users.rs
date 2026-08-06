use crate::models::EndUser;
use godwit_core::PasteurError;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

pub struct EndUsersRepository {
    pool: PgPool,
}

impl EndUsersRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        budget_usd: Option<Decimal>,
        max_budget_usd: Option<Decimal>,
    ) -> Result<EndUser, PasteurError> {
        sqlx::query_as::<_, EndUser>(
            "INSERT INTO end_users (organization_id, user_id, budget_usd, max_budget_usd) 
             VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(organization_id)
        .bind(user_id)
        .bind(budget_usd)
        .bind(max_budget_usd)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_user(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<EndUser, PasteurError> {
        sqlx::query_as::<_, EndUser>(
            "SELECT * FROM end_users WHERE organization_id = $1 AND user_id = $2",
        )
        .bind(organization_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PasteurError::NotFound,
            _ => PasteurError::Database(e.to_string()),
        })
    }

    pub async fn list_by_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<EndUser>, PasteurError> {
        sqlx::query_as::<_, EndUser>(
            "SELECT * FROM end_users WHERE organization_id = $1 ORDER BY user_id",
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn update_budgets(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        budget_usd: Option<Decimal>,
        max_budget_usd: Option<Decimal>,
    ) -> Result<EndUser, PasteurError> {
        sqlx::query_as::<_, EndUser>(
            "UPDATE end_users SET budget_usd = $3, max_budget_usd = $4, updated_at = NOW() 
             WHERE organization_id = $1 AND user_id = $2 RETURNING *",
        )
        .bind(organization_id)
        .bind(user_id)
        .bind(budget_usd)
        .bind(max_budget_usd)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PasteurError::NotFound,
            _ => PasteurError::Database(e.to_string()),
        })
    }

    pub async fn delete(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM end_users WHERE organization_id = $1 AND user_id = $2")
            .bind(organization_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::UserRole;
    use crate::repositories::organizations::OrganizationRepository;
    use crate::repositories::users::UserRepository;
    use sqlx::PgPool;
    use std::str::FromStr;

    #[sqlx::test]
    async fn create_and_get_end_user(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("acme", None).await.expect("create org");
        
        let users = UserRepository::new(pool.clone());
        let user = users.create("user@example.com", None, UserRole::User, Some(org.id))
            .await.expect("create user");

        let repo = EndUsersRepository::new(pool);
        let end_user = repo.create(org.id, user.id, None, None).await.expect("create end_user");
        assert_eq!(end_user.organization_id, org.id);
        assert_eq!(end_user.user_id, user.id);

        let fetched = repo.get_by_user(org.id, user.id).await.expect("get by user");
        assert_eq!(fetched.id, end_user.id);
    }

    #[sqlx::test]
    async fn list_by_organization(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("acme", None).await.expect("create org");
        
        let users = UserRepository::new(pool.clone());
        let user1 = users.create("user1@example.com", None, UserRole::User, Some(org.id))
            .await.expect("create user1");
        let user2 = users.create("user2@example.com", None, UserRole::User, Some(org.id))
            .await.expect("create user2");

        let repo = EndUsersRepository::new(pool);
        repo.create(org.id, user1.id, None, None).await.expect("create end_user1");
        repo.create(org.id, user2.id, None, None).await.expect("create end_user2");

        let listed = repo.list_by_organization(org.id).await.expect("list");
        assert_eq!(listed.len(), 2);
    }

    #[sqlx::test]
    async fn update_budgets(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("acme", None).await.expect("create org");
        
        let users = UserRepository::new(pool.clone());
        let user = users.create("user@example.com", None, UserRole::User, Some(org.id))
            .await.expect("create user");

        let repo = EndUsersRepository::new(pool);
        let end_user = repo.create(org.id, user.id, None, None).await.expect("create end_user");

        let budget = rust_decimal::Decimal::from_str("100.00").unwrap();
        let max_budget = rust_decimal::Decimal::from_str("200.00").unwrap();
        let updated = repo.update_budgets(org.id, user.id, Some(budget), Some(max_budget))
            .await.expect("update budgets");
        
        assert_eq!(updated.budget_usd, Some(budget));
        assert_eq!(updated.max_budget_usd, Some(max_budget));
    }
}
