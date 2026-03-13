-- Add execution persistence to workflows
ALTER TABLE workflows ADD COLUMN last_execution_results JSONB;
ALTER TABLE workflows ADD COLUMN last_run_at TIMESTAMPTZ;
