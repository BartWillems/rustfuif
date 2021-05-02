-- Add down migration script here
DROP FUNCTION IF EXISTS rustfuif_manage_updated_at(_tbl regclass);
DROP FUNCTION IF EXISTS rustfuif_set_updated_at();
