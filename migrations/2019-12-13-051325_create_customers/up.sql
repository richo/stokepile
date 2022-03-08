CREATE TABLE customers (
  id SERIAL PRIMARY KEY,
  user_id integer NOT NULL REFERENCES users(id),
  name varchar NOT NULL,
  address varchar NOT NULL,
  phone_number varchar NOT NULL,
  email varchar NOT NULL
);
