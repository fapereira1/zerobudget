//! Bounded, single-process storage. Secrets never implement Debug or Serialize.
use crate::{Identity, LoginAttempt};
use openidconnect::CsrfToken;
use std::{
    collections::HashMap,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use subtle::ConstantTimeEq;

const LOGIN_TTL: Duration = Duration::from_secs(300);
const SESSION_TTL: u64 = 1800;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SessionError {
    #[error("session storage capacity reached")]
    Capacity,
    #[error("invalid or expired session")]
    Invalid,
    #[error("invalid CSRF token")]
    Csrf,
}

pub struct Session {
    identity: Identity,
    csrf: String,
    expires_at: u64,
}
impl Session {
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    pub fn csrf_token(&self) -> &str {
        &self.csrf
    }
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
    pub fn verify_csrf(&self, token: &str) -> Result<(), SessionError> {
        if bool::from(self.csrf.as_bytes().ct_eq(token.as_bytes())) {
            Ok(())
        } else {
            Err(SessionError::Csrf)
        }
    }
}
struct PendingLogin {
    attempt: LoginAttempt,
    created_at: Instant,
}

/// The caller synchronizes access and must not hold a lock during provider I/O.
pub struct SessionStore {
    pending: HashMap<String, PendingLogin>,
    sessions: HashMap<String, Session>,
    max_pending: usize,
    max_sessions: usize,
}
impl Default for SessionStore {
    fn default() -> Self {
        Self::new(1024, 4096)
    }
}
impl SessionStore {
    pub fn new(max_pending: usize, max_sessions: usize) -> Self {
        Self {
            pending: HashMap::new(),
            sessions: HashMap::new(),
            max_pending,
            max_sessions,
        }
    }
    pub fn cleanup(&mut self) {
        self.pending
            .retain(|_, pending| pending.created_at.elapsed() < LOGIN_TTL);
        let now = unix_now();
        self.sessions.retain(|_, session| session.expires_at > now);
    }
    pub fn start_login(
        &mut self,
        attempt: LoginAttempt,
        previous: Option<&str>,
    ) -> Result<String, SessionError> {
        self.cleanup();
        if let Some(previous) = previous {
            self.pending.remove(previous);
        }
        if self.pending.len() >= self.max_pending {
            return Err(SessionError::Capacity);
        }
        let id = random_secret();
        self.pending.insert(
            id.clone(),
            PendingLogin {
                attempt,
                created_at: Instant::now(),
            },
        );
        Ok(id)
    }
    /// Atomic removal binds a login to the browser cookie and prevents callback replay.
    pub fn take_login(&mut self, id: &str) -> Result<LoginAttempt, SessionError> {
        self.cleanup();
        self.pending
            .remove(id)
            .map(|pending| pending.attempt)
            .ok_or(SessionError::Invalid)
    }
    /// Never reuse a caller-supplied session ID. Rotate on every successful login.
    pub fn authenticate(
        &mut self,
        identity: Identity,
        previous: Option<&str>,
    ) -> Result<(String, u64), SessionError> {
        self.cleanup();
        let now = unix_now();
        let expires_at = u64::try_from(identity.expires_at())
            .map_err(|_| SessionError::Invalid)?
            .min(now.saturating_add(SESSION_TTL));
        if expires_at <= now {
            return Err(SessionError::Invalid);
        }
        if let Some(previous) = previous {
            self.sessions.remove(previous);
        }
        if self.sessions.len() >= self.max_sessions {
            return Err(SessionError::Capacity);
        }
        let id = random_secret();
        self.sessions.insert(
            id.clone(),
            Session {
                identity,
                csrf: random_secret(),
                expires_at,
            },
        );
        Ok((id, expires_at - now))
    }
    pub fn get(&mut self, id: &str) -> Result<&Session, SessionError> {
        self.cleanup();
        self.sessions.get(id).ok_or(SessionError::Invalid)
    }
    pub fn logout(&mut self, id: &str, csrf: &str) -> Result<(), SessionError> {
        self.get(id)?.verify_csrf(csrf)?;
        self.sessions.remove(id);
        Ok(())
    }
}
fn random_secret() -> String {
    CsrfToken::new_random().secret().to_owned()
}
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(u64::MAX, |time| time.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity() -> Identity {
        Identity::for_test("https://dex.example", "alice")
    }

    #[test]
    fn session_rotates_csrf_is_required_and_logout_revokes() {
        let mut store = SessionStore::default();
        let (first, lifetime) = store.authenticate(identity(), None).unwrap();
        assert!(lifetime <= SESSION_TTL);
        let csrf = store.get(&first).unwrap().csrf_token().to_owned();
        assert_eq!(store.logout(&first, "wrong"), Err(SessionError::Csrf));
        assert!(store.get(&first).is_ok());
        let (second, _) = store.authenticate(identity(), Some(&first)).unwrap();
        assert_ne!(first, second);
        assert!(store.get(&first).is_err());
        assert_eq!(store.logout(&second, &csrf), Err(SessionError::Csrf));
        let csrf = store.get(&second).unwrap().csrf_token().to_owned();
        store.logout(&second, &csrf).unwrap();
        assert!(store.get(&second).is_err());
    }
    #[test]
    fn session_capacity_and_expiration_are_enforced() {
        let mut store = SessionStore::new(1, 1);
        let (id, _) = store.authenticate(identity(), None).unwrap();
        assert_eq!(
            store.authenticate(identity(), None).unwrap_err(),
            SessionError::Capacity
        );
        store.sessions.get_mut(&id).unwrap().expires_at = 1;
        assert!(store.get(&id).is_err());
        assert!(store.authenticate(identity(), None).is_ok());
    }
    #[test]
    fn lifetime_does_not_exceed_identity_expiration() {
        let mut store = SessionStore::default();
        let mut identity = identity();
        identity.set_test_expiration((unix_now() + 10) as i64);
        let (id, lifetime) = store.authenticate(identity, None).unwrap();
        assert!(lifetime <= 10);
        assert!(store.get(&id).unwrap().expires_at() <= unix_now() + 10);
    }
}
