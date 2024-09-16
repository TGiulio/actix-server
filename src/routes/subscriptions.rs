use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName, SubscriptionToken},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};
use actix_web::{http::StatusCode, web, HttpResponse, ResponseError, Result};
use anyhow::Context;
use chrono::Utc;
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

fn error_chain_fmt(e: &impl std::error::Error, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    writeln!(f, "{} \n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, " caused by: \n\t {}", cause)?;
        current = cause.source();
    }
    Ok(())
}

pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "a database error was encountered while \
            trying to store a subscription token."
        )
    }
}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

pub struct CheckSubError(sqlx::Error);

impl std::fmt::Display for CheckSubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "a database error was encountered while \
            checking for subscriber existance."
        )
    }
}

impl std::fmt::Debug for CheckSubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::error::Error for CheckSubError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl ResponseError for CheckSubError {}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

#[tracing::instrument(name = "saving subscriber to the database", skip(new_sub, transaction))]
pub async fn insert_subscriber(
    new_sub: &NewSubscriber,
    transaction: &mut sqlx::Transaction<'_, Postgres>,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(r#"INSERT into public.subscriptions (id, email, name, subscribed_at, status) VALUES ($1, $2, $3, $4, 'pending_confirmation')"#,
        subscriber_id,
        new_sub.email.as_ref(),
        new_sub.name.as_ref(),
        Utc::now()
    )
    .execute(transaction)
    .await?;
    Ok(subscriber_id)
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(form.name)?;
        let email = SubscriberEmail::parse(form.email)?;
        Ok(NewSubscriber { name, email })
    }
}

#[tracing::instrument(
    name = "sending confirmation email to the new subscriber",
    skip(email_client, new_sub)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_sub: NewSubscriber,
    base_url: &str,
    subscription_token: &SubscriptionToken,
) -> Result<(), reqwest::Error> {
    let confirmation_link = &format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url,
        subscription_token.as_ref()
    );
    let html_body = format!("Welcome to our mailing list!<br /> Please, click <a href=\"{}\">here</a> to confirm your subscription, you will receive all the updates regarding the development of decisionFlow app!", confirmation_link);
    let plain_body = format!(
        "Welcome to our mailing list! Please, visit this link: {} to confirm your subscription, you will receive all the updates regarding the development of decisionFlow app!",
        confirmation_link
    );

    email_client
        .send_email(
            new_sub.email,
            "Welcome to decisionFlow!",
            &html_body,
            &plain_body,
        )
        .await
}

#[tracing::instrument(
    name = "Adding new subscriber",
    skip(form, db_pool, email_client)
    fields (
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    // checking subscriber existance
    if let Some((existing_sub, token)) = subscriber_existance_check(&form.0.email, &db_pool)
        .await
        .context("Failed to check user existance")?
    {
        send_confirmation_email(&email_client, existing_sub, &base_url.0, &token)
            .await
            .context("Failed to send confirmation email")?;
        return Ok(HttpResponse::Ok().finish());
    }

    // if the subscriber is new
    let new_sub = form.0.try_into().map_err(SubscribeError::ValidationError)?;

    let mut sql_transaction = db_pool
        .begin()
        .await
        .context("Failed to get Postrges connection from the pool")?;

    let subscriber_id = insert_subscriber(&new_sub, &mut sql_transaction)
        .await
        .context("Failed to insert new subscriber")?;

    let subscription_token = SubscriptionToken::new();
    store_token(subscriber_id, &subscription_token, &mut sql_transaction)
        .await
        .context("Failed to store confimration token")?;

    sql_transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction")?;

    send_confirmation_email(&email_client, new_sub, &base_url.0, &subscription_token)
        .await
        .context("Failed to send confirmation email")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "store subscription token",
    skip(subscriber_id, token, transaction)
)]
pub async fn store_token(
    subscriber_id: Uuid,
    token: &SubscriptionToken,
    transaction: &mut sqlx::Transaction<'_, Postgres>,
) -> Result<(), StoreTokenError> {
    sqlx::query!(r#"INSERT into public.subscription_tokens (subscriber_id, subscription_token) VALUES ($1, $2)"#,
        subscriber_id,
        token.as_ref()
     )
    .execute(transaction)
    .await
        .map_err(|e| {
            StoreTokenError(e)
        })?;
    Ok(())
}

async fn subscriber_existance_check(
    email: &str,
    db_pool: &PgPool,
) -> Result<Option<(NewSubscriber, SubscriptionToken)>, CheckSubError> {
    let saved = sqlx::query!(
        r#"SELECT name, email, subscription_token FROM public.subscriptions JOIN public.subscription_tokens ON id = subscriber_id WHERE email = $1"#,
        email
    )
    .fetch_optional(db_pool)
    .await
    .map_err(|e| {
        CheckSubError(e)
    })?;
    match saved {
        None => Ok(None),
        Some(subscriber) => {
            let token =
                SubscriptionToken::parse(subscriber.subscription_token.to_string()).unwrap(); // FIXME: this can be dangerous but it comes from the database so it must have passed this check during the insert operation
            let existing_sub = NewSubscriber {
                name: SubscriberName::parse(subscriber.name).unwrap(), // FIXME: this can be dangerous but it comes from the database so it must have passed this check during the insert operation
                email: SubscriberEmail::parse(subscriber.email).unwrap(), // FIXME: this can be dangerous but it comes from the database so it must have passed this check during the insert operation
            };
            Ok(Some((existing_sub, token)))
        }
    }
}
