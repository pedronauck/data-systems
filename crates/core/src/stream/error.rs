use async_nats::SubscribeError;
use pedronauck_message_broker::MessageBrokerError;
use pedronauck_streams_store::{
    db::{DbError, SqlxError},
    record::{RecordEntityError, RecordPacketError},
    store::StoreError,
};
use pedronauck_web_utils::api_key::ApiKeyError;

use crate::{server::DeliverPolicyError, types::StreamResponseError};

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error(transparent)]
    Db(#[from] DbError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Subscribe(#[from] SubscribeError),
    #[error(transparent)]
    DeliverPolicy(#[from] DeliverPolicyError),
    #[error(transparent)]
    MessageBrokerClient(#[from] MessageBrokerError),
    #[error(transparent)]
    RecordPacket(#[from] RecordPacketError),
    #[error(transparent)]
    Sqlx(#[from] SqlxError),
    #[error(transparent)]
    StreamResponse(#[from] StreamResponseError),
    #[error(transparent)]
    RecordEntity(#[from] RecordEntityError),
    #[error(transparent)]
    ApiKey(#[from] ApiKeyError),
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
}
