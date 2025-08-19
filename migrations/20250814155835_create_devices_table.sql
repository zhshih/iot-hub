-- Add migration script here
CREATE TABLE devices (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    owner_id UUID NOT NULL,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

-- Optional index to speed up owner lookups
CREATE INDEX idx_devices_owner_id ON devices(owner_id);