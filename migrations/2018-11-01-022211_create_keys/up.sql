CREATE TABLE keys (
  id SERIAL NOT NULL PRIMARY KEY,
  user_id integer NOT NULL REFERENCES users(id),
  token VARCHAR(32) NOT NULL,
  created TIMESTAMP NOT NULL,
  expired TIMESTAMP
);
