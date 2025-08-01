INSERT INTO users (id, username, email, hashed_password, role, created_at)
VALUES
  (gen_random_uuid(), 'admin', 'admin@example.com', 'hashed_pw_1', 'Admin', NOW()),
  (gen_random_uuid(), 'operator1', 'op1@example.com', 'hashed_pw_2', 'Operator', NOW());
