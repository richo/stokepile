ALTER TABLE users
DROP COLUMN staging_type,
DROP COLUMN staging_data;

DROP TYPE StagingKind;

CREATE TYPE StagingKind AS ENUM ('device', 'directory');

ALTER TABLE users
ADD COLUMN staging_type StagingKind NOT NULL DEFAULT 'directory',
ADD COLUMN staging_location VARCHAR;
