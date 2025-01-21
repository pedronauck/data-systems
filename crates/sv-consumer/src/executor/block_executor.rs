use std::sync::Arc;

use fuel_message_broker::{Message, MessageBroker};
use fuel_streams_core::{
    types::{Block, Transaction},
    FuelStreams,
};
use fuel_streams_domains::MsgPayload;
use fuel_streams_store::{
    db::Db,
    record::{DataEncoder, PacketBuilder, RecordPacket},
};
use fuel_web_utils::shutdown::shutdown_broker_with_timeout;
use futures::StreamExt;
use tokio::{
    sync::Semaphore,
    task::{JoinError, JoinSet},
};
use tokio_util::sync::CancellationToken;

use super::{
    block_stats::BlockStats,
    process_store::handle_store_insertions,
    process_stream::handle_stream_publishes,
};
use crate::{errors::ConsumerError, FuelStores};

const MAX_CONCURRENT_TASKS: usize = 32;
const BATCH_SIZE: usize = 100;

#[derive(Debug)]
enum ProcessResult {
    Store(Result<BlockStats, ConsumerError>),
    Stream(Result<BlockStats, ConsumerError>),
}

pub struct BlockExecutor {
    db: Arc<Db>,
    message_broker: Arc<dyn MessageBroker>,
    fuel_streams: Arc<FuelStreams>,
    fuel_stores: Arc<FuelStores>,
    semaphore: Arc<Semaphore>,
    max_tasks: usize,
}

impl BlockExecutor {
    pub fn new(
        db: Arc<Db>,
        message_broker: &Arc<dyn MessageBroker>,
        fuel_streams: &Arc<FuelStreams>,
    ) -> Self {
        let pool_size = db.pool.size() as usize;
        let max_tasks = pool_size.saturating_sub(5).min(MAX_CONCURRENT_TASKS);
        let semaphore = Arc::new(Semaphore::new(max_tasks));
        let fuel_stores = FuelStores::new(&db).arc();
        Self {
            db,
            semaphore,
            message_broker: message_broker.clone(),
            fuel_streams: fuel_streams.clone(),
            fuel_stores: fuel_stores.clone(),
            max_tasks,
        }
    }

    pub async fn start(
        &self,
        token: &CancellationToken,
    ) -> Result<(), ConsumerError> {
        let mut join_set = JoinSet::new();
        tracing::info!(
            "Starting consumer with max concurrent tasks: {}",
            self.max_tasks
        );
        while !token.is_cancelled() {
            tokio::select! {
                msg_result = self.message_broker.receive_blocks_stream(BATCH_SIZE) => {
                    let mut messages = msg_result?;
                    while let Some(msg) = messages.next().await {
                        let msg = msg?;
                        self.spawn_processing_tasks(
                            msg,
                            &mut join_set,
                        )
                        .await?;
                    }
                }
                Some(result) = join_set.join_next() => {
                    Self::handle_task_result(result).await?;
                }
            }
        }

        // Wait for all tasks to finish
        while let Some(result) = join_set.join_next().await {
            Self::handle_task_result(result).await?;
        }

        tracing::info!("Stopping actix server ...");
        shutdown_broker_with_timeout(&self.message_broker).await;
        Ok(())
    }

    async fn spawn_processing_tasks(
        &self,
        msg: Box<dyn Message>,
        join_set: &mut JoinSet<Result<ProcessResult, ConsumerError>>,
    ) -> Result<(), ConsumerError> {
        let db = self.db.clone();
        let semaphore = self.semaphore.clone();
        let fuel_streams = self.fuel_streams.clone();
        let fuel_stores = self.fuel_stores.clone();
        let payload = msg.payload();
        let msg_payload = MsgPayload::decode(&payload).await?.arc();
        let packets = Self::build_packets(&msg_payload);

        join_set.spawn({
            let semaphore = semaphore.clone();
            let packets = packets.clone();
            let msg_payload = msg_payload.clone();
            let db = db.clone();
            let fuel_stores = fuel_stores.clone();
            async move {
                let _permit = semaphore.acquire().await?;
                let result = handle_store_insertions(
                    &db,
                    &fuel_stores,
                    &packets,
                    &msg_payload,
                )
                .await;
                Ok::<_, ConsumerError>(ProcessResult::Store(result))
            }
        });

        join_set.spawn({
            let semaphore = semaphore.clone();
            let packets = packets.clone();
            let msg_payload = msg_payload.clone();
            let fuel_streams = fuel_streams.clone();
            async move {
                let _permit = semaphore.acquire_owned().await?;
                let result = handle_stream_publishes(
                    &fuel_streams,
                    &packets,
                    &msg_payload,
                )
                .await;
                Ok(ProcessResult::Stream(result))
            }
        });

        msg.ack().await.map_err(|e| {
            tracing::error!("Failed to ack message: {:?}", e);
            ConsumerError::MessageBrokerClient(e)
        })?;

        Ok(())
    }

    async fn handle_task_result(
        result: Result<Result<ProcessResult, ConsumerError>, JoinError>,
    ) -> Result<(), ConsumerError> {
        match result {
            Ok(Ok(ProcessResult::Store(store_result))) => {
                let store_stats = store_result?;
                match &store_stats.error {
                    Some(error) => store_stats.log_error(error),
                    None => store_stats.log_success(),
                }
            }
            Ok(Ok(ProcessResult::Stream(stream_result))) => {
                let stream_stats = stream_result?;
                match &stream_stats.error {
                    Some(error) => stream_stats.log_error(error),
                    None => stream_stats.log_success(),
                }
            }
            Ok(Err(e)) => tracing::error!("Task error: {}", e),
            Err(e) => tracing::error!("Task panicked: {}", e),
        }
        Ok(())
    }

    fn build_packets(msg_payload: &MsgPayload) -> Arc<Vec<RecordPacket>> {
        let block_packets = Block::build_packets(msg_payload);
        let tx_packets = Transaction::build_packets(msg_payload);
        let packets = block_packets
            .into_iter()
            .chain(tx_packets)
            .collect::<Vec<_>>();
        Arc::new(packets)
    }
}
