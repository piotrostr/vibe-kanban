-- Add PR status badge fields to merges table
ALTER TABLE merges ADD COLUMN pr_is_draft BOOLEAN;
ALTER TABLE merges ADD COLUMN pr_review_decision TEXT;
ALTER TABLE merges ADD COLUMN pr_checks_status TEXT;
