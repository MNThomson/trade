use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
    response::Redirect,
};
use axum_extra::extract::CookieJar;
use tracing::info_span;

use crate::AppState;

pub const AUTH_COOKIE: &str = "authorization";

pub struct User {
    pub session_token: String,
}

pub struct AuthUser(pub User);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Redirect;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let root_span = tracing::Span::current();
        let _span = info_span!("AuthUser Extractor").entered();

        let auth_token = CookieJar::from_headers(&parts.headers)
            .get(AUTH_COOKIE)
            .ok_or(Redirect::temporary("/login"))?
            .value_trimmed()
            .to_string();

        let _db = AppState::from_ref(state).db;

        root_span.record("user.id", "testuser");

        Ok(AuthUser(User {
            session_token: auth_token,
        }))
    }
}
