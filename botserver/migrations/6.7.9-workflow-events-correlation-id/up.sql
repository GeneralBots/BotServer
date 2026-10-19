-- `workflow_events.workflow_id` is a correlation id, not a FK.
--
-- `6.2.0-01-automation` created the column as
-- `workflow_id UUID REFERENCES workflow_executions(id)`, but the channel event
-- bus (`botcore::shared::basic_events::publish`) stores the *publishing bot*
-- in it so the dispatcher can resolve the bot without a second lookup. Every
-- such insert was rejected by the foreign key (a bot id is not an execution
-- id), so `ON EVENT "media_uploaded" CALL "<tool>"` never queued anything.
--
-- Workflow-scoped events (`PUBLISH EVENT`) leave the column NULL and are now
-- distinguished by that, instead of the non-existent `event_type` column the
-- dispatcher used to filter on.

ALTER TABLE workflow_events
    DROP CONSTRAINT IF EXISTS workflow_events_workflow_id_fkey;
