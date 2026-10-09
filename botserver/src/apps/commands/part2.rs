use super::*;

/// Deep-link params declared per app id.
pub static APP_DEEP_LINKS: &[(&str, &[DeepLinkParam])] = &[
    (
        "chat",
        &[DeepLinkParam {
            key: "q",
            description: "prefilled prompt",
            example: "resume my last conversation",
        }],
    ),
    (
        "vibe",
        &[DeepLinkParam {
            key: "run_id",
            description: "assistant run id",
            example: "123e4567-e89b-12d3-a456-426614174000",
        }],
    ),
    (
        "research",
        &[DeepLinkParam {
            key: "q",
            description: "search query",
            example: "market size 2026",
        }],
    ),
    (
        "video",
        &[DeepLinkParam {
            key: "project_id",
            description: "video project id",
            example: "123e4567-e89b-12d3-a456-426614174000",
        }],
    ),
    (
        "jukebox",
        &[DeepLinkParam {
            key: "job_id",
            description: "music generation job id",
            example: "music-job-42",
        }],
    ),
    (
        "vision",
        &[DeepLinkParam {
            key: "image_id",
            description: "analysis id",
            example: "analysis-1",
        }],
    ),
    (
        "learn",
        &[DeepLinkParam {
            key: "course_id",
            description: "course id",
            example: "course-42",
        }],
    ),
    (
        "mail",
        &[DeepLinkParam {
            key: "message_id",
            description: "email message id",
            example: "msg-91",
        }],
    ),
    (
        "calendar",
        &[DeepLinkParam {
            key: "event_id",
            description: "calendar event id",
            example: "evt-7",
        }],
    ),
    (
        "meet",
        &[DeepLinkParam {
            key: "meeting_id",
            description: "meeting id",
            example: "room-3",
        }],
    ),
    (
        "docs",
        &[DeepLinkParam {
            key: "file",
            description: "drive file",
            example: "proposta/contrato.docx",
        }],
    ),
    (
        "sheet",
        &[DeepLinkParam {
            key: "file_id",
            description: "spreadsheet id",
            example: "fluxo-2026-08",
        }],
    ),
    (
        "slides",
        &[DeepLinkParam {
            key: "deck_id",
            description: "presentation id",
            example: "deck-12",
        }],
    ),
    (
        "paper",
        &[DeepLinkParam {
            key: "note_id",
            description: "note id",
            example: "note-5",
        }],
    ),
    (
        "tasks",
        &[DeepLinkParam {
            key: "task_id",
            description: "task id",
            example: "123e4567-e89b-12d3-a456-426614174000",
        }],
    ),
    (
        "plan",
        &[DeepLinkParam {
            key: "plan_id",
            description: "plan/roadmap id",
            example: "plan-2",
        }],
    ),
    (
        "goals",
        &[DeepLinkParam {
            key: "okr_id",
            description: "OKR id",
            example: "okr-1",
        }],
    ),
    (
        "minutes",
        &[DeepLinkParam {
            key: "minutes_id",
            description: "meeting minutes id",
            example: "123e4567-e89b-12d3-a456-426614174000",
        }],
    ),
    (
        "timeclock",
        &[DeepLinkParam {
            key: "entry_id",
            description: "time entry id",
            example: "entry-9",
        }],
    ),
    (
        "templates",
        &[DeepLinkParam {
            key: "template_id",
            description: "template id",
            example: "tpl-4",
        }],
    ),
    (
        "designer",
        &[DeepLinkParam {
            key: "page_id",
            description: "designer page id",
            example: "pg-8",
        }],
    ),
    (
        "crm",
        &[DeepLinkParam {
            key: "person_id",
            description: "person/contact id",
            example: "123e4567-e89b-12d3-a456-426614174000",
        }],
    ),
    (
        "people",
        &[DeepLinkParam {
            key: "person_id",
            description: "person/lead id",
            example: "123e4567-e89b-12d3-a456-426614174000",
        }],
    ),
    (
        "campaigns",
        &[DeepLinkParam {
            key: "campaign_id",
            description: "campaign id",
            example: "cmp-3",
        }],
    ),
    (
        "lists",
        &[DeepLinkParam {
            key: "list_id",
            description: "structured list id",
            example: "lst-1",
        }],
    ),
    (
        "billing",
        &[DeepLinkParam {
            key: "invoice_id",
            description: "invoice id",
            example: "123e4567-e89b-12d3-a456-426614174000",
        }],
    ),
    (
        "products",
        &[DeepLinkParam {
            key: "product_id",
            description: "product/service id",
            example: "123e4567-e89b-12d3-a456-426614174000",
        }],
    ),
    (
        "tickets",
        &[DeepLinkParam {
            key: "ticket_id",
            description: "support ticket id",
            example: "123-55",
        }],
    ),
    (
        "hr",
        &[DeepLinkParam {
            key: "employee_id",
            description: "employee id",
            example: "emp-21",
        }],
    ),
    (
        "banking",
        &[DeepLinkParam {
            key: "transaction_id",
            description: "bank transaction id",
            example: "123e4567-e89b-12d3-a456-426614174000",
        }],
    ),
    (
        "pos",
        &[DeepLinkParam {
            key: "order_id",
            description: "POS order id",
            example: "ord-77",
        }],
    ),
    (
        "retail",
        &[DeepLinkParam {
            key: "product_id",
            description: "stock item id",
            example: "sku-100",
        }],
    ),
    (
        "handoff",
        &[DeepLinkParam {
            key: "handoff_id",
            description: "handoff session id",
            example: "hd-6",
        }],
    ),
    (
        "kyc",
        &[DeepLinkParam {
            key: "verification_id",
            description: "KYC verification id",
            example: "vrf-3",
        }],
    ),
    (
        "fraud",
        &[DeepLinkParam {
            key: "case_id",
            description: "fraud case id",
            example: "case-12",
        }],
    ),
    (
        "compliance",
        &[
            DeepLinkParam {
                key: "audit_id",
                description: "audit log id",
                example: "aud-9",
            },
            DeepLinkParam {
                key: "framework_id",
                description: "compliance framework id",
                example: "123e4567-e89b-12d3-a456-426614174000",
            },
        ],
    ),
    (
        "tax",
        &[DeepLinkParam {
            key: "doc_id",
            description: "tax document id",
            example: "nfe-55",
        }],
    ),
    (
        "social",
        &[DeepLinkParam {
            key: "post_id",
            description: "social post id",
            example: "post-2",
        }],
    ),
    (
        "attendant",
        &[DeepLinkParam {
            key: "queue_id",
            description: "attendant queue id",
            example: "q-4",
        }],
    ),
    (
        "editor",
        &[DeepLinkParam {
            key: "file_id",
            description: "file path",
            example: "src/main.rs",
        }],
    ),
    (
        "bas-editor",
        &[DeepLinkParam {
            key: "script_id",
            description: "BASIC script path",
            example: "start.bas",
        }],
    ),
    (
        "database",
        &[DeepLinkParam {
            key: "table",
            description: "table name",
            example: "customers",
        }],
    ),
    (
        "browser",
        &[DeepLinkParam {
            key: "url",
            description: "page url",
            example: "https://generalbots.org",
        }],
    ),
    (
        "integrations",
        &[
            DeepLinkParam {
                key: "provider",
                description: "provider catalog id",
                example: "aws",
            },
            DeepLinkParam {
                key: "connector_id",
                description: "configured connection id",
                example: "conn-5",
            },
        ],
    ),
    (
        "canvas",
        &[DeepLinkParam {
            key: "canvas_id",
            description: "canvas/wboard id",
            example: "cv-2",
        }],
    ),
    (
        "workspace",
        &[DeepLinkParam {
            key: "page_id",
            description: "workspace page id",
            example: "pg-11",
        }],
    ),
    (
        "project",
        &[DeepLinkParam {
            key: "project_id",
            description: "project id",
            example: "123e4567-e89b-12d3-a456-426614174000",
        }],
    ),
    (
        "analytics",
        &[DeepLinkParam {
            key: "report_id",
            description: "report id",
            example: "rpt-3",
        }],
    ),
    (
        "drive",
        &[DeepLinkParam {
            key: "path",
            description: "drive folder/file path",
            example: "faturas%2F2026-08",
        }],
    ),
    (
        "player",
        &[DeepLinkParam {
            key: "media_id",
            description: "media stream id",
            example: "cam-1",
        }],
    ),
    (
        "itsm",
        &[DeepLinkParam {
            key: "incident_id",
            description: "incident id",
            example: "inc-15",
        }],
    ),
];
