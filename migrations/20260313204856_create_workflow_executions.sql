-- Revert columns from workflows
ALTER TABLE workflows DROP COLUMN IF EXISTS last_execution_results;
ALTER TABLE workflows DROP COLUMN IF EXISTS last_run_at;

-- Create workflow_executions table
CREATE TABLE workflow_executions (
    id                UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id       UUID          NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    owner_id          UUID          NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status            TEXT          NOT NULL, -- 'running', 'completed', 'failed'
    results           JSONB         NOT NULL DEFAULT '{}',
    execution_time_ms BIGINT,
    created_at        TIMESTAMPTZ   NOT NULL DEFAULT now()
);

CREATE INDEX idx_workflow_executions_workflow ON workflow_executions(workflow_id);
CREATE INDEX idx_workflow_executions_owner ON workflow_executions(owner_id);
CREATE INDEX idx_workflow_executions_created ON workflow_executions(created_at DESC);
