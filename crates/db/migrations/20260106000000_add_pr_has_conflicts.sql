-- Add has_conflicts field to merges table for tracking PR merge conflicts
ALTER TABLE merges ADD COLUMN pr_has_conflicts BOOLEAN DEFAULT FALSE;
