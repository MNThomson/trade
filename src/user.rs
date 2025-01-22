use axum::{
    RequestPartsExt, async_trait,
    extract::{FromRequestParts, Json, State},
    http::request::Parts,
};
use axum_extra::TypedHeader;
use headers::{Header, HeaderName, HeaderValue};
use http::StatusCode;
use jsonwebtoken::{DecodingKey, EncodingKey, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use tracing::{Instrument, info_span};

use crate::{AppState, types::ApiResponse};

static SECRET: &str = "SECRET";
static JWT_EXPIRATION_SECS: u64 = 60 * 5;

#[derive(Serialize, Deserialize)]
pub struct Jwt {
    pub sub: String,
    pub exp: u64,
}

pub struct AuthUser(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = ApiResponse;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let root_span = tracing::Span::current();
        let s = info_span!("AuthUser Extractor");
        let t = async move {
            let TypedHeader(token) = parts
                .extract::<TypedHeader<TokenHeader>>()
                .await
                .map_err(|_| ApiResponse::Error(StatusCode::UNAUTHORIZED, "No Token"))?;

            let token_data = decode::<Jwt>(
                &token.0,
                &DecodingKey::from_secret(SECRET.as_bytes()),
                &Validation::default(),
            )
            .map_err(|_| ApiResponse::Error(StatusCode::UNAUTHORIZED, "Invalid Token"))?;
            Ok(token_data.claims.sub)
        }
        .instrument(s)
        .await?;

        root_span.record("user.id", &t);

        Ok(AuthUser(t))
    }
}

static TOKEN_HEADER: HeaderName = HeaderName::from_static("token");
struct TokenHeader(String);
impl Header for TokenHeader {
    fn name() -> &'static HeaderName {
        &TOKEN_HEADER
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(headers::Error::invalid)?;
        Ok(TokenHeader(
            value
                .to_str()
                .map_err(|_| headers::Error::invalid())?
                .to_string(),
        ))
    }

    fn encode<E>(&self, _values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        unreachable!();
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
) -> Result<ApiResponse, ApiResponse> {
    //let u = _state.db.get_user(_payload.user_name).await;
    let token = encode(
        &jsonwebtoken::Header::default(),
        &Jwt {
            sub: "0".to_owned(),
            exp: std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .map_err(|_| ApiResponse::Error(StatusCode::INTERNAL_SERVER_ERROR, ""))?
                .as_secs()
                + JWT_EXPIRATION_SECS,
        },
        &EncodingKey::from_secret(SECRET.as_bytes()),
    )
    .map_err(|_| ApiResponse::Error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create token"))?;

    Ok(ApiResponse::Token(token))
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
