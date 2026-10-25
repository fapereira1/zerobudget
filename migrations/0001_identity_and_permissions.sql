CREATE SCHEMA auth;
REVOKE ALL ON SCHEMA auth FROM PUBLIC;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;

CREATE TABLE auth.users (
    id uuid PRIMARY KEY,
    issuer text NOT NULL CHECK (octet_length(issuer) BETWEEN 1 AND 2048),
    subject text NOT NULL CHECK (octet_length(subject) BETWEEN 1 AND 255),
    enabled boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    last_login_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (issuer, subject)
);
CREATE TABLE auth.permissions (
    user_id uuid NOT NULL REFERENCES auth.users(id),
    permission text NOT NULL CHECK (permission IN ('budget:read', 'budget:write')),
    PRIMARY KEY (user_id, permission)
);
CREATE TABLE auth.audit_events (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES auth.users(id),
    action text NOT NULL CHECK (action IN ('grant', 'revoke', 'enable', 'disable')),
    permission text CHECK (permission IN ('budget:read', 'budget:write')),
    actor text NOT NULL DEFAULT session_user,
    occurred_at timestamptz NOT NULL DEFAULT now(),
    CHECK ((action IN ('grant', 'revoke') AND permission IS NOT NULL)
        OR (action IN ('enable', 'disable') AND permission IS NULL))
);
CREATE INDEX ON auth.audit_events (user_id, occurred_at);
