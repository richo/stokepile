CREATE TABLE equipment (
  id SERIAL PRIMARY KEY,
  user_id integer NOT NULL REFERENCES users(id),
  customer_id integer NOT NULL REFERENCES customers(id),
  -- TODO(richo) Figure out how to make this freeform but still normalized enough
  -- that we can look up service bulls etc.
  container varchar NOT NULL,
  reserve varchar NOT NULL,
  aad varchar NOT NULL
);
