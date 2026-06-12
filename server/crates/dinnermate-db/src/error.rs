use dinnermate_core::RepoError;

const UNIQUE_VIOLATION: &str = "23505";

/// Orphan rules prevent `impl From<sqlx::Error> for RepoError` here
/// (both types are foreign), so mapping is a plain function.
pub(crate) fn into_repo_error(err: sqlx::Error) -> RepoError {
    match &err {
        sqlx::Error::RowNotFound => RepoError::NotFound,
        sqlx::Error::Database(db) if db.code().as_deref() == Some(UNIQUE_VIOLATION) => {
            RepoError::Conflict
        }
        _ => RepoError::Database(err.to_string()),
    }
}
