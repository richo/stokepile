CREATE TABLE devices (
  id SERIAL PRIMARY KEY,
  user_id integer NOT NULL REFERENCES users(id),
  serial TEXT NOT NULL
);

CREATE INDEX devices_by_user ON devices (user_id);
