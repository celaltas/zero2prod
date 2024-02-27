use crate::{domain::SubscriberEmail, email_client::EmailClient};
use actix_web::{web, HttpResponse, ResponseError};
use reqwest::StatusCode;
use sqlx::PgPool;
use std::{error::Error, fmt::Formatter};

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

pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, PublishError> {
    let subscribers = get_confirmed_subscribers(&pool).await?;
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
        }
    }
}

impl Error for PublishError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PublishError::GetSubscriberError(e) => Some(e),
            PublishError::SendEmailError(e) => Some(e),
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

impl ResponseError for PublishError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublishError::GetSubscriberError(_) | PublishError::SendEmailError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}
