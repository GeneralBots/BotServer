CREATE TABLE public.system_automations (
	id uuid NOT NULL,
	kind int4 NULL,
	target varchar(32) NULL,
	schedule bpchar(12) NULL,
	param varchar(32) NOT NULL,
	is_active bool DEFAULT true NOT NULL,
	last_triggered timestamptz NULL,
	CONSTRAINT system_automations_pkey PRIMARY KEY (id)
);
CREATE INDEX idx_active_automations ON public.system_automations USING btree (kind) WHERE is_active;
