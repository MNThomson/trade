use axum::{RequestPartsExt, async_trait, extract::FromRequestParts, http::request::Parts};
use axum_extra::TypedHeader;
use headers::{Header, HeaderName, HeaderValue};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use tracing::{Instrument, info_span};

use crate::types::AppError;

pub static SECRET: &str = "SECRET";

#[derive(Serialize, Deserialize)]
pub struct Jwt {
    pub sub: i64,
    pub exp: u64,
}

pub struct AuthUser(pub i64);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let root_span = tracing::Span::current();
        let s = info_span!("AuthUser Extractor");
        let t = async move {
            let TypedHeader(token) = parts
                .extract::<TypedHeader<TokenHeader>>()
                .await
                .map_err(|_| AppError::AuthTokenNotPresent)?;

            let token_data = decode::<Jwt>(
                &token.0,
                &DecodingKey::from_secret(SECRET.as_bytes()),
                &Validation::default(),
            )
            .map_err(|_| AppError::AuthTokenInvalid)?;
            Ok(token_data.claims.sub)
        }
        .instrument(s)
        .await?;

        root_span.record("user.id", t);

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
        unreachable!("token header is never sent as a response");
    }
}
