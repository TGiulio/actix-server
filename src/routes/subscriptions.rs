use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::{self, EmailClient},
    startup::ApplicationBaseUrl,
};
use actix_web::{web, HttpResponse, Responder, Result};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
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
    .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
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
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = &format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let html_body = format!("Welcome to our newsletter!<br /> Please, click <a href=\"{}\">here</a> to confirm your subscription.", confirmation_link);
    let plain_body = format!(
        "Welcome to our newsletter! Please, visit this link: {} to confirm your subscription.",
        confirmation_link
    );

    email_client
        .send_email(new_sub.email, "Welcome!", &html_body, &plain_body)
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
) -> impl Responder {
    let new_sub = match form.0.try_into() {
        Ok(new_sub) => new_sub,
        Err(_) => return HttpResponse::BadRequest(),
    };
    let mut sql_transaction = match db_pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return HttpResponse::InternalServerError(),
    };

    let subscriber_id = match insert_subscriber(&new_sub, &mut sql_transaction).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return HttpResponse::InternalServerError(),
    };

    let subscription_token = generate_subscription_token();
    if store_token(subscriber_id, &subscription_token, &mut sql_transaction)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError();
    }

    if sql_transaction.commit().await.is_err() {
        return HttpResponse::InternalServerError();
    }

    if send_confirmation_email(&email_client, new_sub, &base_url.0, &subscription_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError();
    }

    HttpResponse::Ok()
}

#[tracing::instrument(
    name = "store subscription token",
    skip(subscriber_id, token, transaction)
)]
pub async fn store_token(
    subscriber_id: Uuid,
    token: &str,
    transaction: &mut sqlx::Transaction<'_, Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(r#"INSERT into public.subscription_tokens (subscriber_id, subscription_token) VALUES ($1, $2)"#,
        subscriber_id,
        token
     )
    .execute(transaction)
    .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
    Ok(())
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
