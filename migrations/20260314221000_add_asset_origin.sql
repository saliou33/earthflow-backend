ALTER TABLE assets ADD COLUMN IF NOT EXISTS origin VARCHAR(20) NOT NULL DEFAULT 'user';
ALTER TABLE assets ADD COLUMN IF NOT EXISTS execution_id UUID REFERENCES workflow_executions(id) ON DELETE CASCADE;
ALTER TABLE workflow_executions ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
CREATE INDEX IF NOT EXISTS idx_assets_origin ON assets(origin);
CREATE INDEX IF NOT EXISTS idx_assets_execution_id ON assets(execution_id);
