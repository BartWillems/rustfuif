-- Add down migration script here
ALTER TABLE invitations
ALTER COLUMN state TYPE VARCHAR;

ALTER TABLE invitations
ALTER COLUMN state SET NOT NULL;

ALTER TABLE invitations
ADD CHECK(state = 'PENDING' OR state = 'ACCEPTED' OR state = 'DECLINED');

DROP TYPE invitation_state;
