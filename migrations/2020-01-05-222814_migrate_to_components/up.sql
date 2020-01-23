CREATE TABLE components (
  id SERIAL PRIMARY KEY,
  equipment_id integer NOT NULL REFERENCES equipment(id),
  kind varchar NOT NULL,
  manufacturer varchar NOT NULL,
  model varchar NOT NULL,
  serial varchar NOT NULL,
  manufactured TIMESTAMP NOT NULL,

  -- Extra data associated with this component
  data jsonb DEFAULT '{}'::jsonb NOT NULL
);

-- No attempt to migrate the existing components we never had enough information
ALTER TABLE equipment
  DROP COLUMN container,
  DROP COLUMN reserve,
  DROP COLUMN aad;

CREATE INDEX components_by_equipment ON components (equipment_id);
