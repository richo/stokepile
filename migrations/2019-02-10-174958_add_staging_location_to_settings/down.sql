ALTER TABLE users
DROP COLUMN staging_type,
DROP COLUMN staging_location;

DROP TYPE StagingKind;
