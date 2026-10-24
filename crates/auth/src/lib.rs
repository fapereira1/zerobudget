#![doc = include_str!("../README.md")]
//! Authentication through Dex OIDC and deny-by-default application authorization.
//! This crate does not interpret Dex access tokens as API JWTs.
mod authorization;
mod oidc;
mod session;

pub use authorization::{Permission, Policy};
pub use oidc::{AuthConfig, AuthError, DexAuth, Identity, LoginAttempt};

pub use session::{Session, SessionError, SessionStore};
