-- This schema defines the global state for the application.
-- There will only ever be a single row in this table (hopefully) enforced by that constraint.
-- If you're creating new settings, make sure the default is reasonable since it will immediately be applied to the (only) row.
-- Columns must never be nullable and must always have a default value.

CREATE TABLE global_settings (
  onerow_id bool PRIMARY KEY DEFAULT TRUE,

  invites_required BOOLEAN NOT NULL DEFAULT TRUE,


  -- This is the constraint that guarantees only ever one row.
  CONSTRAINT onerow_uni CHECK (onerow_id)
);

INSERT INTO global_settings VALUES (
  TRUE, TRUE);

