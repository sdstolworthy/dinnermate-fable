use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

use crate::error::ApiError;

pub const USER_HEADER: &str = "x-dinnermate-user";

/// Caller identity from the `X-Dinnermate-User` header.
#[derive(Debug, Clone, Copy)]
pub struct UserId(pub Uuid);

impl<S: Send + Sync> FromRequestParts<S> for UserId {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .headers
            .get(USER_HEADER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| Uuid::parse_str(value).ok())
            .map(UserId)
            .ok_or(ApiError::MissingUser)
    }
}
