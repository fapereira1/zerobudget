use crate::{Identity, IdentityError, IdentityRepository, MemoryIdentityRepository, Permission};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

#[async_trait]
pub(crate) trait Admin: Send + Sync {
    async fn grant(&self, user: Uuid, permission: Permission) -> Result<(), IdentityError>;
    async fn revoke(&self, user: Uuid, permission: Permission) -> Result<(), IdentityError>;
    async fn enabled(&self, user: Uuid, enabled: bool) -> Result<(), IdentityError>;
}
#[async_trait]
impl Admin for MemoryIdentityRepository {
    async fn grant(&self, user: Uuid, permission: Permission) -> Result<(), IdentityError> {
        self.grant(user, permission).await
    }
    async fn revoke(&self, user: Uuid, permission: Permission) -> Result<(), IdentityError> {
        self.revoke(user, permission).await
    }
    async fn enabled(&self, user: Uuid, enabled: bool) -> Result<(), IdentityError> {
        self.set_enabled(user, enabled).await
    }
}
pub(crate) async fn contract(
    a: Arc<dyn IdentityRepository>,
    b: Arc<dyn IdentityRepository>,
    admin: Arc<dyn Admin>,
) {
    let alice = Identity::for_test("https://dex.example", "alice");
    assert!(matches!(a.find(&alice).await, Err(IdentityError::NotFound)));
    let (one, two) = tokio::join!(a.register_login(&alice), b.register_login(&alice));
    let user = one.unwrap();
    assert_eq!(user.id(), two.unwrap().id());
    assert!(user.permissions().is_empty());
    assert!(matches!(
        user.require(Permission::ReadBudget),
        Err(IdentityError::Forbidden)
    ));
    let other = b
        .register_login(&Identity::for_test("https://other.example", "alice"))
        .await
        .unwrap();
    assert_ne!(user.id(), other.id());
    let bob = a
        .register_login(&Identity::for_test("https://dex.example", "bob"))
        .await
        .unwrap();
    assert_ne!(user.id(), bob.id());

    admin
        .grant(user.id(), Permission::ReadBudget)
        .await
        .unwrap();
    admin
        .grant(user.id(), Permission::ReadBudget)
        .await
        .unwrap(); // Idempotent.
    let current = b.find(&alice).await.unwrap();
    current.require(Permission::ReadBudget).unwrap();
    assert!(current.require(Permission::WriteBudget).is_err());
    current
        .require_owner(Permission::ReadBudget, current.id())
        .unwrap();
    assert!(current
        .require_owner(Permission::ReadBudget, bob.id())
        .is_err());
    admin
        .revoke(user.id(), Permission::ReadBudget)
        .await
        .unwrap();
    admin
        .revoke(user.id(), Permission::ReadBudget)
        .await
        .unwrap();
    assert!(a
        .find(&alice)
        .await
        .unwrap()
        .require(Permission::ReadBudget)
        .is_err());
    admin.enabled(user.id(), false).await.unwrap();
    assert!(matches!(b.find(&alice).await, Err(IdentityError::Disabled)));
    assert!(matches!(
        a.register_login(&alice).await,
        Err(IdentityError::Disabled)
    ));
    admin.enabled(user.id(), true).await.unwrap();
    assert_eq!(a.register_login(&alice).await.unwrap().id(), user.id());
    let mut expired = alice.clone();
    expired.set_test_expiration(1);
    assert!(matches!(
        a.register_login(&expired).await,
        Err(IdentityError::Invalid)
    ));
    assert!(matches!(
        b.find(&expired).await,
        Err(IdentityError::Invalid)
    ));
}
#[tokio::test]
async fn memory_identity_repository_contract() {
    let repository = Arc::new(MemoryIdentityRepository::default());
    contract(repository.clone(), repository.clone(), repository).await;
}
