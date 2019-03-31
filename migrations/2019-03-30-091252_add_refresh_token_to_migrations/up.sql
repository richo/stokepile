ALTER TABLE integrations
ADD COLUMN refresh_token TEXT,
ADD COLUMN refreshed TIMESTAMP NOT NULL;
