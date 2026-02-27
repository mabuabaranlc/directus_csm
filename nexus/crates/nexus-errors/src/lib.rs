use actix_web::http::StatusCode;
use serde::Serialize;

/// Error codes matching Directus ErrorCode enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum ErrorCode {
    #[serde(rename = "CONTAINS_NULL_VALUES")]
    ContainsNullValues,
    #[serde(rename = "CONTENT_TOO_LARGE")]
    ContentTooLarge,
    #[serde(rename = "EMAIL_LIMIT_EXCEEDED")]
    EmailLimitExceeded,
    #[serde(rename = "FORBIDDEN")]
    Forbidden,
    #[serde(rename = "ILLEGAL_ASSET_TRANSFORMATION")]
    IllegalAssetTransformation,
    #[serde(rename = "INTERNAL_SERVER_ERROR")]
    Internal,
    #[serde(rename = "INVALID_CREDENTIALS")]
    InvalidCredentials,
    #[serde(rename = "INVALID_FOREIGN_KEY")]
    InvalidForeignKey,
    #[serde(rename = "INVALID_IP")]
    InvalidIp,
    #[serde(rename = "INVALID_OTP")]
    InvalidOtp,
    #[serde(rename = "INVALID_PAYLOAD")]
    InvalidPayload,
    #[serde(rename = "INVALID_PATH_PARAMETER")]
    InvalidPathParameter,
    #[serde(rename = "INVALID_PROVIDER")]
    InvalidProvider,
    #[serde(rename = "INVALID_PROVIDER_CONFIG")]
    InvalidProviderConfig,
    #[serde(rename = "INVALID_QUERY")]
    InvalidQuery,
    #[serde(rename = "INVALID_TOKEN")]
    InvalidToken,
    #[serde(rename = "LIMIT_EXCEEDED")]
    LimitExceeded,
    #[serde(rename = "METHOD_NOT_ALLOWED")]
    MethodNotAllowed,
    #[serde(rename = "NOT_NULL_VIOLATION")]
    NotNullViolation,
    #[serde(rename = "OUT_OF_DATE")]
    OutOfDate,
    #[serde(rename = "RANGE_NOT_SATISFIABLE")]
    RangeNotSatisfiable,
    #[serde(rename = "RECORD_NOT_UNIQUE")]
    RecordNotUnique,
    #[serde(rename = "REQUESTS_EXCEEDED")]
    RequestsExceeded,
    #[serde(rename = "ROUTE_NOT_FOUND")]
    RouteNotFound,
    #[serde(rename = "SERVICE_UNAVAILABLE")]
    ServiceUnavailable,
    #[serde(rename = "TOKEN_EXPIRED")]
    TokenExpired,
    #[serde(rename = "UNEXPECTED_RESPONSE")]
    UnexpectedResponse,
    #[serde(rename = "UNPROCESSABLE_CONTENT")]
    UnprocessableContent,
    #[serde(rename = "UNSUPPORTED_MEDIA_TYPE")]
    UnsupportedMediaType,
    #[serde(rename = "USER_SUSPENDED")]
    UserSuspended,
    #[serde(rename = "VALUE_OUT_OF_RANGE")]
    ValueOutOfRange,
    #[serde(rename = "VALUE_TOO_LONG")]
    ValueTooLong,
}

/// The main Nexus error type — mirrors Directus' createError pattern
#[derive(Debug, thiserror::Error)]
pub enum NexusError {
    #[error("Field \"{field}\" in collection \"{collection}\" contains null values.")]
    ContainsNullValues { collection: String, field: String },

    #[error("Uploaded content is too large.")]
    ContentTooLarge,

    #[error("Email sending limit exceeded.")]
    EmailLimitExceeded {
        points: Option<u64>,
        duration: Option<u64>,
        message: Option<String>,
    },

    #[error("{}", reason.as_deref().unwrap_or("You don't have permission to access this."))]
    Forbidden { reason: Option<String> },

    #[error("Illegal asset transformation.")]
    IllegalAssetTransformation {
        invalid_transformations: Vec<String>,
    },

    #[error("An unexpected error occurred.")]
    Internal,

    #[error("Invalid user credentials.")]
    InvalidCredentials,

    #[error("Invalid foreign key for field in collection.")]
    InvalidForeignKey {
        collection: Option<String>,
        field: Option<String>,
        value: Option<String>,
    },

    #[error("Invalid IP address.")]
    InvalidIp,

    #[error("Invalid user OTP.")]
    InvalidOtp,

    #[error("Invalid payload. {reason}.")]
    InvalidPayload { reason: String },

    #[error("Invalid path parameter. {reason}.")]
    InvalidPathParameter { reason: String },

    #[error("Invalid provider.")]
    InvalidProvider,

    #[error("Invalid config for provider \"{provider}\".")]
    InvalidProviderConfig {
        provider: String,
        reason: Option<String>,
    },

    #[error("Invalid query. {reason}.")]
    InvalidQuery { reason: String },

    #[error("Invalid token.")]
    InvalidToken,

    #[error("{category} limit exceeded.")]
    LimitExceeded { category: String },

    #[error("Invalid method \"{current}\" used.")]
    MethodNotAllowed {
        allowed: Vec<String>,
        current: String,
    },

    #[error("Value for field can't be null.")]
    NotNullViolation {
        collection: Option<String>,
        field: Option<String>,
    },

    #[error("Operation could not be executed: Your current instance of Nexus is out of date.")]
    OutOfDate,

    #[error("Range is invalid or the file's size doesn't match the requested range.")]
    RangeNotSatisfiable {
        start: Option<u64>,
        end: Option<u64>,
    },

    #[error("Value has to be unique.")]
    RecordNotUnique {
        collection: Option<String>,
        field: Option<String>,
        value: Option<String>,
    },

    #[error("Too many requests, retry after {ms}ms.")]
    RequestsExceeded { limit: u64, reset: String, ms: u64 },

    #[error("Route {path} doesn't exist.")]
    RouteNotFound { path: String },

    #[error("Service \"{service}\" is unavailable. {reason}.")]
    ServiceUnavailable { service: String, reason: String },

    #[error("Token expired.")]
    TokenExpired,

    #[error("Received an unexpected response.")]
    UnexpectedResponse,

    #[error("Can't process content. {reason}.")]
    UnprocessableContent { reason: String },

    #[error("Unsupported media type \"{media_type}\" in {location}.")]
    UnsupportedMediaType {
        media_type: String,
        location: String,
    },

    #[error("User suspended.")]
    UserSuspended,

    #[error("Numeric value is out of range.")]
    ValueOutOfRange {
        collection: Option<String>,
        field: Option<String>,
        value: Option<String>,
    },

    #[error("Value is too long.")]
    ValueTooLong {
        collection: Option<String>,
        field: Option<String>,
        value: Option<String>,
    },
}

impl NexusError {
    /// Get the error code for this error
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::ContainsNullValues { .. } => ErrorCode::ContainsNullValues,
            Self::ContentTooLarge => ErrorCode::ContentTooLarge,
            Self::EmailLimitExceeded { .. } => ErrorCode::EmailLimitExceeded,
            Self::Forbidden { .. } => ErrorCode::Forbidden,
            Self::IllegalAssetTransformation { .. } => ErrorCode::IllegalAssetTransformation,
            Self::Internal => ErrorCode::Internal,
            Self::InvalidCredentials => ErrorCode::InvalidCredentials,
            Self::InvalidForeignKey { .. } => ErrorCode::InvalidForeignKey,
            Self::InvalidIp => ErrorCode::InvalidIp,
            Self::InvalidOtp => ErrorCode::InvalidOtp,
            Self::InvalidPayload { .. } => ErrorCode::InvalidPayload,
            Self::InvalidPathParameter { .. } => ErrorCode::InvalidPathParameter,
            Self::InvalidProvider => ErrorCode::InvalidProvider,
            Self::InvalidProviderConfig { .. } => ErrorCode::InvalidProviderConfig,
            Self::InvalidQuery { .. } => ErrorCode::InvalidQuery,
            Self::InvalidToken => ErrorCode::InvalidToken,
            Self::LimitExceeded { .. } => ErrorCode::LimitExceeded,
            Self::MethodNotAllowed { .. } => ErrorCode::MethodNotAllowed,
            Self::NotNullViolation { .. } => ErrorCode::NotNullViolation,
            Self::OutOfDate => ErrorCode::OutOfDate,
            Self::RangeNotSatisfiable { .. } => ErrorCode::RangeNotSatisfiable,
            Self::RecordNotUnique { .. } => ErrorCode::RecordNotUnique,
            Self::RequestsExceeded { .. } => ErrorCode::RequestsExceeded,
            Self::RouteNotFound { .. } => ErrorCode::RouteNotFound,
            Self::ServiceUnavailable { .. } => ErrorCode::ServiceUnavailable,
            Self::TokenExpired => ErrorCode::TokenExpired,
            Self::UnexpectedResponse => ErrorCode::UnexpectedResponse,
            Self::UnprocessableContent { .. } => ErrorCode::UnprocessableContent,
            Self::UnsupportedMediaType { .. } => ErrorCode::UnsupportedMediaType,
            Self::UserSuspended => ErrorCode::UserSuspended,
            Self::ValueOutOfRange { .. } => ErrorCode::ValueOutOfRange,
            Self::ValueTooLong { .. } => ErrorCode::ValueTooLong,
        }
    }

    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::ContainsNullValues { .. } => StatusCode::BAD_REQUEST,
            Self::ContentTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::EmailLimitExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::Forbidden { .. } => StatusCode::FORBIDDEN,
            Self::IllegalAssetTransformation { .. } => StatusCode::BAD_REQUEST,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::InvalidForeignKey { .. } => StatusCode::BAD_REQUEST,
            Self::InvalidIp => StatusCode::UNAUTHORIZED,
            Self::InvalidOtp => StatusCode::UNAUTHORIZED,
            Self::InvalidPayload { .. } => StatusCode::BAD_REQUEST,
            Self::InvalidPathParameter { .. } => StatusCode::BAD_REQUEST,
            Self::InvalidProvider => StatusCode::FORBIDDEN,
            Self::InvalidProviderConfig { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::InvalidQuery { .. } => StatusCode::BAD_REQUEST,
            Self::InvalidToken => StatusCode::FORBIDDEN,
            Self::LimitExceeded { .. } => StatusCode::FORBIDDEN,
            Self::MethodNotAllowed { .. } => StatusCode::METHOD_NOT_ALLOWED,
            Self::NotNullViolation { .. } => StatusCode::BAD_REQUEST,
            Self::OutOfDate => StatusCode::SERVICE_UNAVAILABLE,
            Self::RangeNotSatisfiable { .. } => StatusCode::RANGE_NOT_SATISFIABLE,
            Self::RecordNotUnique { .. } => StatusCode::BAD_REQUEST,
            Self::RequestsExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::RouteNotFound { .. } => StatusCode::NOT_FOUND,
            Self::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::TokenExpired => StatusCode::UNAUTHORIZED,
            Self::UnexpectedResponse => StatusCode::SERVICE_UNAVAILABLE,
            Self::UnprocessableContent { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::UnsupportedMediaType { .. } => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::UserSuspended => StatusCode::UNAUTHORIZED,
            Self::ValueOutOfRange { .. } => StatusCode::BAD_REQUEST,
            Self::ValueTooLong { .. } => StatusCode::BAD_REQUEST,
        }
    }
}

/// Directus-compatible error response format
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub errors: Vec<ErrorDetail>,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub message: String,
    pub extensions: ErrorExtensions,
}

#[derive(Debug, Serialize)]
pub struct ErrorExtensions {
    pub code: ErrorCode,
    #[serde(flatten)]
    pub details: serde_json::Value,
}

impl From<&NexusError> for ErrorResponse {
    fn from(err: &NexusError) -> Self {
        ErrorResponse {
            errors: vec![ErrorDetail {
                message: err.to_string(),
                extensions: ErrorExtensions {
                    code: err.code(),
                    details: serde_json::json!({}),
                },
            }],
        }
    }
}

/// Implement actix-web ResponseError for automatic HTTP error responses
impl actix_web::ResponseError for NexusError {
    fn status_code(&self) -> StatusCode {
        NexusError::status_code(self)
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        let response = ErrorResponse::from(self);
        actix_web::HttpResponse::build(self.status_code()).json(response)
    }
}

/// Helper to create a Forbidden error with optional reason
pub fn forbidden(reason: Option<&str>) -> NexusError {
    NexusError::Forbidden {
        reason: reason.map(|s| s.to_string()),
    }
}

/// Helper to create an InvalidPayload error
pub fn invalid_payload(reason: &str) -> NexusError {
    NexusError::InvalidPayload {
        reason: reason.to_string(),
    }
}

/// Helper to create an InvalidQuery error
pub fn invalid_query(reason: &str) -> NexusError {
    NexusError::InvalidQuery {
        reason: reason.to_string(),
    }
}
