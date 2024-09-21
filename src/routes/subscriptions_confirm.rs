use crate::{domain::SubscriptionToken, errors::ConfirmError};
use actix_web::{web, HttpResponse, Result};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct ConfirmationParameters {
    subscription_token: String,
}

#[tracing::instrument(name = "confirm pending subscriber", skip(parameters, db_pool))]
pub async fn confirm(
    parameters: web::Query<ConfirmationParameters>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, ConfirmError> {
    let token = SubscriptionToken::parse(parameters.subscription_token.to_owned())
        .map_err(|e| ConfirmError::ValidationError(e))?;
    let id = SubscriptionToken::get_subscriber_id_from_token(&db_pool, token)
        .await
        .context("failed to retrieve confirming subscriber")?;

    match id {
        None => Err(ConfirmError::UnauthorizedError(
            "The token received does not correspond to any user id".into(),
        )),
        Some(id) => {
            confirm_subscriber(id, &db_pool)
                .await
                .context("failed to confirm subscriber")?;
            Ok(HttpResponse::Ok().body("Grazie per aver confermato!"))
        }
    }
}

#[tracing::instrument(name = "update subscriber status", skip(subscriber_id, db_pool))]
pub async fn confirm_subscriber(subscriber_id: Uuid, db_pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id
    )
    .execute(db_pool)
    .await?;
    Ok(())
}
