-- Add migration script here
INSERT INTO devices (id, name, description, owner_id, registered_at, is_active)
VALUES
    (gen_random_uuid(), 'Thermostat X200', 'Smart thermostat for home automation', gen_random_uuid(), NOW() - interval '10 days', true),
    (gen_random_uuid(), 'Security Cam Pro', 'Outdoor wireless security camera', gen_random_uuid(), NOW() - interval '5 days', true),
    (gen_random_uuid(), 'Smart Lock Z', 'Keyless entry door lock', gen_random_uuid(), NOW() - interval '3 days', false),
    (gen_random_uuid(), 'Weather Station V3', 'Advanced weather monitoring station', gen_random_uuid(), NOW() - interval '1 day', true);
