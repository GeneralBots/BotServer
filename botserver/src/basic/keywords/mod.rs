// ===== LOCAL KEYWORDS =====
pub mod keyword_reference;
pub mod mention_config;
pub mod preview;

// ===== RE-EXPORTED FROM botbasic_core =====
pub use botbasic_core::keywords::arrays;
pub use botbasic_core::keywords::core_functions;
pub use botbasic_core::keywords::datetime;
pub use botbasic_core::keywords::errors;
pub use botbasic_core::keywords::first;
pub use botbasic_core::keywords::for_next;
pub use botbasic_core::keywords::format;
pub use botbasic_core::keywords::hear_talk;
pub use botbasic_core::keywords::hearing;
pub use botbasic_core::keywords::last;
pub use botbasic_core::keywords::math;
pub use botbasic_core::keywords::print;
pub use botbasic_core::keywords::procedures;
pub use botbasic_core::keywords::set_context;
pub use botbasic_core::keywords::set_user;
pub use botbasic_core::keywords::string_functions;
pub use botbasic_core::keywords::switch_case;
pub use botbasic_core::keywords::validation;
pub use botbasic_core::keywords::wait;

// ===== RE-EXPORTED FROM botbasic_data =====
pub use botbasic_data::keywords::bot_memory;
pub use botbasic_data::keywords::clear_kb;
#[cfg(feature = "people")]
pub use botbasic_data::keywords::crm;
pub use botbasic_data::keywords::data_operations;
#[cfg(feature = "people")]
pub use botbasic_data::keywords::db_api;
pub use botbasic_data::keywords::detect;
pub use botbasic_data::keywords::find;
pub use botbasic_data::keywords::get;
pub use botbasic_data::keywords::import_export;
pub use botbasic_data::keywords::kb_statistics;
#[cfg(feature = "people")]
pub use botbasic_data::keywords::lead_scoring;
#[cfg(all(feature = "billing", feature = "multimodal"))]
pub use botbasic_data::keywords::products;
pub use botbasic_data::keywords::save_from_unstructured;
pub use botbasic_data::keywords::search;
pub use botbasic_data::keywords::set;
pub use botbasic_data::keywords::table_access;
pub use botbasic_data::keywords::table_definition;
pub use botbasic_data::keywords::table_migration;
pub use botbasic_data::keywords::think_kb;
pub use botbasic_data::keywords::use_account;
pub use botbasic_data::keywords::use_kb;
pub use botbasic_data::keywords::user_memory;

// ===== RE-EXPORTED FROM botbasic_comms =====
#[cfg(feature = "chat")]
pub use botbasic_comms::keywords::add_bot;
#[cfg(feature = "chat")]
pub use botbasic_comms::keywords::add_member;
#[cfg(feature = "chat")]
pub use botbasic_comms::keywords::add_suggestion;
#[cfg(feature = "calendar")]
pub use botbasic_comms::keywords::book;
#[cfg(feature = "mail")]
pub use botbasic_comms::keywords::create_draft;
pub use botbasic_comms::keywords::messaging;
#[cfg(feature = "mail")]
pub use botbasic_comms::keywords::on_email;
#[cfg(feature = "meet")]
pub use botbasic_comms::keywords::play;
#[cfg(feature = "mail")]
pub use botbasic_comms::keywords::send_mail;
#[cfg(feature = "mail")]
pub use botbasic_comms::keywords::send_template;
#[cfg(any(feature = "whatsapp", feature = "telegram", feature = "mail"))]
pub use botbasic_comms::keywords::sms;
#[cfg(feature = "social")]
pub use botbasic_comms::keywords::social;
#[cfg(feature = "social")]
pub use botbasic_comms::keywords::social_media;
#[cfg(feature = "chat")]
pub use botbasic_comms::keywords::switcher;
#[cfg(feature = "chat")]
pub use botbasic_comms::keywords::transfer_to_human;
pub use botbasic_comms::keywords::universal_messaging;
#[cfg(feature = "video")]
pub use botbasic_comms::keywords::weather;
#[cfg(feature = "automation")]
pub use botbasic_comms::keywords::webhook;

// ===== RE-EXPORTED FROM botbasic_ai =====
pub use botbasic_ai::keywords::agent_reflection;
#[cfg(feature = "llm")]
pub use botbasic_ai::keywords::ai_tools;
#[cfg(feature = "automation")]
pub use botbasic_ai::keywords::api_tool_generator;
pub use botbasic_ai::keywords::clear_tools;
#[cfg(feature = "automation")]
pub use botbasic_ai::keywords::code_sandbox;
pub use botbasic_ai::keywords::enhanced_llm;
pub use botbasic_ai::keywords::enhanced_memory;
#[cfg(feature = "llm")]
pub use botbasic_ai::keywords::episodic_memory;
pub use botbasic_ai::keywords::events;
pub use botbasic_ai::keywords::http_operations;
pub use botbasic_ai::keywords::human_approval;
#[cfg(feature = "llm")]
pub use botbasic_ai::keywords::knowledge_graph;
#[cfg(feature = "llm")]
pub use botbasic_ai::keywords::llm_keyword;
#[cfg(feature = "llm")]
pub use botbasic_ai::keywords::llm_macros;
#[cfg(feature = "automation")]
pub use botbasic_ai::keywords::mcp_client;
#[cfg(feature = "automation")]
pub use botbasic_ai::keywords::mcp_directory;
#[cfg(feature = "llm")]
pub use botbasic_ai::keywords::model_routing;
#[cfg(feature = "multimodal")]
pub use botbasic_ai::keywords::multimodal;
#[cfg(feature = "automation")]
pub use botbasic_ai::keywords::on_form_submit;
pub use botbasic_ai::keywords::orchestration;
pub use botbasic_ai::keywords::qrcode;
#[cfg(feature = "llm")]
pub use botbasic_ai::keywords::remember;
pub use botbasic_ai::keywords::use_tool;
pub use botbasic_ai::keywords::use_website;
pub use botbasic_ai::keywords::web_data;

// ===== RE-EXPORTED FROM botbasic_system =====
pub use botbasic_system::keywords::app_server;
#[cfg(feature = "drive")]
pub use botbasic_system::keywords::create_site;
#[cfg(feature = "tasks")]
pub use botbasic_system::keywords::create_task;
pub use botbasic_system::keywords::face_api;
#[cfg(feature = "drive")]
pub use botbasic_system::keywords::file_operations;
#[cfg(feature = "automation")]
pub use botbasic_system::keywords::on;
#[cfg(feature = "automation")]
pub use botbasic_system::keywords::on_change;
#[cfg(feature = "security")]
pub use botbasic_system::keywords::security_protection;
pub use botbasic_system::keywords::on_update;
pub use botbasic_system::keywords::set_schedule;
#[cfg(feature = "automation")]
pub use botbasic_system::keywords::synchronize;

// ===== LOCAL MODULES (botserver-only, no crate counterpart) =====
pub mod card;
#[cfg(feature = "social")]
pub mod post_to;
pub mod set_answer_mode;
pub mod invoke_action;
pub mod vibe_agent;

pub fn get_all_keywords() -> Vec<String> {
    vec![
        "ADD BOT".to_string(),
        "BOT REFLECTION".to_string(),
        "BROADCAST TO BOTS".to_string(),
        "DELEGATE TO BOT".to_string(),
        "TRANSFER CONVERSATION".to_string(),
        "ADD MEMBER".to_string(),
        "CREATE DRAFT".to_string(),
        "SEND MAIL".to_string(),
        "SEND TEMPLATE".to_string(),
        "SMS".to_string(),
        "ADD SUGGESTION".to_string(),
        "ADD_SUGGESTION_TOOL".to_string(),
        "ADD SWITCHER".to_string(),
        "ADD_SWITCHER".to_string(),
        "CLEAR SWITCHERS".to_string(),
        "CLEAR SUGGESTIONS".to_string(),
        "ADD TOOL".to_string(),
        "CLEAR TOOLS".to_string(),
        "CREATE SITE".to_string(),
        "CREATE TASK".to_string(),
        "DETECT".to_string(),
        "USE TOOL".to_string(),
        "AGGREGATE".to_string(),
        "DELETE".to_string(),
        "FILL".to_string(),
        "FILTER".to_string(),
        "FIND".to_string(),
        "FIRST".to_string(),
        "SEARCH".to_string(),
        "GROUP BY".to_string(),
        "INSERT".to_string(),
        "JOIN".to_string(),
        "LAST".to_string(),
        "MAP".to_string(),
        "MERGE".to_string(),
        "PIVOT".to_string(),
        "SAVE".to_string(),
        "SAVE FROM UNSTRUCTURED".to_string(),
        "UPDATE".to_string(),
        "COMPRESS".to_string(),
        "COPY".to_string(),
        "DELETE FILE".to_string(),
        "DOWNLOAD".to_string(),
        "EXTRACT".to_string(),
        "GENERATE PDF".to_string(),
        "LIST".to_string(),
        "MERGE PDF".to_string(),
        "MOVE".to_string(),
        "READ".to_string(),
        "UPLOAD".to_string(),
        "WRITE".to_string(),
        "CLEAR HEADERS".to_string(),
        "DELETE HTTP".to_string(),
        "GET".to_string(),
        "GRAPHQL".to_string(),
        "PATCH".to_string(),
        "POST".to_string(),
        "PUT".to_string(),
        "SET HEADER".to_string(),
        "SOAP".to_string(),
        "EXIT FOR".to_string(),
        "FOR EACH".to_string(),
        "IF".to_string(),
        "SWITCH".to_string(),
        "WAIT".to_string(),
        "WHILE".to_string(),
        "GET".to_string(),
        "SET".to_string(),
        "GET BOT MEMORY".to_string(),
        "GET USER MEMORY".to_string(),
        "REMEMBER".to_string(),
        "SET BOT MEMORY".to_string(),
        "SET CONTEXT".to_string(),
        "SET USER FACT".to_string(),
        "SET USER MEMORY".to_string(),
        "USER FACTS".to_string(),
        "CLEAR KB".to_string(),
        "THINK KB".to_string(),
        "USE KB".to_string(),
        "USE ACCOUNT".to_string(),
        "USE WEBSITE".to_string(),
        "LLM".to_string(),
        "SET CONTEXT".to_string(),
        "USE MODEL".to_string(),
        "RUN BASH".to_string(),
        "RUN JAVASCRIPT".to_string(),
        "RUN PYTHON".to_string(),
        "HEAR".to_string(),
        "TALK".to_string(),
        "ON".to_string(),
        "ON EMAIL".to_string(),
        "ON CHANGE".to_string(),
        "SET ANSWER MODE".to_string(),
        "SET SCHEDULE".to_string(),
        "WEBHOOK".to_string(),
        "SET USER".to_string(),
        "BOOK".to_string(),
        "WEATHER".to_string(),
        "PRINT".to_string(),
        "FORMAT".to_string(),
        "INSTR".to_string(),
        "IS NUMERIC".to_string(),
        "REQUIRE APPROVAL".to_string(),
        "SIMULATE IMPACT".to_string(),
        "CHECK CONSTRAINTS".to_string(),
        "AUDIT LOG".to_string(),
        "PLAN START".to_string(),
        "PLAN END".to_string(),
        "STEP".to_string(),
        "AUTO TASK".to_string(),
        "USE MCP".to_string(),
        "MCP LIST TOOLS".to_string(),
        "MCP INVOKE".to_string(),
        "OPTION A OR B".to_string(),
        "DECIDE".to_string(),
        "ESCALATE".to_string(),
        "SECURITY TOOL STATUS".to_string(),
        "SECURITY RUN SCAN".to_string(),
        "SECURITY GET REPORT".to_string(),
        "SECURITY UPDATE DEFINITIONS".to_string(),
        "SECURITY START SERVICE".to_string(),
        "SECURITY STOP SERVICE".to_string(),
        "SECURITY INSTALL TOOL".to_string(),
        "SECURITY HARDENING SCORE".to_string(),
        // Filesystem: only DELETE FILE used to be listed, so a script calling
        // CREATE FILE / WRITE FILE was told by the "closed set" that it did not
        // exist while the runtime registers all of them.
        "CREATE FILE".to_string(),
        "WRITE FILE".to_string(),
        "READ FILE".to_string(),
        "LIST FILES".to_string(),
        "GET FILE".to_string(),
        // HTTP verbs: same gap as the file verbs.
        "GET HTTP".to_string(),
        "POST HTTP".to_string(),
        "PUT HTTP".to_string(),
        "WEBHOOK".to_string(),
        // Messaging.
        "SEND SMS".to_string(),
        "SEND TO".to_string(),
        // Session memory.
        "REMEMBER".to_string(),
        "RECALL".to_string(),
        // Perception / media, used by the shipped media-filing tools.
        "DESCRIBE IMAGE".to_string(),
        "DESCRIBE VIDEO".to_string(),
        "SPEECH TO TEXT".to_string(),
        "CLASSIFY".to_string(),
        // Declarative control.
        "ON EVENT".to_string(),
        "TRANSFER TO HUMAN".to_string(),
        // Registered engine functions, not multi-word keywords, but the scripts
        // use them as if they were keywords and the reference must teach the
        // exact form (`TODAY`/`NOW` are registered functions, so the call form
        // is what compiles).
        "COUNT".to_string(),
        "SPLIT".to_string(),
        "TRIM".to_string(),
        "UPPER".to_string(),
        "LEN".to_string(),
        "REPLACE".to_string(),
        "LEFT".to_string(),
        "STR".to_string(),
        "TODAY".to_string(),
        "NOW".to_string(),
    ]
}
