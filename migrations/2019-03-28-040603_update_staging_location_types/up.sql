/* This is literally the down for the previous migration. Rather than trying to juggle all this data through we're just going to erase it. */
ALTER TABLE users
DROP COLUMN staging_type,
DROP COLUMN staging_location;

DROP TYPE StagingKind;

CREATE TYPE StagingKind AS ENUM ('none', 'mountpoint', 'label');

ALTER TABLE users
/* none is a bad value but now we can spit out a useful error message when it happens */
ADD COLUMN staging_type StagingKind NOT NULL DEFAULT 'none',
/* Being more deliberate this time around about how this is opaque data */
ADD COLUMN staging_data VARCHAR;
