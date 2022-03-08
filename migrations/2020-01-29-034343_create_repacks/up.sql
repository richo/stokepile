CREATE TABLE repacks (
  id SERIAL PRIMARY KEY,
  rigger integer NOT NULL REFERENCES users(id),
  equipment integer NOT NULL REFERENCES equipment(id),
  date DATE NOT NULL,
  service VARCHAR NOT NULL,
  location VARCHAR NOT NULL
);

CREATE INDEX repacks_by_equipment ON repacks (equipment);
CREATE INDEX repacks_by_rigger ON repacks (rigger);
