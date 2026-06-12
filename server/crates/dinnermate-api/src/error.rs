use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use dinnermate_core::CoreError;
use serde_json::json;

/// API-level error that renders the `{"error": {"code", "message"}}` envelope.
#[derive(Debug)]
pub enum ApiError {
    /// `X-Dinnermate-User` header absent or not a UUID.
    MissingUser,
    Core(CoreError),
}

impl From<CoreError> for ApiError {
    fn from(err: CoreError) -> Self {
        ApiError::Core(err)
    }
}

impl ApiError {
    fn status_code_message(&self) -> (StatusCode, &'static str, String) {
        match self {
            ApiError::MissingUser => (
                StatusCode::BAD_REQUEST,
                "MISSING_USER",
                "missing or invalid X-Dinnermate-User header".to_string(),
            ),
            ApiError::Core(err) => {
                let (status, code) = match err {
                    CoreError::RoomNotFound => (StatusCode::NOT_FOUND, "ROOM_NOT_FOUND"),
                    CoreError::ListNotFound => (StatusCode::NOT_FOUND, "LIST_NOT_FOUND"),
                    CoreError::NotInRoom => (StatusCode::FORBIDDEN, "NOT_IN_ROOM"),
                    CoreError::AlreadySwiped => (StatusCode::CONFLICT, "ALREADY_SWIPED"),
                    CoreError::UnknownRestaurant => {
                        (StatusCode::UNPROCESSABLE_ENTITY, "UNKNOWN_RESTAURANT")
                    }
                    CoreError::NotListMember => (StatusCode::FORBIDDEN, "NOT_LIST_MEMBER"),
                    CoreError::OwnerCannotLeave => {
                        (StatusCode::UNPROCESSABLE_ENTITY, "OWNER_CANNOT_LEAVE")
                    }
                    CoreError::InvalidParams(_) => {
                        (StatusCode::UNPROCESSABLE_ENTITY, "INVALID_PARAMS")
                    }
                    CoreError::Provider(_) => (StatusCode::BAD_GATEWAY, "PROVIDER_UNAVAILABLE"),
                    CoreError::Repo(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL"),
                };
                // Repo errors may carry database detail; never leak it to clients.
                let message = if status == StatusCode::INTERNAL_SERVER_ERROR {
                    "internal server error".to_string()
                } else {
                    err.to_string()
                };
                (status, code, message)
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = self.status_code_message();
        if status.is_server_error() {
            tracing::error!(error = ?self, "request failed");
        }
        let body = json!({"error": {"code": code, "message": message}});
        (status, Json(body)).into_response()
    }
}
