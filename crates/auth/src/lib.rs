#![doc = include_str!("../README.md")]
//! Authentication through Dex OIDC and deny-by-default application authorization.
//! This crate does not interpret Dex access tokens as API JWTs.
mod authorization;
mod identity;
mod oidc;
mod postgres_identity;
mod records;
mod redis_store;
mod session;

pub use authorization::{Permission, Policy};
pub use oidc::{AuthConfig, AuthError, DexAuth, Identity, LoginAttempt};

pub use session::{MemorySessionStore, Session, SessionError, SessionStore};

pub use redis_store::{RedisSessionStore, RedisStoreConfig};

#[cfg(test)]
mod storage_contract;

pub use identity::{IdentityError, IdentityRepository, LocalUser, MemoryIdentityRepository};
pub use postgres_identity::{DatabaseConfig, PostgresIdentityRepository};
#[cfg(test)]
mod identity_tests;
