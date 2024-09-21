use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName, SubscriptionToken},
    email_client::EmailClient,
    errors::{CheckSubError, StoreTokenError, SubscribeError},
    startup::ApplicationBaseUrl,
};
use actix_web::{web, HttpResponse, Result};
use anyhow::Context;
use chrono::Utc;
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

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
    let revocation_link = &format!(
        "{}/subscriptions/revoke?subscription_token={}",
        base_url,
        subscription_token.as_ref()
    );
    let html_body = format!("Welcome to our mailing list!<br /> Please, click <a href=\"{}\">here</a> to confirm your subscription, you will receive all the updates regarding the development of decisionFlow app!<br /><br />You can unsubscribe at any moment clicking <a href=\"{}\">here</a>", confirmation_link, revocation_link);
    let plain_body = format!(
        "Welcome to our mailing list! Please, visit this link: {} to confirm your subscription, you will receive all the updates regarding the development of decisionFlow app! \r\r You can unsubscribe at any time visiting this link instead: {}",
        confirmation_link, revocation_link
    );

    email_client
        .send_email(
            new_sub.email,
            "Welcome to DecisionFlow!",
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
        .context("Failed to get Postgres connection from the pool")?;

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
        .context("Failed to commit subscription SQL transaction")?;

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
