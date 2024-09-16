use crate::errors::ConfirmError;
use actix_web::{web, HttpResponse, Result};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "confirm pending subscriber", skip(parameters, db_pool))]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, ConfirmError> {
    let id = get_subscriber_id_from_token(&db_pool, &parameters.subscription_token)
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
            Ok(HttpResponse::Ok().finish())
        }
    }
}

#[tracing::instrument(name = "get subscriber id from token", skip(token, db_pool))]
pub async fn get_subscriber_id_from_token(
    db_pool: &PgPool,
    token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id from subscription_tokens WHERE subscription_token = $1"#,
        token
    )
    .fetch_optional(db_pool)
    .await?;
    Ok(result.map(|r| r.subscriber_id))
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
