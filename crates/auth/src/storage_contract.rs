//! Shared behavioral tests for every storage implementation.
use crate::{
    session::unix_now, Identity, LoginAttempt, MemorySessionStore, SessionError, SessionStore,
};
use std::sync::Arc;

pub(crate) async fn contract(a: Arc<dyn SessionStore>, b: Arc<dyn SessionStore>) {
    let id = a.start_login(LoginAttempt::for_test(), None).await.unwrap();
    let (first, second) = tokio::join!(a.take_login(&id), b.take_login(&id));
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert!(a.take_login(&id).await.is_err());
    let first = a.start_login(LoginAttempt::for_test(), None).await.unwrap();
    let second = b.start_login(LoginAttempt::for_test(), None).await.unwrap();
    assert!(matches!(
        a.start_login(LoginAttempt::for_test(), None).await,
        Err(SessionError::Capacity)
    ));
    let replaced = b
        .start_login(LoginAttempt::for_test(), Some(&first))
        .await
        .unwrap();
    assert!(a.take_login(&first).await.is_err());
    b.take_login(&second).await.unwrap();
    a.take_login(&replaced).await.unwrap();

    let identity = || Identity::for_test("https://dex.example", "alice");
    let (first, lifetime) = a.authenticate(identity(), None).await.unwrap();
    assert!(lifetime <= 1800);
    let csrf = b.get(&first).await.unwrap().csrf_token().to_owned();
    assert_eq!(b.logout(&first, "wrong").await, Err(SessionError::Csrf));
    assert!(a.get(&first).await.is_ok());
    let (second, _) = b.authenticate(identity(), None).await.unwrap();
    assert!(matches!(
        a.authenticate(identity(), None).await,
        Err(SessionError::Capacity)
    ));
    let (rotated, _) = a.authenticate(identity(), Some(&first)).await.unwrap();
    assert_ne!(first, rotated);
    assert!(b.get(&first).await.is_err());
    assert_eq!(b.logout(&rotated, &csrf).await, Err(SessionError::Csrf));
    let rotated_csrf = b.get(&rotated).await.unwrap().csrf_token().to_owned();
    a.logout(&rotated, &rotated_csrf).await.unwrap();
    assert!(b.get(&rotated).await.is_err());
    let csrf = a.get(&second).await.unwrap().csrf_token().to_owned();
    b.logout(&second, &csrf).await.unwrap();

    let mut expiring = identity();
    let expires_at = unix_now() + 2;
    expiring.set_test_expiration(expires_at as i64);
    let (id, lifetime) = a.authenticate(expiring, None).await.unwrap();
    assert!(lifetime <= 2);
    assert_eq!(b.get(&id).await.unwrap().expires_at(), expires_at);
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    assert!(matches!(b.get(&id).await, Err(SessionError::Invalid)));
    assert!(a.authenticate(identity(), None).await.is_ok());
    a.cleanup().await.unwrap();
}

#[tokio::test]
async fn memory_backend_obeys_storage_contract() {
    let store = Arc::new(MemorySessionStore::new(2, 2));
    contract(store.clone(), store).await;
}
