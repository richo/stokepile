CREATE TABLE confirmation_tokens (
  id SERIAL PRIMARY KEY,
  user_id integer NOT NULL REFERENCES users(id),
  token TEXT NOT NULL
);

CREATE UNIQUE INDEX confirmation_tokens_by_user ON confirmation_tokens (user_id);
