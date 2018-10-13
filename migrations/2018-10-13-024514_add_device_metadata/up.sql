ALTER TABLE devices
ADD COLUMN metadata jsonb DEFAULT '{}'::jsonb NOT NULL;
