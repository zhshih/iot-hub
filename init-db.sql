-- Create test database
CREATE DATABASE iot_monitoring_test;

-- Create test user with password
CREATE USER test_user WITH PASSWORD 'test_password';

-- Grant privileges to the test database
GRANT ALL PRIVILEGES ON DATABASE iot_monitoring_test TO test_user;

-- Connect to the test database and set schema ownership and privileges
\connect iot_monitoring_test;

ALTER SCHEMA public OWNER TO test_user;
GRANT ALL ON SCHEMA public TO test_user;
