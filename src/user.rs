use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, Json, State},
    http::request::Parts,
    response::Redirect,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use tracing::info_span;

use crate::{AppState, types::ApiResponse};

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

#[derive(Deserialize)]
pub struct LoginRequest {
    user_name: String,
    password: String,
}

#[tracing::instrument(skip_all)]
pub async fn login(
    State(_state): State<AppState>,
    Json(_payload): Json<LoginRequest>,
) -> ApiResponse {
    let u = _state.db.get_user(_payload.user_name).await;
    ApiResponse::Token("EySAuaASioASDh...".to_string())
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    user_name: String,
    password: String,
    name: String,
}

#[tracing::instrument(skip_all)]
pub async fn register(
    State(_state): State<AppState>,
    Json(_payload): Json<RegisterRequest>,
) -> ApiResponse {
    ApiResponse::NoneCreated
}
