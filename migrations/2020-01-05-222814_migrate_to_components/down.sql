DROP INDEX components_by_equipment;
DROP TABLE components;

ALTER TABLE equipment
  ADD COLUMN container varchar,
  ADD COLUMN reserve varchar,
  ADD COLUMN aad varchar;
