CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  email varchar NOT NULL,
  password varchar NOT NULL
);

CREATE UNIQUE INDEX user_email ON users(email);
