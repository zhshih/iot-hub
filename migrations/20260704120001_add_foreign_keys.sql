-- Add referential integrity that was missing from the original schema.
-- Safe to add now: seed data lives in scripts/seed_dev.sql (run manually,
-- never as part of `sqlx migrate run`), so a freshly migrated database has
-- no rows at all in devices/readings, let alone orphaned ones.

ALTER TABLE devices
    ADD CONSTRAINT fk_devices_owner FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE readings
    ADD CONSTRAINT fk_readings_device FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE;
