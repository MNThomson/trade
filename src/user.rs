use argon2::{
    Algorithm, Argon2, Params,
    password_hash::{
        self, PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng,
    },
};
use axum::{
    RequestPartsExt, async_trait,
    extract::{FromRequestParts, Json, State},
    http::request::Parts,
};
use axum_extra::TypedHeader;
use headers::{Header, HeaderName, HeaderValue};
use jsonwebtoken::{DecodingKey, EncodingKey, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use tracing::{Instrument, error, info_span};

use crate::{
    AppState,
    types::{ApiResponse, AppError},
};

static SECRET: &str = "SECRET";
static JWT_EXPIRATION_SECS: u64 = 60 * 5;

#[derive(Serialize, Deserialize)]
pub struct Jwt {
    pub sub: i64,
    pub exp: u64,
}

pub struct AuthUser(pub String);

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

        Ok(AuthUser(t.to_string()))
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

#[derive(Deserialize)]
pub struct LoginRequest {
    user_name: String,
    password: String,
}

#[tracing::instrument(skip_all)]
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<ApiResponse, AppError> {
    let u = state.db.get_user(body.user_name).await?;

    hasher()
        .verify_password(
            body.password.as_bytes(),
            &PasswordHash::new(&u.password).expect("stored password hash to be valid"),
        )
        .map_err(|e| {
            if matches!(e, password_hash::Error::Password) {
                return AppError::PasswordInvalid;
            }
            error!("verifying password failed: {}", e);
            AppError::InternalServerError
        })?;

    let claims = Jwt {
        sub: u.user_id,
        exp: std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map_err(|e| {
                error!("expected to get system time: {}", e);
                AppError::InternalServerError
            })?
            .as_secs()
            + JWT_EXPIRATION_SECS,
    };

    let token = encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &EncodingKey::from_secret(SECRET.as_bytes()),
    )
    .map_err(|e| {
        error!("couldn't encode token: {}", e);
        AppError::InternalServerError
    })?;

    Ok(ApiResponse::Token(token))
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    user_name: String,
    password: String,
    #[allow(unused)]
    name: String,
}

#[tracing::instrument(skip_all)]
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<ApiResponse, AppError> {
    let password_hash = hasher()
        .hash_password(body.password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map_err(|e| {
            error!("cannot hash password :{}", e);
            AppError::InternalServerError
        })?
        .to_string();

    state.db.create_user(body.user_name, password_hash).await?;

    Ok(ApiResponse::NoneCreated)
}

pub fn hasher() -> Argon2<'static> {
    Argon2::new(
        Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::new(1024, 1, 1, Some(Params::DEFAULT_OUTPUT_LEN)).expect("correct Argon2 params"),
    )
}
