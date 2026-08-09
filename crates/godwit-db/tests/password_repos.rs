use godwit_db::repositories::password_history::PasswordHistoryRepository;
use godwit_db::repositories::password_reset_tokens::PasswordResetTokenRepository;
use godwit_db::repositories::users::UserRepository;
use sqlx::PgPool;
use uuid::Uuid;
use std::time::Duration;

async fn seed_user(pool: &PgPool) -> Uuid {
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, 'o')")
        .bind(org_id).execute(pool).await.unwrap();
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, role, organization_id, password_hash) VALUES ($1, 'a@b.c', 'user', $2, 'x')")
        .bind(user_id).bind(org_id).execute(pool).await.unwrap();
    user_id
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn password_history_push_get_purge(pool: PgPool) {
    let uid = seed_user(&pool).await;
    let repo = PasswordHistoryRepository::new(pool.clone());
    repo.push(uid, "hash1").await.unwrap();
    repo.push(uid, "hash2").await.unwrap();
    repo.push(uid, "hash3").await.unwrap();
    let last2 = repo.get_last_n(uid, 2).await.unwrap();
    assert_eq!(last2, vec!["hash3".to_string(), "hash2".to_string()]);
    repo.purge_older_than(uid, 2).await.unwrap();
    let all = repo.get_last_n(uid, 100).await.unwrap();
    assert_eq!(all.len(), 2);
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn password_reset_token_lifecycle(pool: PgPool) {
    let uid = seed_user(&pool).await;
    let repo = PasswordResetTokenRepository::new(pool.clone());
    let t = repo.create(uid, "th", Duration::from_secs(1800)).await.unwrap();
    let got = repo.get_by_hash("th").await.unwrap();
    assert_eq!(got.id, t.id);
    assert!(got.used_at.is_none());
    repo.mark_used(got.id).await.unwrap();
    let after = repo.get_by_hash("th").await.unwrap();
    assert!(after.used_at.is_some());
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn user_update_password_flags(pool: PgPool) {
    let uid = seed_user(&pool).await;
    let urepo = UserRepository::new(pool.clone());
    let u = urepo.update_password(uid, "newhash", None).await.unwrap();
    assert_eq!(u.password_hash.as_deref(), Some("newhash"));
    assert!(!u.must_change_password);
    assert!(u.password_changed_at.is_some());
    let u2 = urepo.set_must_change(uid, true).await.unwrap();
    assert!(u2.must_change_password);
}
