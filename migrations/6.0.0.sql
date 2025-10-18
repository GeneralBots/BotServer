
CREATE TABLE public.bots (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	"name" varchar(255) NOT NULL,
	description text NULL,
	llm_provider varchar(100) NOT NULL,
	llm_config jsonb DEFAULT '{}'::jsonb NOT NULL,
	context_provider varchar(100) NOT NULL,
	context_config jsonb DEFAULT '{}'::jsonb NOT NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	is_active bool DEFAULT true NULL,
	CONSTRAINT bots_pkey PRIMARY KEY (id)
);


-- public.clicks definition

-- Drop table

-- DROP TABLE public.clicks;

CREATE TABLE public.clicks (
	campaign_id text NOT NULL,
	email text NOT NULL,
	updated_at timestamptz DEFAULT now() NULL,
	CONSTRAINT clicks_campaign_id_email_key UNIQUE (campaign_id, email)
);


-- public.organizations definition

-- Drop table

-- DROP TABLE public.organizations;

CREATE TABLE public.organizations (
	org_id uuid DEFAULT gen_random_uuid() NOT NULL,
	"name" varchar(255) NOT NULL,
	slug varchar(255) NOT NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT organizations_pkey PRIMARY KEY (org_id),
	CONSTRAINT organizations_slug_key UNIQUE (slug)
);
CREATE INDEX idx_organizations_created_at ON public.organizations USING btree (created_at);
CREATE INDEX idx_organizations_slug ON public.organizations USING btree (slug);


-- public.system_automations definition

-- Drop table

-- DROP TABLE public.system_automations;

CREATE TABLE public.system_automations (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	kind int4 NOT NULL,
	"target" varchar(32) NULL,
	schedule bpchar(12) NULL,
	param varchar(32) NOT NULL,
	is_active bool DEFAULT true NOT NULL,
	last_triggered timestamptz NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT system_automations_pkey PRIMARY KEY (id)
);
CREATE INDEX idx_system_automations_active ON public.system_automations USING btree (kind) WHERE is_active;


-- public.tools definition

-- Drop table

-- DROP TABLE public.tools;

CREATE TABLE public.tools (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	"name" varchar(255) NOT NULL,
	description text NOT NULL,
	parameters jsonb DEFAULT '{}'::jsonb NOT NULL,
	script text NOT NULL,
	is_active bool DEFAULT true NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT tools_name_key UNIQUE (name),
	CONSTRAINT tools_pkey PRIMARY KEY (id)
);


-- public.users definition

-- Drop table

-- DROP TABLE public.users;

CREATE TABLE public.users (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	username varchar(255) NOT NULL,
	email varchar(255) NOT NULL,
	password_hash varchar(255) NOT NULL,
	phone_number varchar(50) NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	is_active bool DEFAULT true NULL,
	CONSTRAINT users_email_key UNIQUE (email),
	CONSTRAINT users_pkey PRIMARY KEY (id),
	CONSTRAINT users_username_key UNIQUE (username)
);


-- public.bot_channels definition

-- Drop table

-- DROP TABLE public.bot_channels;

CREATE TABLE public.bot_channels (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	bot_id uuid NOT NULL,
	channel_type int4 NOT NULL,
	config jsonb DEFAULT '{}'::jsonb NOT NULL,
	is_active bool DEFAULT true NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT bot_channels_bot_id_channel_type_key UNIQUE (bot_id, channel_type),
	CONSTRAINT bot_channels_pkey PRIMARY KEY (id),
	CONSTRAINT bot_channels_bot_id_fkey FOREIGN KEY (bot_id) REFERENCES public.bots(id) ON DELETE CASCADE
);
CREATE INDEX idx_bot_channels_type ON public.bot_channels USING btree (channel_type) WHERE is_active;


-- public.user_sessions definition

-- Drop table

-- DROP TABLE public.user_sessions;

CREATE TABLE public.user_sessions (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	user_id uuid NOT NULL,
	bot_id uuid NOT NULL,
	title varchar(500) DEFAULT 'New Conversation'::character varying NOT NULL,
	answer_mode int4 DEFAULT 0 NOT NULL,
	context_data jsonb DEFAULT '{}'::jsonb NOT NULL,
	current_tool varchar(255) NULL,
	message_count int4 DEFAULT 0 NOT NULL,
	total_tokens int4 DEFAULT 0 NOT NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	updated_at timestamptz DEFAULT now() NOT NULL,
	last_activity timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT user_sessions_pkey PRIMARY KEY (id),
	CONSTRAINT user_sessions_bot_id_fkey FOREIGN KEY (bot_id) REFERENCES public.bots(id) ON DELETE CASCADE,
	CONSTRAINT user_sessions_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE
);
CREATE INDEX idx_user_sessions_updated_at ON public.user_sessions USING btree (updated_at);
CREATE INDEX idx_user_sessions_user_bot ON public.user_sessions USING btree (user_id, bot_id);


-- public.whatsapp_numbers definition

-- Drop table

-- DROP TABLE public.whatsapp_numbers;

CREATE TABLE public.whatsapp_numbers (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	bot_id uuid NOT NULL,
	phone_number varchar(50) NOT NULL,
	is_active bool DEFAULT true NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT whatsapp_numbers_phone_number_bot_id_key UNIQUE (phone_number, bot_id),
	CONSTRAINT whatsapp_numbers_pkey PRIMARY KEY (id),
	CONSTRAINT whatsapp_numbers_bot_id_fkey FOREIGN KEY (bot_id) REFERENCES public.bots(id) ON DELETE CASCADE
);


-- public.context_injections definition

-- Drop table

-- DROP TABLE public.context_injections;

CREATE TABLE public.context_injections (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	session_id uuid NOT NULL,
	injected_by uuid NOT NULL,
	context_data jsonb NOT NULL,
	reason text NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT context_injections_pkey PRIMARY KEY (id),
	CONSTRAINT context_injections_injected_by_fkey FOREIGN KEY (injected_by) REFERENCES public.users(id) ON DELETE CASCADE,
	CONSTRAINT context_injections_session_id_fkey FOREIGN KEY (session_id) REFERENCES public.user_sessions(id) ON DELETE CASCADE
);


-- public.message_history definition

-- Drop table

-- DROP TABLE public.message_history;

CREATE TABLE public.message_history (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	session_id uuid NOT NULL,
	user_id uuid NOT NULL,
	"role" int4 NOT NULL,
	content_encrypted text NOT NULL,
	message_type int4 DEFAULT 0 NOT NULL,
	media_url text NULL,
	token_count int4 DEFAULT 0 NOT NULL,
	processing_time_ms int4 NULL,
	llm_model varchar(100) NULL,
	created_at timestamptz DEFAULT now() NOT NULL,
	message_index int4 NOT NULL,
	CONSTRAINT message_history_pkey PRIMARY KEY (id),
	CONSTRAINT message_history_session_id_fkey FOREIGN KEY (session_id) REFERENCES public.user_sessions(id) ON DELETE CASCADE,
	CONSTRAINT message_history_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE
);
CREATE INDEX idx_message_history_created_at ON public.message_history USING btree (created_at);
CREATE INDEX idx_message_history_session_id ON public.message_history USING btree (session_id);


-- public.usage_analytics definition

-- Drop table

-- DROP TABLE public.usage_analytics;

CREATE TABLE public.usage_analytics (
	id uuid DEFAULT gen_random_uuid() NOT NULL,
	user_id uuid NOT NULL,
	bot_id uuid NOT NULL,
	session_id uuid NOT NULL,
	"date" date DEFAULT CURRENT_DATE NOT NULL,
	message_count int4 DEFAULT 0 NOT NULL,
	total_tokens int4 DEFAULT 0 NOT NULL,
	total_processing_time_ms int4 DEFAULT 0 NOT NULL,
	CONSTRAINT usage_analytics_pkey PRIMARY KEY (id),
	CONSTRAINT usage_analytics_bot_id_fkey FOREIGN KEY (bot_id) REFERENCES public.bots(id) ON DELETE CASCADE,
	CONSTRAINT usage_analytics_session_id_fkey FOREIGN KEY (session_id) REFERENCES public.user_sessions(id) ON DELETE CASCADE,
	CONSTRAINT usage_analytics_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE
);
CREATE INDEX idx_usage_analytics_date ON public.usage_analytics USING btree (date);
