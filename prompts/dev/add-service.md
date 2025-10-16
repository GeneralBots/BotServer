Generate a Rust service module following these patterns:

Core Structure:

Use actix-web for HTTP endpoints (get, post, etc.)

Isolate shared resources (DB, clients, config) in AppState

Split logic into reusable helper functions

do not create main logic

Endpoints:

Follow REST conventions (e.g., POST /{resource}/create) use anotations in methods.

Use web::Path for route parameters, web::Json for payloads

Return consistent responses (e.g., HttpResponse::Ok().json(data))

Error Handling:

Wrap fallible operations in Result

Use map_err to convert errors to actix_web::Error

Provide clear error messages (e.g., ErrorInternalServerError)

Async Patterns:

Use async/await for I/O (DB, external APIs)

Leverage streams for pagination/large datasets

Isolate blocking ops in spawn_blocking if needed

Configuration:

Load settings (e.g., URLs, credentials) from AppConfig

Initialize clients (DB, SDKs) at startup (e.g., init_*() helpers)

Documentation:

Add brief doc comments for public functions

Note safety assumptions (e.g., #[post] invariants)
postgres sqlx
Omit domain-specific logic (e.g., file/email details), focusing on the scaffolding."

Key Features:

Generic (applies to any service: auth, payments, etc.)

KISS (avoids over-engineering)

Copy-paste friendly (clear patterns without verbosity)
