-- Add column to store Linear issue labels as JSON
ALTER TABLE tasks ADD COLUMN linear_labels TEXT;
