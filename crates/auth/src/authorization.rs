use std::collections::{BTreeMap, BTreeSet};

use crate::{AuthError, Identity};

/// Application permissions are assigned by trusted application configuration,
/// never by a request body or an unverified email address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum Permission {
    #[serde(rename = "budget:read")]
    ReadBudget,
    #[serde(rename = "budget:write")]
    WriteBudget,
}

impl Permission {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadBudget => "budget:read",
            Self::WriteBudget => "budget:write",
        }
    }
}
impl std::str::FromStr for Permission {
    type Err = crate::IdentityError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "budget:read" => Ok(Self::ReadBudget),
            "budget:write" => Ok(Self::WriteBudget),
            _ => Err(crate::IdentityError::Invalid),
        }
    }
}

/// Grants are scoped to the stable OIDC pair (issuer, subject).
#[derive(Debug, Default)]
pub struct Policy {
    grants: BTreeMap<(String, String), BTreeSet<Permission>>,
}

impl Policy {
    pub fn grant(&mut self, issuer: &str, subject: &str, permission: Permission) {
        self.grants
            .entry((issuer.to_owned(), subject.to_owned()))
            .or_default()
            .insert(permission);
    }

    pub fn require(&self, identity: &Identity, permission: Permission) -> Result<(), AuthError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| AuthError::InvalidToken)?;
        if now.as_secs() >= identity.expires_at() as u64 || identity.expires_at() < 0 {
            return Err(AuthError::InvalidToken);
        }
        match self
            .grants
            .get(&(identity.issuer().to_owned(), identity.subject().to_owned()))
        {
            Some(grants) if grants.contains(&permission) => Ok(()),
            _ => Err(AuthError::Forbidden),
        }
    }

    /// A permission does not imply access to another user's financial records.
    pub fn require_owner(
        &self,
        identity: &Identity,
        permission: Permission,
        owner_issuer: &str,
        owner_subject: &str,
    ) -> Result<(), AuthError> {
        self.require(identity, permission)?;
        if identity.issuer() != owner_issuer || identity.subject() != owner_subject {
            return Err(AuthError::Forbidden);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grants_are_explicit_scoped_and_do_not_bypass_ownership() {
        let identity = Identity::for_test("https://dex.example", "alice");
        let mut policy = Policy::default();
        assert!(policy.require(&identity, Permission::ReadBudget).is_err());
        policy.grant(
            identity.issuer(),
            identity.subject(),
            Permission::ReadBudget,
        );
        assert!(policy.require(&identity, Permission::ReadBudget).is_ok());
        assert!(policy.require(&identity, Permission::WriteBudget).is_err());
        assert!(policy
            .require(
                &Identity::for_test("https://other.example", "alice"),
                Permission::ReadBudget
            )
            .is_err());
        assert!(policy
            .require_owner(&identity, Permission::ReadBudget, identity.issuer(), "bob")
            .is_err());
        assert!(policy
            .require_owner(
                &identity,
                Permission::ReadBudget,
                identity.issuer(),
                "alice"
            )
            .is_ok());
    }
}
