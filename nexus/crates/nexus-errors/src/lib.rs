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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_mapping() {
        assert_eq!(NexusError::Internal.code(), ErrorCode::Internal);
        assert_eq!(NexusError::InvalidCredentials.code(), ErrorCode::InvalidCredentials);
        assert_eq!(NexusError::TokenExpired.code(), ErrorCode::TokenExpired);
        assert_eq!(NexusError::UserSuspended.code(), ErrorCode::UserSuspended);
        assert_eq!(NexusError::InvalidOtp.code(), ErrorCode::InvalidOtp);
        assert_eq!(NexusError::InvalidIp.code(), ErrorCode::InvalidIp);
        assert_eq!(NexusError::OutOfDate.code(), ErrorCode::OutOfDate);
        assert_eq!(NexusError::ContentTooLarge.code(), ErrorCode::ContentTooLarge);
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(NexusError::Internal.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(NexusError::InvalidCredentials.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(NexusError::TokenExpired.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(NexusError::UserSuspended.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(NexusError::ContentTooLarge.status_code(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(forbidden(None).status_code(), StatusCode::FORBIDDEN);
        assert_eq!(invalid_payload("bad data").status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(invalid_query("bad query").status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(
            NexusError::RouteNotFound { path: "/test".to_string() }.status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            NexusError::MethodNotAllowed { allowed: vec!["GET".into()], current: "DELETE".into() }.status_code(),
            StatusCode::METHOD_NOT_ALLOWED
        );
    }

    #[test]
    fn test_error_messages() {
        assert_eq!(NexusError::InvalidCredentials.to_string(), "Invalid user credentials.");
        assert_eq!(forbidden(Some("Admin access required.")).to_string(), "Admin access required.");
        assert_eq!(forbidden(None).to_string(), "You don't have permission to access this.");
        assert_eq!(
            invalid_payload("Missing required field: name").to_string(),
            "Invalid payload. Missing required field: name."
        );
        assert_eq!(
            NexusError::RouteNotFound { path: "/api/v1/test".to_string() }.to_string(),
            "Route /api/v1/test doesn't exist."
        );
        assert_eq!(NexusError::TokenExpired.to_string(), "Token expired.");
    }

    #[test]
    fn test_error_response_format() {
        let err = NexusError::InvalidCredentials;
        let response = ErrorResponse::from(&err);
        assert_eq!(response.errors.len(), 1);
        assert_eq!(response.errors[0].message, "Invalid user credentials.");
        assert_eq!(response.errors[0].extensions.code, ErrorCode::InvalidCredentials);
    }

    #[test]
    fn test_error_response_serialization() {
        let err = NexusError::InvalidCredentials;
        let response = ErrorResponse::from(&err);
        let json = serde_json::to_value(&response).unwrap();
        assert!(json["errors"].is_array());
        assert_eq!(json["errors"][0]["message"], "Invalid user credentials.");
        assert_eq!(json["errors"][0]["extensions"]["code"], "INVALID_CREDENTIALS");
    }

    #[test]
    fn test_error_code_serialization_format() {
        let test_cases = vec![
            (ErrorCode::ContainsNullValues, "CONTAINS_NULL_VALUES"),
            (ErrorCode::ContentTooLarge, "CONTENT_TOO_LARGE"),
            (ErrorCode::Forbidden, "FORBIDDEN"),
            (ErrorCode::Internal, "INTERNAL_SERVER_ERROR"),
            (ErrorCode::InvalidCredentials, "INVALID_CREDENTIALS"),
            (ErrorCode::InvalidPayload, "INVALID_PAYLOAD"),
            (ErrorCode::InvalidQuery, "INVALID_QUERY"),
            (ErrorCode::InvalidToken, "INVALID_TOKEN"),
            (ErrorCode::TokenExpired, "TOKEN_EXPIRED"),
            (ErrorCode::RouteNotFound, "ROUTE_NOT_FOUND"),
            (ErrorCode::ServiceUnavailable, "SERVICE_UNAVAILABLE"),
        ];
        for (code, expected) in test_cases {
            let serialized = serde_json::to_value(&code).unwrap();
            assert_eq!(serialized.as_str().unwrap(), expected, "Failed for {:?}", code);
        }
    }

    #[test]
    fn test_helper_functions() {
        match forbidden(Some("test reason")) {
            NexusError::Forbidden { reason } => assert_eq!(reason.unwrap(), "test reason"),
            _ => panic!("Expected Forbidden"),
        }
        match invalid_payload("bad data") {
            NexusError::InvalidPayload { reason } => assert_eq!(reason, "bad data"),
            _ => panic!("Expected InvalidPayload"),
        }
        match invalid_query("invalid filter") {
            NexusError::InvalidQuery { reason } => assert_eq!(reason, "invalid filter"),
            _ => panic!("Expected InvalidQuery"),
        }
    }

    #[test]
    fn test_error_with_fields() {
        let err = NexusError::ContainsNullValues {
            collection: "users".to_string(),
            field: "email".to_string(),
        };
        assert_eq!(err.code(), ErrorCode::ContainsNullValues);
        assert!(err.to_string().contains("email"));
        assert!(err.to_string().contains("users"));

        let err = NexusError::RecordNotUnique {
            collection: Some("articles".to_string()),
            field: Some("slug".to_string()),
            value: Some("hello-world".to_string()),
        };
        assert_eq!(err.code(), ErrorCode::RecordNotUnique);
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
    }
}
