use crate::{
    domain::SubscriptionToken,
    errors::{RevokeError, SubcriberDeletionError},
};
use actix_web::{web, HttpResponse, Result};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct RevocationParameters {
    subscription_token: String,
}

#[tracing::instrument(name = "revoke subscription", skip(parameters, db_pool))]
pub async fn revoke(
    parameters: web::Query<RevocationParameters>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, RevokeError> {
    let token = SubscriptionToken::parse(parameters.subscription_token.to_owned())
        .map_err(|e| RevokeError::ValidationError(e))?;
    let id = SubscriptionToken::get_subscriber_id_from_token(&db_pool, token)
        .await
        .context("failed to retrieve confirming subscriber")?;

    match id {
        None => Err(RevokeError::UnauthorizedError(
            "The token received does not correspond to any user id".into(),
        )),
        Some(id) => {
            revoke_subscription(id, &db_pool)
                .await
                .context("failed to revoke subscription")?;
            Ok(HttpResponse::Ok().body("you have correctly unsubscribed"))
        }
    }
}

#[tracing::instrument(name = "update subscriber status", skip(subscriber_id, db_pool))]
pub async fn revoke_subscription(
    subscriber_id: Uuid,
    db_pool: &PgPool,
) -> Result<(), SubcriberDeletionError> {
    let mut sql_transaction = db_pool
        .begin()
        .await
        .map_err(|e| SubcriberDeletionError(e))?;

    sqlx::query!(
        r#"DELETE FROM subscription_tokens WHERE subscriber_id = $1"#,
        subscriber_id
    )
    .execute(&mut sql_transaction)
    .await
    .map_err(|e| SubcriberDeletionError(e))?;

    sqlx::query!(r#"DELETE FROM subscriptions WHERE id = $1"#, subscriber_id)
        .execute(&mut sql_transaction)
        .await
        .map_err(|e| SubcriberDeletionError(e))?;

    sql_transaction
        .commit()
        .await
        .map_err(|e| SubcriberDeletionError(e))?;

    Ok(())
}
