use std::borrow::Cow;
use std::collections::HashMap;

use axum::http::{header::WWW_AUTHENTICATE, HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use sqlx::error::DatabaseError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Return `404 Not Found`
    #[error("request path not found")]
    NotFound,

    // would love something like #[error("authorization required", status = StatusCode::Unauthorized)]
    /// Return `422 Unprocessable Entity`
    #[error("")]
    UnprocessableEntity,

    /// Return `400 Bad Request`
    #[error("error in the request body")]
    BadRequest {
        errors: HashMap<Cow<'static, str>, Vec<Cow<'static, str>>>,
    },

    /// Return `403 Forbidden`
    #[error("user may not have access rights to the content")]
    Forbidden,

    /// Return `401 Unauthorized`
    #[error("authorization required")]
    Unauthorized,

    /// Automatically return `500 Internal Server Error` on a `sqlx::Error`
    ///
    /// Via the generated `From<sqlx::Error> for Error` impl,
    /// this allows using `?` on the database calls in handler functions without a manual mapping
    /// step.
    ///
    /// The actual error message isn't returned to the client for security reasons.
    /// It should be logged instead
    ///
    /// Note that could also contain database constraint errors, which should usually
    /// be transformed into client errors (e.g. `422 Unprocessable Entity` or `409 Conflict`)
    #[error("an error occurred with the database")]
    Sqlx(#[from] sqlx::Error),

    /// Return `500 Internal Server Error`
    #[error("an internal server error has occurred")]
    Anyhow(#[from] anyhow::Error),
}

impl Error {
    pub fn bad_request<K, V>(errors: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<Cow<'static, str>>,
        V: Into<Cow<'static, str>>,
    {
        let mut error_map = HashMap::new();

        for (key, val) in errors {
            error_map
                .entry(key.into())
                .or_insert_with(Vec::new)
                .push(val.into());
        }

        Self::BadRequest { errors: error_map }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::UnprocessableEntity => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::BadRequest { .. } => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn title(&self) -> String {
        match self {
            Self::Unauthorized => "Unauthorized",
            Self::BadRequest { .. } => "Bad Request",
            Self::UnprocessableEntity => "Unprocessable Entity",
            Self::Forbidden => "Forbidden",
            Self::NotFound => "Not Found",
            _ => "Internal Server Error",
        }
        .to_string()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ErrorBody {
    title: String,
    status: u16,
    message: String,
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let e = ErrorBody::from(self);
        let status = StatusCode::from_u16(e.status).unwrap();
        let header: HeaderMap = match status {
            StatusCode::UNAUTHORIZED => [(WWW_AUTHENTICATE, HeaderValue::from_static("Token"))]
                .into_iter()
                .collect(),
            _ => HeaderMap::new(),
        };

        (status, header, Json(e)).into_response()
    }
}

impl From<Error> for ErrorBody {
    fn from(error: Error) -> Self {
        Self {
            title: error.title(),
            message: error.to_string(),
            status: error.status_code().as_u16(),
        }
    }
}

/// A little helper trait for more easily converting database constraint errors into API errors.
///
/// ```rust,ignore
/// let user_id = sqlx::query_scalar!(
///     r#"insert into "user" (username, email, password_hash) values ($1, $2, $3) returning user_id"#,
///     username,
///     email,
///     password_hash
/// )
///     .fetch_one(&ctxt.db)
///     .await
///     .on_constraint("user_username_key", |_| Error::unprocessable_entity([("username", "already taken")]))?;
/// ```
///
/// Something like this would ideally live in a `sqlx-axum` crate if it made sense to author one,
/// however its definition is tied pretty intimately to the `Error` type, which is itself
/// tied directly to application semantics.
///
/// To actually make this work in a generic context would make it quite a bit more complex,
/// as you'd need an intermediate error type to represent either a mapped or an unmapped error,
/// and even then it's not clear how to handle `?` in the unmapped case without more boilerplate.
pub trait ResultExt<T> {
    /// If `self` contains a SQLx database constraint error with the given name,
    /// transform the error.
    ///
    /// Otherwise, the result is passed through unchanged.
    fn on_constraint(
        self,
        name: &str,
        f: impl FnOnce(Box<dyn DatabaseError>) -> Error,
    ) -> Result<T>;
}

impl<T, E> ResultExt<T> for std::result::Result<T, E>
where
    E: Into<Error>,
{
    fn on_constraint(
        self,
        name: &str,
        map_err: impl FnOnce(Box<dyn DatabaseError>) -> Error,
    ) -> Result<T> {
        self.map_err(|e| match e.into() {
            Error::Sqlx(sqlx::Error::Database(dbe)) if dbe.constraint() == Some(name) => {
                map_err(dbe)
            }
            e => e,
        })
    }
}
