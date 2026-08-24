-- Audit trail for device ownership transfers. pgcrypto (for gen_random_uuid())
-- is already enabled by the users-table migration.

CREATE TABLE ownership_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    previous_owner_id UUID NOT NULL REFERENCES users(id),
    new_owner_id UUID NOT NULL REFERENCES users(id),
    transferred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
