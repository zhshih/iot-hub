-- Dev-only seed data. NOT run automatically by `sqlx::migrate!` / `cargo run` / CI.
-- Run manually against a local dev database after migrations:
--   psql "$DATABASE_URL" -f scripts/seed_dev.sql
--
-- NOTE: passwords below are placeholder strings, not real Argon2 hashes.
-- These accounts cannot actually log in through the API as-is; this seed data
-- is for populating list/read endpoints during local development only.

INSERT INTO users (id, username, email, hashed_password, role, created_at)
VALUES
    (gen_random_uuid(), 'admin', 'admin@example.com', 'hashed_pw_1', 'Admin', NOW()),
    (gen_random_uuid(), 'operator1', 'op1@example.com', 'hashed_pw_2', 'Operator', NOW());

-- Devices reference the actual seeded users above via subquery, instead of a
-- disconnected random UUID (the bug in the old seed_dummy_devices.sql migration).
INSERT INTO devices (id, name, description, owner_id, registered_at, is_active)
VALUES
    (gen_random_uuid(), 'Thermostat X200', 'Smart thermostat for home automation',
        (SELECT id FROM users WHERE username = 'admin'), NOW() - interval '10 days', true),
    (gen_random_uuid(), 'Security Cam Pro', 'Outdoor wireless security camera',
        (SELECT id FROM users WHERE username = 'admin'), NOW() - interval '5 days', true),
    (gen_random_uuid(), 'Smart Lock Z', 'Keyless entry door lock',
        (SELECT id FROM users WHERE username = 'operator1'), NOW() - interval '3 days', false),
    (gen_random_uuid(), 'Weather Station V3', 'Advanced weather monitoring station',
        (SELECT id FROM users WHERE username = 'operator1'), NOW() - interval '1 day', true);

-- Readings reference a real seeded device via subquery, instead of a
-- disconnected random UUID.
INSERT INTO readings (id, device_id, arrived_timestamp, processed_timestamp, reading_type, value)
VALUES
    (gen_random_uuid(), (SELECT id FROM devices WHERE name = 'Thermostat X200'), now(), now(), 'temperature', 21.5),
    (gen_random_uuid(), (SELECT id FROM devices WHERE name = 'Thermostat X200'), now() - interval '5 minutes', now() - interval '5 minutes', 'temperature', 22.1),
    (gen_random_uuid(), (SELECT id FROM devices WHERE name = 'Weather Station V3'), now() - interval '10 minutes', now() - interval '10 minutes', 'humidity', 45.2),
    (gen_random_uuid(), (SELECT id FROM devices WHERE name = 'Weather Station V3'), now() - interval '15 minutes', now() - interval '15 minutes', 'voltage', 1013.25);
