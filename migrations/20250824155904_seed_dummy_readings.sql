-- Add migration script here
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

INSERT INTO readings (id, device_id, arrived_timestamp, processed_timestamp, reading_type, value)
VALUES
  (gen_random_uuid(), gen_random_uuid(), now(), now(), 'temperature', 21.5),
  (gen_random_uuid(), gen_random_uuid(), now() - interval '5 minutes', now() - interval '5 minutes', 'temperature', 22.1),
  (gen_random_uuid(), gen_random_uuid(), now() - interval '10 minutes', now() - interval '10 minutes', 'humidity', 45.2),
  (gen_random_uuid(), gen_random_uuid(), now() - interval '15 minutes', now() - interval '15 minutes', 'voltage', 1013.25);
