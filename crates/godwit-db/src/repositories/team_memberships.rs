use crate::models::TeamMembership;
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct TeamMembershipRepository {
    pool: PgPool,
}

impl TeamMembershipRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn add_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<TeamMembership, PasteurError> {
        sqlx::query_as::<_, TeamMembership>(
            "INSERT INTO team_memberships (user_id, team_id, role) VALUES ($1, $2, $3)
             ON CONFLICT (user_id, team_id) DO UPDATE SET role = EXCLUDED.role
             RETURNING *",
        )
        .bind(user_id)
        .bind(team_id)
        .bind(role)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn remove_member(&self, team_id: Uuid, user_id: Uuid) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM team_memberships WHERE team_id = $1 AND user_id = $2")
            .bind(team_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn get_membership(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<TeamMembership, PasteurError> {
        sqlx::query_as::<_, TeamMembership>(
            "SELECT * FROM team_memberships WHERE team_id = $1 AND user_id = $2",
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PasteurError::NotFound,
            _ => PasteurError::Database(e.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::{
        organizations::OrganizationRepository, teams::TeamRepository, users::UserRepository,
    };
    use crate::models::UserRole;
    use sqlx::PgPool;

    async fn seed(pool: &PgPool) -> (Uuid, Uuid) {
        let org = OrganizationRepository::new(pool.clone())
            .create("acme", None)
            .await
            .expect("create org");
        let team = TeamRepository::new(pool.clone())
            .create(org.id, "engineering")
            .await
            .expect("create team");
        let user = UserRepository::new(pool.clone())
            .create("dave@example.com", None, UserRole::User, Some(org.id))
            .await
            .expect("create user");
        (team.id, user.id)
    }

    #[sqlx::test]
    async fn add_and_get_membership(pool: PgPool) {
        let (team_id, user_id) = seed(&pool).await;
        let repo = TeamMembershipRepository::new(pool);
        let membership = repo
            .add_member(team_id, user_id, "member")
            .await
            .expect("add member");
        assert_eq!(membership.role, "member");

        let fetched = repo
            .get_membership(team_id, user_id)
            .await
            .expect("get membership");
        assert_eq!(fetched.user_id, user_id);
    }

    #[sqlx::test]
    async fn add_member_upserts_role(pool: PgPool) {
        let (team_id, user_id) = seed(&pool).await;
        let repo = TeamMembershipRepository::new(pool);
        repo.add_member(team_id, user_id, "member")
            .await
            .expect("add as member");
        let promoted = repo
            .add_member(team_id, user_id, "team_admin")
            .await
            .expect("re-add as team_admin");
        assert_eq!(promoted.role, "team_admin");
    }

    #[sqlx::test]
    async fn remove_member_deletes_row(pool: PgPool) {
        let (team_id, user_id) = seed(&pool).await;
        let repo = TeamMembershipRepository::new(pool);
        repo.add_member(team_id, user_id, "member")
            .await
            .expect("add member");
        repo.remove_member(team_id, user_id)
            .await
            .expect("remove member");
        let err = repo.get_membership(team_id, user_id).await.unwrap_err();
        assert!(matches!(err, godwit_core::PasteurError::NotFound));
    }

    #[sqlx::test]
    async fn get_membership_not_found(pool: PgPool) {
        let (team_id, user_id) = seed(&pool).await;
        let repo = TeamMembershipRepository::new(pool);
        let err = repo.get_membership(team_id, user_id).await.unwrap_err();
        assert!(matches!(err, godwit_core::PasteurError::NotFound));
    }
}
