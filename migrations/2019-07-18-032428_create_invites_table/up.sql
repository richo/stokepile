CREATE TABLE invites (
  id SERIAL PRIMARY KEY,
  email varchar NOT NULL,
  consumed TIMESTAMP
);

CREATE UNIQUE INDEX invite_email ON invites(email);
