use crate::{
    identity::validate_identity, Identity, IdentityError, IdentityRepository, LocalUser, Permission,
};
use async_trait::async_trait;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
    ConnectOptions, PgPool, Row,
};
use std::{collections::BTreeSet, str::FromStr, time::Duration};
use uuid::Uuid;

/// Deliberately lacks Debug: URLs can contain credentials.
pub struct DatabaseConfig {
    pub url: String,
    pub allow_loopback_plaintext: bool,
    pub timeout: Duration,
    pub max_connections: u32,
}
impl DatabaseConfig {
    pub fn new(url: String, allow_loopback_plaintext: bool) -> Self {
        Self {
            url,
            allow_loopback_plaintext,
            timeout: Duration::from_secs(3),
            max_connections: 10,
        }
    }
}
#[derive(Clone)]
pub struct PostgresIdentityRepository {
    pool: PgPool,
    timeout: Duration,
}
const USER_SELECT: &str = "SELECT u.id, u.enabled, COALESCE(array_agg(p.permission ORDER BY p.permission) FILTER (WHERE p.permission IS NOT NULL), ARRAY[]::text[]) AS permissions FROM auth.users u LEFT JOIN auth.permissions p ON p.user_id = u.id WHERE u.issuer = $1 AND u.subject = $2 GROUP BY u.id";

impl PostgresIdentityRepository {
    pub async fn connect(config: DatabaseConfig) -> Result<Self, IdentityError> {
        if config.timeout.is_zero()
            || config.timeout > Duration::from_secs(30)
            || config.max_connections == 0
            || config.max_connections > 100
        {
            return Err(IdentityError::Configuration);
        }
        let url = url::Url::parse(&config.url).map_err(|_| IdentityError::Configuration)?;
        if !matches!(url.scheme(), "postgres" | "postgresql")
            || url.fragment().is_some()
            || url.host_str().is_none()
            || url.password().is_none_or(str::is_empty)
        {
            return Err(IdentityError::Configuration);
        }
        let local = config.allow_loopback_plaintext
            && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        // TLS verification cannot be weakened by sslmode in a production URL.
        // Prevent endpoint override through libpq-style URL options.
        if url
            .query_pairs()
            .any(|(name, _)| !matches!(name.as_ref(), "sslmode" | "sslrootcert"))
        {
            return Err(IdentityError::Configuration);
        }
        let options = PgConnectOptions::from_str(&config.url)
            .map_err(|_| IdentityError::Configuration)?
            .ssl_mode(if local {
                PgSslMode::Disable
            } else {
                PgSslMode::VerifyFull
            })
            .disable_statement_logging();
        let pool = tokio::time::timeout(
            config.timeout,
            PgPoolOptions::new()
                .max_connections(config.max_connections)
                .acquire_timeout(config.timeout)
                .connect_with(options),
        )
        .await
        .map_err(|_| IdentityError::Unavailable)?
        .map_err(|_| IdentityError::Unavailable)?;
        Ok(Self {
            pool,
            timeout: config.timeout,
        })
    }
    pub async fn check_schema(&self) -> Result<(), IdentityError> {
        self.bounded(async {
            sqlx::query("SELECT u.id, u.enabled, p.permission FROM auth.users u LEFT JOIN auth.permissions p ON p.user_id = u.id WHERE false").execute(&self.pool).await?;
            Ok(())
        }).await
    }
    pub async fn migrate(&self) -> Result<(), IdentityError> {
        tokio::time::timeout(
            Duration::from_secs(30),
            sqlx::migrate!("./migrations").run(&self.pool),
        )
        .await
        .map_err(|_| IdentityError::Unavailable)?
        .map_err(|_| IdentityError::Unavailable)
    }
    async fn bounded<T>(
        &self,
        operation: impl std::future::Future<Output = Result<T, sqlx::Error>>,
    ) -> Result<T, IdentityError> {
        tokio::time::timeout(self.timeout, operation)
            .await
            .map_err(|_| IdentityError::Unavailable)?
            .map_err(|_| IdentityError::Unavailable)
    }
    /// Administrative provisioning only. The API never calls this method.
    pub async fn grant_runtime_role(&self, role: &str) -> Result<(), IdentityError> {
        if role.is_empty()
            || role.len() > 63
            || !role.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
        {
            return Err(IdentityError::Configuration);
        }
        self.bounded(async {
            let mut transaction = self.pool.begin().await?;
            for statement in [
                format!("GRANT USAGE ON SCHEMA auth TO \"{role}\""),
                format!("GRANT SELECT ON auth.users, auth.permissions TO \"{role}\""),
                format!("GRANT INSERT (id, issuer, subject, last_login_at), UPDATE (last_login_at) ON auth.users TO \"{role}\""),
            ] { sqlx::query(&statement).execute(&mut *transaction).await?; }
            transaction.commit().await
        }).await
    }
    pub async fn grant(&self, user_id: Uuid, permission: Permission) -> Result<(), IdentityError> {
        self.change_permission(user_id, permission, true).await
    }
    pub async fn revoke(&self, user_id: Uuid, permission: Permission) -> Result<(), IdentityError> {
        self.change_permission(user_id, permission, false).await
    }
    async fn change_permission(
        &self,
        user_id: Uuid,
        permission: Permission,
        grant: bool,
    ) -> Result<(), IdentityError> {
        // Serialize administrative changes per user, with audit in the same transaction.
        let mut transaction = tokio::time::timeout(self.timeout, self.pool.begin())
            .await
            .map_err(|_| IdentityError::Unavailable)?
            .map_err(|_| IdentityError::Unavailable)?;
        let action = if grant { "grant" } else { "revoke" };
        let result = tokio::time::timeout(self.timeout, async {
            let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM auth.users WHERE id = $1 FOR UPDATE").bind(user_id).fetch_optional(&mut *transaction).await?;
            if exists.is_none() { return Ok::<_, sqlx::Error>(false); }
            let changed = if grant {
                sqlx::query("INSERT INTO auth.permissions (user_id, permission) VALUES ($1, $2) ON CONFLICT DO NOTHING").bind(user_id).bind(permission.as_str()).execute(&mut *transaction).await?.rows_affected()
            } else {
                sqlx::query("DELETE FROM auth.permissions WHERE user_id = $1 AND permission = $2").bind(user_id).bind(permission.as_str()).execute(&mut *transaction).await?.rows_affected()
            };
            if changed > 0 {
                sqlx::query("INSERT INTO auth.audit_events (id, user_id, action, permission) VALUES ($1, $2, $3, $4)").bind(Uuid::new_v4()).bind(user_id).bind(action).bind(permission.as_str()).execute(&mut *transaction).await?;
            }
            transaction.commit().await?;
            Ok(true)
        }).await.map_err(|_| IdentityError::Unavailable)?.map_err(|_| IdentityError::Unavailable)?;
        if result {
            Ok(())
        } else {
            Err(IdentityError::NotFound)
        }
    }
    pub async fn set_enabled(&self, user_id: Uuid, enabled: bool) -> Result<(), IdentityError> {
        self.bounded(async {
            let mut transaction = self.pool.begin().await?;
            let changed = sqlx::query(
                "UPDATE auth.users SET enabled = $2 WHERE id = $1 AND enabled IS DISTINCT FROM $2",
            )
            .bind(user_id)
            .bind(enabled)
            .execute(&mut *transaction)
            .await?
            .rows_affected();
            if changed > 0 {
                sqlx::query(
                    "INSERT INTO auth.audit_events (id, user_id, action) VALUES ($1, $2, $3)",
                )
                .bind(Uuid::new_v4())
                .bind(user_id)
                .bind(if enabled { "enable" } else { "disable" })
                .execute(&mut *transaction)
                .await?;
            }
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM auth.users WHERE id = $1)")
                    .bind(user_id)
                    .fetch_one(&mut *transaction)
                    .await?;
            transaction.commit().await?;
            Ok(exists)
        })
        .await
        .and_then(|exists| {
            if exists {
                Ok(())
            } else {
                Err(IdentityError::NotFound)
            }
        })
    }
}
#[async_trait]
impl IdentityRepository for PostgresIdentityRepository {
    async fn register_login(&self, identity: &Identity) -> Result<LocalUser, IdentityError> {
        validate_identity(identity)?;
        self.bounded(async {
            sqlx::query("INSERT INTO auth.users (id, issuer, subject, last_login_at) VALUES ($1, $2, $3, now()) ON CONFLICT (issuer, subject) DO UPDATE SET last_login_at = EXCLUDED.last_login_at WHERE auth.users.enabled")
                .bind(Uuid::new_v4()).bind(identity.issuer()).bind(identity.subject()).execute(&self.pool).await?;
            Ok(())
        }).await?;
        self.find(identity).await
    }
    async fn find(&self, identity: &Identity) -> Result<LocalUser, IdentityError> {
        validate_identity(identity)?;
        let row = self
            .bounded(
                sqlx::query(USER_SELECT)
                    .bind(identity.issuer())
                    .bind(identity.subject())
                    .fetch_optional(&self.pool),
            )
            .await?
            .ok_or(IdentityError::NotFound)?;
        let enabled: bool = row
            .try_get("enabled")
            .map_err(|_| IdentityError::Unavailable)?;
        if !enabled {
            return Err(IdentityError::Disabled);
        }
        let permissions: Vec<String> = row
            .try_get("permissions")
            .map_err(|_| IdentityError::Unavailable)?;
        let permissions: BTreeSet<Permission> = permissions
            .into_iter()
            .map(|permission| permission.parse().map_err(|_| IdentityError::Unavailable))
            .collect::<Result<_, _>>()?;
        Ok(LocalUser {
            id: row.try_get("id").map_err(|_| IdentityError::Unavailable)?,
            permissions,
        })
    }
}

#[cfg(test)]
#[path = "../tests/support/postgres.rs"]
mod support;

#[cfg(test)]
mod tests {
    use super::support::TestDatabase;
    use super::*;
    use std::sync::Arc;

    async fn setup() -> (
        TestDatabase,
        Arc<PostgresIdentityRepository>,
        Arc<PostgresIdentityRepository>,
    ) {
        let database = TestDatabase::create().await;
        let owner = Arc::new(
            PostgresIdentityRepository::connect(DatabaseConfig::new(
                database.owner_url.clone(),
                true,
            ))
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
        crate::identity_tests::contract(runtime.clone(), replica, owner.clone()).await;
        let rows = sqlx::query(
            "SELECT action, permission, actor FROM auth.audit_events ORDER BY occurred_at, id",
        )
        .fetch_all(&owner.pool)
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
        let (database, owner, runtime) = setup().await;
        let identity = Identity::for_test("https://dex.example", "alice");
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
        ] { assert!(sqlx::query(statement).execute(&runtime.pool).await.is_err()); }
        assert!(runtime
            .find(&identity)
            .await
            .unwrap()
            .permissions()
            .is_empty());
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.audit_events")
            .fetch_one(&owner.pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
        // Database constraints also reject unknown permission names.
        assert!(sqlx::query(
            "INSERT INTO auth.permissions (user_id, permission) VALUES ($1, 'admin')"
        )
        .bind(user.id())
        .execute(&owner.pool)
        .await
        .is_err());
        database.destroy().await;
    }
    #[tokio::test]
    #[ignore = "requires AUTH_TEST_DATABASE_URL and AUTH_TEST_RUNTIME_DATABASE_URL"]
    async fn permissions_survive_repository_reconstruction() {
        let (database, owner, runtime) = setup().await;
        let identity = Identity::for_test("https://dex.example", "alice");
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
}
