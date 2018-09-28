CREATE TABLE sessions (
  id VARCHAR(32) NOT NULL PRIMARY KEY,
  user_id integer NOT NULL REFERENCES users(id),
  data jsonb DEFAULT '{}'::jsonb NOT NULL
);
