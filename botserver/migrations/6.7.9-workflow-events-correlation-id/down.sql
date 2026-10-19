-- Restore the FK dropped by this migration. Rows written after it (channel
-- events carry a bot id in `workflow_id`) do not satisfy it; clear them first
-- so the inverse migration can apply.
UPDATE workflow_events SET workflow_id = NULL
WHERE workflow_id IS NOT NULL
  AND workflow_id NOT IN (SELECT id FROM workflow_executions);

ALTER TABLE workflow_events
    ADD CONSTRAINT workflow_events_workflow_id_fkey
    FOREIGN KEY (workflow_id) REFERENCES workflow_executions(id);
