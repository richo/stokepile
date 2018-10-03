CREATE TABLE integrations (
  id SERIAL PRIMARY KEY,
  user_id integer NOT NULL REFERENCES users(id),
  provider TEXT NOT NULL,
  access_token TEXT NOT NULL
);

CREATE INDEX integrations_by_user ON integrations (user_id);
