use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::collections::HashMap;

/// All default configuration values
/// Mirrors packages/env/src/constants/defaults.ts
pub static DEFAULTS: Lazy<HashMap<&'static str, Value>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // Server
    m.insert("HOST", json!("0.0.0.0"));
    m.insert("PORT", json!(8055));
    m.insert("PUBLIC_URL", json!("/"));
    m.insert("MAX_PAYLOAD_SIZE", json!("1mb"));
    m.insert("ROOT_REDIRECT", json!("./admin"));
    m.insert("SERVE_APP", json!(true));
    m.insert("SERVER_SHUTDOWN_TIMEOUT", json!(1000));

    // Query
    m.insert("MAX_RELATIONAL_DEPTH", json!(10));
    m.insert("QUERYSTRING_MAX_PARSE_DEPTH", json!(10));
    m.insert("QUERYSTRING_ARRAY_LIMIT", json!(500));
    m.insert("QUERY_LIMIT_DEFAULT", json!(100));
    m.insert("MAX_BATCH_MUTATION", json!(f64::INFINITY));
    m.insert("MAX_IMPORT_ERRORS", json!(1000));

    // Database
    m.insert("DB_EXCLUDE_TABLES", json!("spatial_ref_sys,sysdiagrams"));

    // Storage
    m.insert("STORAGE_LOCATIONS", json!("local"));
    m.insert("STORAGE_LOCAL_DRIVER", json!("local"));
    m.insert("STORAGE_LOCAL_ROOT", json!("./uploads"));

    // Rate limiting
    m.insert("RATE_LIMITER_ENABLED", json!(false));
    m.insert("RATE_LIMITER_POINTS", json!(50));
    m.insert("RATE_LIMITER_DURATION", json!(1));
    m.insert("RATE_LIMITER_STORE", json!("memory"));
    m.insert("RATE_LIMITER_GLOBAL_ENABLED", json!(false));
    m.insert("RATE_LIMITER_GLOBAL_POINTS", json!(1000));
    m.insert("RATE_LIMITER_GLOBAL_DURATION", json!(1));
    m.insert("RATE_LIMITER_REGISTRATION_ENABLED", json!(true));
    m.insert("RATE_LIMITER_REGISTRATION_POINTS", json!(5));
    m.insert("RATE_LIMITER_REGISTRATION_DURATION", json!(60));

    // Auth tokens
    m.insert("ACCESS_TOKEN_TTL", json!("15m"));
    m.insert("REFRESH_TOKEN_TTL", json!("7d"));
    m.insert("REFRESH_TOKEN_COOKIE_NAME", json!("nexus_refresh_token"));
    m.insert("REFRESH_TOKEN_COOKIE_SECURE", json!(false));
    m.insert("REFRESH_TOKEN_COOKIE_SAME_SITE", json!("lax"));
    m.insert("SESSION_COOKIE_TTL", json!("1d"));
    m.insert("SESSION_COOKIE_NAME", json!("nexus_session_token"));
    m.insert("SESSION_COOKIE_SECURE", json!(false));
    m.insert("SESSION_COOKIE_SAME_SITE", json!("lax"));
    m.insert("SESSION_REFRESH_GRACE_PERIOD", json!("10s"));
    m.insert("USER_INVITE_TOKEN_TTL", json!("7d"));
    m.insert("EMAIL_VERIFICATION_TOKEN_TTL", json!("7d"));
    m.insert("LOGIN_STALL_TIME", json!(500));
    m.insert("REGISTER_STALL_TIME", json!(750));
    m.insert("AUTH_PROVIDERS", json!(""));
    m.insert("AUTH_DISABLE_DEFAULT", json!(false));

    // CORS
    m.insert("CORS_ENABLED", json!(false));
    m.insert("CORS_ORIGIN", json!(false));
    m.insert("CORS_METHODS", json!("GET,POST,PATCH,DELETE"));
    m.insert("CORS_ALLOWED_HEADERS", json!("Content-Type,Authorization"));
    m.insert("CORS_EXPOSED_HEADERS", json!("Content-Range"));
    m.insert("CORS_CREDENTIALS", json!(true));
    m.insert("CORS_MAX_AGE", json!(18000));

    // Cache
    m.insert("CACHE_ENABLED", json!(false));
    m.insert("CACHE_STORE", json!("memory"));
    m.insert("CACHE_TTL", json!("5m"));
    m.insert("CACHE_NAMESPACE", json!("system-cache"));
    m.insert("CACHE_AUTO_PURGE", json!(false));
    m.insert("CACHE_SCHEMA", json!(true));
    m.insert("CACHE_SCHEMA_MAX_ITERATIONS", json!(100));
    m.insert("CACHE_SCHEMA_SYNC_TIMEOUT", json!(10000));
    m.insert("CACHE_SKIP_ALLOWED", json!(false));

    // Extensions
    m.insert("EXTENSIONS_PATH", json!("./extensions"));
    m.insert("EXTENSIONS_MUST_LOAD", json!(false));
    m.insert("EXTENSIONS_AUTO_RELOAD", json!(false));
    m.insert("EXTENSIONS_SANDBOX_MEMORY", json!(100));
    m.insert("EXTENSIONS_SANDBOX_TIMEOUT", json!(1000));

    // Email
    m.insert("EMAIL_FROM", json!("no-reply@example.com"));
    m.insert("EMAIL_VERIFY_SETUP", json!(true));
    m.insert("EMAIL_TRANSPORT", json!("sendmail"));
    m.insert("EMAIL_TEMPLATES_PATH", json!("./templates"));

    // Assets
    m.insert("ASSETS_CACHE_TTL", json!("30d"));
    m.insert("ASSETS_TRANSFORM_MAX_CONCURRENT", json!(25));
    m.insert("ASSETS_TRANSFORM_IMAGE_MAX_DIMENSION", json!(6000));
    m.insert("ASSETS_TRANSFORM_MAX_OPERATIONS", json!(5));

    // GraphQL
    m.insert("GRAPHQL_INTROSPECTION", json!(true));
    m.insert("GRAPHQL_SCHEMA_GENERATION_MAX_CONCURRENT", json!(5));
    m.insert("GRAPHQL_QUERY_TOKEN_LIMIT", json!(5000));

    // WebSocket
    m.insert("WEBSOCKETS_ENABLED", json!(false));
    m.insert("WEBSOCKETS_REST_ENABLED", json!(true));
    m.insert("WEBSOCKETS_REST_AUTH", json!("handshake"));
    m.insert("WEBSOCKETS_REST_AUTH_TIMEOUT", json!(10));
    m.insert("WEBSOCKETS_REST_PATH", json!("/websocket"));
    m.insert("WEBSOCKETS_GRAPHQL_ENABLED", json!(true));
    m.insert("WEBSOCKETS_GRAPHQL_AUTH", json!("handshake"));
    m.insert("WEBSOCKETS_GRAPHQL_AUTH_TIMEOUT", json!(10));
    m.insert("WEBSOCKETS_GRAPHQL_PATH", json!("/graphql"));
    m.insert("WEBSOCKETS_HEARTBEAT_ENABLED", json!(true));
    m.insert("WEBSOCKETS_HEARTBEAT_PERIOD", json!(30));

    // Flows
    m.insert("FLOWS_ENV_ALLOW_LIST", json!(false));
    m.insert("FLOWS_RUN_SCRIPT_MAX_MEMORY", json!(32));
    m.insert("FLOWS_RUN_SCRIPT_TIMEOUT", json!(10000));

    // Pressure limiter
    m.insert("PRESSURE_LIMITER_ENABLED", json!(true));
    m.insert("PRESSURE_LIMITER_SAMPLE_INTERVAL", json!(250));
    m.insert("PRESSURE_LIMITER_MAX_EVENT_LOOP_UTILIZATION", json!(0.99));
    m.insert("PRESSURE_LIMITER_MAX_EVENT_LOOP_DELAY", json!(500));

    // Telemetry
    m.insert("TELEMETRY", json!(true));

    // Files
    m.insert("FILES_MIME_TYPE_ALLOW_LIST", json!("*/*"));
    m.insert("TUS_ENABLED", json!(false));
    m.insert("TUS_CHUNK_SIZE", json!(8_388_608)); // 8MB

    // Robots
    m.insert("ROBOTS_TXT", json!("User-agent: *\nDisallow: /"));

    // Data retention
    m.insert("RETENTION_ENABLED", json!(false));
    m.insert("ACTIVITY_RETENTION", json!("90d"));
    m.insert("REVISIONS_RETENTION", json!("90d"));
    m.insert("FLOW_LOGS_RETENTION", json!("90d"));

    // Batch sizes
    m.insert("RELATIONAL_BATCH_SIZE", json!(25000));
    m.insert("EXPORT_BATCH_SIZE", json!(5000));

    // User limits
    m.insert("USERS_ADMIN_ACCESS_LIMIT", json!(f64::INFINITY));
    m.insert("USERS_APP_ACCESS_LIMIT", json!(f64::INFINITY));
    m.insert("USERS_API_ACCESS_LIMIT", json!(f64::INFINITY));

    // IP
    m.insert("IP_TRUST_PROXY", json!(true));
    m.insert("IP_CUSTOM_HEADER", json!(false));

    // Misc
    m.insert("ACCEPT_TERMS", json!(false));

    m
});
