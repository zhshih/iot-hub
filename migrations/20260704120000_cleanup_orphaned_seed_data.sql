-- Add migration script here
DELETE FROM devices WHERE owner_id NOT IN (SELECT id FROM users);
DELETE FROM readings WHERE device_id NOT IN (SELECT id FROM devices);
