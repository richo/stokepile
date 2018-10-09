CREATE TABLE devices (
  id SERIAL NOT NULL PRIMARY KEY,
  user_id integer NOT NULL REFERENCES users(id),
  kind VARCHAR NOT NULL,
  identifier VARCHAR NOT NULL
);

CREATE INDEX devices_by_user ON devices (user_id);
