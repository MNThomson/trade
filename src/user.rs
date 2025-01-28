use argon2::{
    Algorithm, Argon2, Params,
    password_hash::{
        self, PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng,
    },
};
use axum::extract::{Json, State};
use jsonwebtoken::{EncodingKey, encode};
use serde::Deserialize;
use tracing::error;

use crate::{
    AppState,
    auth::{Jwt, SECRET},
    types::{ApiResponse, AppError},
};

static JWT_EXPIRATION_SECS: u64 = 60 * 5;

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
