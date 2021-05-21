-- Add up migration script here
CREATE TYPE invitation_state AS ENUM (
    'ACCEPTED', 'PENDING', 'DECLINED'
);

ALTER TABLE invitations
ADD COLUMN state_enum invitation_state;

UPDATE invitations
SET state_enum = CAST(state AS invitation_state);

ALTER TABLE invitations
ALTER COLUMN state_enum SET NOT NULL;

ALTER TABLE invitations
DROP COLUMN state;

ALTER TABLE invitations
RENAME COLUMN state_enum TO state;