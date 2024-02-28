use crate::{domain::SubscriberEmail, email_client::EmailClient};
use actix_web::{http::header::HeaderMap, web, HttpRequest, HttpResponse, ResponseError};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use reqwest::{
    header::{self, HeaderValue},
    StatusCode,
};
use sqlx::PgPool;
use std::{error::Error, fmt::Formatter};
use uuid::Uuid;

#[derive(serde::Deserialize, Debug)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize, Debug)]
pub struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(body, pool, email_client, request),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
    )]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let subscribers = get_confirmed_subscribers(&pool).await?;
    let credentials = basic_authentication(request.headers())?;
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &pool).await?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await?
            }
            Err(err) => {
                tracing::warn!(
                    "Skipping a confirmed subscriber. {:#?} \
                    Their stored contact details are invalid",
                    err
                )
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

#[derive(Debug)]
struct Credentials {
    username: String,
    password: String,
}

async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<Uuid, PublishError> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        credentials.username,
    )
    .fetch_optional(pool)
    .await
    .map_err(PublishError::GetSubscriberError)?;

    let (expected_password_hash, user_id) = match row {
        Some(row) => (row.password_hash, row.user_id),
        None => {
            return Err(PublishError::AuthError("Unknown Username".to_string()));
        }
    };

    let expected_password_hash = PasswordHash::new(&expected_password_hash)
        .map_err(|err| PublishError::Unexpected(err.to_string()))?;

    let _ = Argon2::default()
        .verify_password(credentials.password.as_bytes(), &expected_password_hash)
        .map_err(|_err| PublishError::AuthError("Invalid password".to_string()))?;

    Ok(user_id)
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, String> {
    let header_value = headers
        .get("Authorization")
        .ok_or("The 'Authorization' header was missing")?
        .to_str()
        .map_err(|_err| "The 'Authorization' header was not a valid UTF8 string.".to_string())?;

    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .ok_or("The authorization scheme was not 'Basic'.")?;
    let decoded_bytes = base64::decode_config(base64encoded_segment, base64::STANDARD)
        .map_err(|_err| "Failed to base64-decode 'Basic' credentials.".to_string())?;

    let decoded_credentials = String::from_utf8(decoded_bytes)
        .map_err(|_err| "The decoded credential string is not valid UTF8.".to_string())?;

    let mut credentials = decoded_credentials.splitn(2, ":");

    let username = credentials
        .next()
        .ok_or_else(|| "A username must be provided in 'Basic' auth.".to_string())?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| "A password must be provided in 'Basic' auth.".to_string())?
        .to_string();

    Ok(Credentials { username, password })
}

async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, String>>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
    SELECT email
    FROM subscriptions
    WHERE status = 'confirmed'
    "#,
    )
    .fetch_all(pool)
    .await?;

    let confirmed_subs = rows
        .into_iter()
        .map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(error),
        })
        .collect();

    Ok(confirmed_subs)
}

fn error_chain_fmt(e: &impl Error, f: &mut Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

pub enum PublishError {
    GetSubscriberError(sqlx::Error),
    SendEmailError(reqwest::Error),
    AuthError(String),
    Unexpected(String),
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublishError::GetSubscriberError(_) => {
                write!(f, "Failed to get subscribers in the database.")
            }
            PublishError::SendEmailError(_) => {
                write!(f, "Failed to send a confirmation email.")
            }
            PublishError::AuthError(e) => write!(f, "{}", e),
            PublishError::Unexpected(e) => write!(f, "{}", e),
        }
    }
}

impl Error for PublishError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PublishError::GetSubscriberError(e) => Some(e),
            PublishError::SendEmailError(e) => Some(e),
            PublishError::AuthError(_) => None,
            PublishError::Unexpected(_) => None,
        }
    }
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl From<sqlx::Error> for PublishError {
    fn from(value: sqlx::Error) -> Self {
        Self::GetSubscriberError(value)
    }
}
impl From<reqwest::Error> for PublishError {
    fn from(value: reqwest::Error) -> Self {
        Self::SendEmailError(value)
    }
}

impl From<String> for PublishError {
    fn from(e: String) -> Self {
        Self::AuthError(e)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::GetSubscriberError(_)
            | PublishError::SendEmailError(_)
            | PublishError::Unexpected(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
        }
    }
}
