CREATE TYPE StagingKind AS ENUM ('label', 'mountpoint', 'location');

ALTER TABLE users
ADD COLUMN staging_type StagingKind NOT NULL DEFAULT 'location',
ADD COLUMN staging_location VARCHAR;
