
async fn verified_identity() -> Identity {
    let f = Fixture::new(false, 300).await;
    let (cookie, _) = f.authenticate().await;
    let (mut parts, _) = axum::http::Request::builder()
        .uri("/api/auth/me")
        .header("cookie", cookie)
        .body(())
        .unwrap()
        .into_parts();
    Authenticated::from_request_parts(&mut parts, &f.state)
        .await
        .ok()
        .unwrap()
        .identity
}
async fn identity_contract(
    a: Arc<dyn IdentityRepository>,
    b: Arc<dyn IdentityRepository>,
    admin: Arc<PostgresIdentityRepository>,
) {
    let alice = verified_identity().await;
    assert!(matches!(a.find(&alice).await, Err(IdentityError::NotFound)));
    let (one, two) = tokio::join!(a.register_login(&alice), b.register_login(&alice));
    let user = one.unwrap();
    assert_eq!(user.id(), two.unwrap().id());
    assert!(user.permissions().is_empty());
    // Same subject, independently verified issuer.
    let other = b.register_login(&verified_identity().await).await.unwrap();
    assert_ne!(user.id(), other.id());
    admin
        .grant(user.id(), Permission::ReadBudget)
        .await
        .unwrap();
    admin
        .grant(user.id(), Permission::ReadBudget)
        .await
        .unwrap();
    let current = b.find(&alice).await.unwrap();
    current
        .require_owner(Permission::ReadBudget, user.id())
        .unwrap();
    assert!(current
        .require_owner(Permission::ReadBudget, other.id())
        .is_err());
    assert!(current.require(Permission::WriteBudget).is_err());
    admin
        .revoke(user.id(), Permission::ReadBudget)
        .await
        .unwrap();
    admin
        .revoke(user.id(), Permission::ReadBudget)
        .await
        .unwrap();
    assert!(a.find(&alice).await.unwrap().permissions().is_empty());
    admin.set_enabled(user.id(), false).await.unwrap();
    assert!(matches!(b.find(&alice).await, Err(IdentityError::Disabled)));
    assert!(matches!(
        a.register_login(&alice).await,
        Err(IdentityError::Disabled)
    ));
    admin.set_enabled(user.id(), true).await.unwrap();
    assert_eq!(a.register_login(&alice).await.unwrap().id(), user.id());
}

use super::super::postgres_support::TestDatabase;
use super::*;
use obz_infrastructure::{DatabaseConfig, PostgresIdentityRepository};
use sqlx::Row;
use std::collections::BTreeSet;

use std::sync::Arc;

async fn setup() -> (
    TestDatabase,
    Arc<PostgresIdentityRepository>,
    Arc<PostgresIdentityRepository>,
) {
    let database = TestDatabase::create().await;
    let owner = Arc::new(
        PostgresIdentityRepository::connect(DatabaseConfig::new(database.owner_url.clone(), true))
            .await
            .unwrap(),
    );
    owner.migrate().await.unwrap();
    owner.migrate().await.unwrap(); // Migration runner is idempotent.
    owner
        .grant_runtime_role(&database.runtime_role)
        .await
        .unwrap();
    let runtime = Arc::new(
        PostgresIdentityRepository::connect(DatabaseConfig::new(
            database.runtime_url.clone(),
            true,
        ))
        .await
        .unwrap(),
    );
    runtime.check_schema().await.unwrap();
    (database, owner, runtime)
}
#[tokio::test]
#[ignore = "requires AUTH_TEST_DATABASE_URL and AUTH_TEST_RUNTIME_DATABASE_URL"]
async fn postgres_identity_contract_with_runtime_credentials_and_atomic_audit() {
    let (database, owner, runtime) = setup().await;
    let replica = Arc::new(
        PostgresIdentityRepository::connect(DatabaseConfig::new(
            database.runtime_url.clone(),
            true,
        ))
        .await
        .unwrap(),
    );
    identity_contract(runtime.clone(), replica, owner.clone()).await;
    let owner_pool = sqlx::PgPool::connect(&database.owner_url).await.unwrap();
    let rows = sqlx::query(
        "SELECT action, permission, actor FROM auth.audit_events ORDER BY occurred_at, id",
    )
    .fetch_all(&owner_pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 4);
    let actions: BTreeSet<String> = rows.iter().map(|row| row.get("action")).collect();
    assert_eq!(
        actions,
        BTreeSet::from([
            "grant".into(),
            "revoke".into(),
            "enable".into(),
            "disable".into()
        ])
    );
    assert!(rows
        .iter()
        .all(|row| !row.get::<String, _>("actor").is_empty()));
    database.destroy().await;
}
#[tokio::test]
#[ignore = "requires AUTH_TEST_DATABASE_URL and AUTH_TEST_RUNTIME_DATABASE_URL"]
async fn runtime_cannot_self_grant_change_identity_enable_accounts_or_forge_audit() {
    let (database, _owner, runtime) = setup().await;
    let owner_pool = sqlx::PgPool::connect(&database.owner_url).await.unwrap();
    let runtime_pool = sqlx::PgPool::connect(&database.runtime_url).await.unwrap();
    let identity = verified_identity().await;
    let user = runtime.register_login(&identity).await.unwrap();
    assert!(runtime
        .grant(user.id(), Permission::ReadBudget)
        .await
        .is_err());
    assert!(runtime.set_enabled(user.id(), false).await.is_err());
    for statement in [
            "UPDATE auth.users SET issuer = 'https://attacker.example'",
            "UPDATE auth.users SET subject = 'another-user'",
            "UPDATE auth.users SET enabled = false",
            "DELETE FROM auth.users",
            "INSERT INTO auth.audit_events (id, user_id, action) SELECT id, id, 'disable' FROM auth.users",
        ] { assert!(sqlx::query(statement).execute(&runtime_pool).await.is_err()); }
    assert!(runtime
        .find(&identity)
        .await
        .unwrap()
        .permissions()
        .is_empty());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.audit_events")
        .fetch_one(&owner_pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    // Database constraints also reject unknown permission names.
    assert!(
        sqlx::query("INSERT INTO auth.permissions (user_id, permission) VALUES ($1, 'admin')")
            .bind(user.id())
            .execute(&owner_pool)
            .await
            .is_err()
    );
    database.destroy().await;
}
#[tokio::test]
#[ignore = "requires AUTH_TEST_DATABASE_URL and AUTH_TEST_RUNTIME_DATABASE_URL"]
async fn permissions_survive_repository_reconstruction() {
    let (database, owner, runtime) = setup().await;
    let identity = verified_identity().await;
    let user = runtime.register_login(&identity).await.unwrap();
    owner
        .grant(user.id(), Permission::WriteBudget)
        .await
        .unwrap();
    drop(runtime);
    let replica = PostgresIdentityRepository::connect(DatabaseConfig::new(
        database.runtime_url.clone(),
        true,
    ))
    .await
    .unwrap();
    let persisted = replica.find(&identity).await.unwrap();
    assert_eq!(persisted.id(), user.id());
    persisted.require(Permission::WriteBudget).unwrap();
    database.destroy().await;
}
#[tokio::test]
async fn invalid_database_urls_are_rejected_without_exposing_credentials() {
    for url in [
        "http://localhost/database",
        "postgres://user@localhost/database",
        "postgres://user:secret@localhost/database?host=attacker",
        "postgres://user:secret@localhost/database#fragment",
    ] {
        assert!(matches!(
            PostgresIdentityRepository::connect(DatabaseConfig::new(url.into(), true)).await,
            Err(IdentityError::Configuration)
        ));
    }
}
