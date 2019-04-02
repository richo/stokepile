CREATE TABLE email_confirmations (
  id SERIAL PRIMARY KEY,
  user_id integer NOT NULL REFERENCES users(id),
  token TEXT NOT NULL
);

CREATE UNIQUE INDEX email_confirmation_by_user ON email_confirmations (user_id);
