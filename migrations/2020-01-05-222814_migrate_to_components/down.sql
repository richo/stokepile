DROP INDEX components_by_equipment;
DROP TABLE components;

ALTER TABLE equipment
  ADD COLUMN container varchar NOT NULL,
  ADD COLUMN reserve varchar NOT NULL,
  ADD COLUMN aad varchar NOT NULL;
