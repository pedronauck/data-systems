use std::sync::Arc;

use fuel_streams_domains::{
    blocks::BlockDbItem,
    inputs::InputDbItem,
    outputs::OutputDbItem,
    receipts::ReceiptDbItem,
    transactions::TransactionDbItem,
    utxos::UtxoDbItem,
};
use fuel_streams_store::{
    db::{DbError, DbItem},
    record::{
        DataEncoder,
        EncoderError,
        RecordEntity,
        RecordEntityError,
        RecordPacket,
        RecordPacketError,
        RecordPointer,
    },
};
use fuel_web_utils::server::api::API_VERSION;
use serde::{Deserialize, Serialize};

use crate::types::*;

#[derive(thiserror::Error, Debug)]
pub enum MessagePayloadError {
    #[error("Invalid data: {0}")]
    InvalidData(String),
    #[error(transparent)]
    RecordEntity(#[from] RecordEntityError),
    #[error(transparent)]
    Decode(#[from] DbError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessagePayload {
    Block(Arc<Block>),
    Input(Arc<Input>),
    Output(Arc<Output>),
    Transaction(Arc<Transaction>),
    Receipt(Arc<Receipt>),
    Utxo(Arc<Utxo>),
}

impl MessagePayload {
    pub fn new(
        subject_id: &str,
        value: &[u8],
    ) -> Result<Self, MessagePayloadError> {
        let record_entity = RecordEntity::from_subject_id(subject_id)?;
        match record_entity {
            RecordEntity::Block => {
                Ok(MessagePayload::Block(Arc::new(Block::decode_json(value)?)))
            }
            RecordEntity::Input => {
                Ok(MessagePayload::Input(Arc::new(Input::decode_json(value)?)))
            }
            RecordEntity::Output => Ok(MessagePayload::Output(Arc::new(
                Output::decode_json(value)?,
            ))),
            RecordEntity::Transaction => Ok(MessagePayload::Transaction(
                Arc::new(Transaction::decode_json(value)?),
            )),
            RecordEntity::Receipt => Ok(MessagePayload::Receipt(Arc::new(
                Receipt::decode_json(value)?,
            ))),
            RecordEntity::Utxo => {
                Ok(MessagePayload::Utxo(Arc::new(Utxo::decode_json(value)?)))
            }
        }
    }

    pub fn as_block(&self) -> Result<Arc<Block>, MessagePayloadError> {
        match self {
            MessagePayload::Block(block) => Ok(block.clone()),
            _ => Err(MessagePayloadError::InvalidData("block".to_string())),
        }
    }

    pub fn as_input(&self) -> Result<Arc<Input>, MessagePayloadError> {
        match self {
            MessagePayload::Input(input) => Ok(input.clone()),
            _ => Err(MessagePayloadError::InvalidData("input".to_string())),
        }
    }

    pub fn as_output(&self) -> Result<Arc<Output>, MessagePayloadError> {
        match self {
            MessagePayload::Output(output) => Ok(output.clone()),
            _ => Err(MessagePayloadError::InvalidData("output".to_string())),
        }
    }

    pub fn as_transaction(
        &self,
    ) -> Result<Arc<Transaction>, MessagePayloadError> {
        match self {
            MessagePayload::Transaction(transaction) => Ok(transaction.clone()),
            _ => {
                Err(MessagePayloadError::InvalidData("transaction".to_string()))
            }
        }
    }

    pub fn as_receipt(&self) -> Result<Arc<Receipt>, MessagePayloadError> {
        match self {
            MessagePayload::Receipt(receipt) => Ok(receipt.clone()),
            _ => Err(MessagePayloadError::InvalidData("receipt".to_string())),
        }
    }

    pub fn as_utxo(&self) -> Result<Arc<Utxo>, MessagePayloadError> {
        match self {
            MessagePayload::Utxo(utxo) => Ok(utxo.clone()),
            _ => Err(MessagePayloadError::InvalidData("utxo".to_string())),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum StreamResponseError {
    #[error(transparent)]
    Encoder(#[from] EncoderError),
    #[error(transparent)]
    MessagePayload(#[from] MessagePayloadError),
    #[error(transparent)]
    RecordEntity(#[from] RecordEntityError),
    #[error(transparent)]
    RecordPacket(#[from] RecordPacketError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResponse {
    pub version: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub subject: String,
    pub pointer: RecordPointer,
    pub payload: MessagePayload,
}

impl StreamResponse {
    pub fn new(
        subject: String,
        subject_id: String,
        value: &[u8],
        pointer: RecordPointer,
    ) -> Result<Self, StreamResponseError> {
        let payload = MessagePayload::new(&subject_id, value)?;
        Ok(Self {
            ty: subject_id,
            version: API_VERSION.to_string(),
            subject,
            payload,
            pointer,
        })
    }
}

impl DataEncoder for StreamResponse {
    type Err = StreamResponseError;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ServerResponse {
    Subscribed(Subscription),
    Unsubscribed(Subscription),
    Response(StreamResponse),
    Error(String),
}

impl<T: DbItem + Into<RecordPointer>> TryFrom<(String, T)> for StreamResponse {
    type Error = StreamResponseError;
    fn try_from((subject_id, item): (String, T)) -> Result<Self, Self::Error> {
        let pointer: RecordPointer = item.to_owned().into();
        StreamResponse::new(
            item.subject_str(),
            subject_id,
            item.encoded_value(),
            pointer,
        )
    }
}

impl TryFrom<&RecordPacket> for StreamResponse {
    type Error = StreamResponseError;
    fn try_from(packet: &RecordPacket) -> Result<Self, Self::Error> {
        let subject_id = packet.subject_id();
        let entity = RecordEntity::from_subject_id(&subject_id)?;
        match entity {
            RecordEntity::Block => {
                let db_item = BlockDbItem::try_from(packet)?;
                StreamResponse::try_from((subject_id, db_item))
            }
            RecordEntity::Transaction => {
                let db_item = TransactionDbItem::try_from(packet)?;
                StreamResponse::try_from((subject_id, db_item))
            }
            RecordEntity::Input => {
                let db_item = InputDbItem::try_from(packet)?;
                StreamResponse::try_from((subject_id, db_item))
            }
            RecordEntity::Output => {
                let db_item = OutputDbItem::try_from(packet)?;
                StreamResponse::try_from((subject_id, db_item))
            }
            RecordEntity::Receipt => {
                let db_item = ReceiptDbItem::try_from(packet)?;
                StreamResponse::try_from((subject_id, db_item))
            }
            RecordEntity::Utxo => {
                let db_item = UtxoDbItem::try_from(packet)?;
                StreamResponse::try_from((subject_id, db_item))
            }
        }
    }
}
