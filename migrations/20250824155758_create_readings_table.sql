-- Add migration script here
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE readings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id UUID NOT NULL,
    arrived_timestamp TIMESTAMPTZ NOT NULL,
    processed_timestamp TIMESTAMPTZ NOT NULL,
    reading_type TEXT NOT NULL,
    value DOUBLE PRECISION NOT NULL
);