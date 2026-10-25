CREATE SCHEMA budgeting;
REVOKE ALL ON SCHEMA budgeting FROM PUBLIC;

CREATE TABLE budgeting.cycles (
    id uuid PRIMARY KEY,
    owner_id uuid NOT NULL REFERENCES auth.users(id),
    period_start date NOT NULL,
    period_end date NOT NULL,
    income_cents bigint NOT NULL CHECK (income_cents >= 0),
    currency text NOT NULL DEFAULT 'BRL' CHECK (currency = 'BRL'),
    status text NOT NULL DEFAULT 'planned' CHECK (status = 'planned'),
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (period_start >= DATE '0001-01-01' AND period_end <= DATE '9999-12-31' AND period_end > period_start),
    UNIQUE (owner_id, period_start, period_end)
);
CREATE INDEX cycles_owner_period_idx ON budgeting.cycles (owner_id, period_start DESC, id DESC);

ALTER TABLE budgeting.cycles ENABLE ROW LEVEL SECURITY;
ALTER TABLE budgeting.cycles FORCE ROW LEVEL SECURITY;

CREATE POLICY read_own_cycles ON budgeting.cycles FOR SELECT USING (
    owner_id = NULLIF(current_setting('zerobudget.user_id', true), '')::uuid
    AND EXISTS (
        SELECT 1 FROM auth.users u JOIN auth.permissions p ON p.user_id = u.id
        WHERE u.id = owner_id AND u.enabled AND p.permission = 'budget:read'
    )
);
CREATE POLICY insert_own_cycles ON budgeting.cycles FOR INSERT WITH CHECK (
    owner_id = NULLIF(current_setting('zerobudget.user_id', true), '')::uuid
    AND EXISTS (
        SELECT 1 FROM auth.users u JOIN auth.permissions p ON p.user_id = u.id
        WHERE u.id = owner_id AND u.enabled AND p.permission = 'budget:write'
    )
);
