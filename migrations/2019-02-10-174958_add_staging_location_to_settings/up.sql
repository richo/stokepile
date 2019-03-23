CREATE TYPE StagingKind AS ENUM ('device', 'directory');

ALTER TABLE users
ADD COLUMN staging_type StagingKind,
ADD COLUMN staging_location VARCHAR;
