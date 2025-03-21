use pedronauck_streams_core::types::{Input, Transaction};
use pedronauck_streams_domains::{
    inputs::{types::MockInput, DynInputSubject, InputDbItem},
    transactions::types::MockTransaction,
    MockMsgPayload,
    Subjects,
};
use pedronauck_streams_store::{
    record::{QueryOptions, Record, RecordPacket},
    store::Store,
};
use pedronauck_streams_test::{
    close_db,
    create_random_db_name,
    setup_db,
    setup_store,
};
use pedronauck_streams_types::TxId;
use pretty_assertions::assert_eq;

async fn insert_input(input: Input) -> anyhow::Result<()> {
    let prefix = create_random_db_name();
    let (tx, tx_id) = create_tx(vec![input]);
    let packets = create_packets(&tx, &tx_id, &prefix);
    assert_eq!(packets.len(), 1);

    // Add namespace handling
    let mut store = setup_store::<Input>().await?;
    let packet = packets.first().unwrap().clone();
    store.with_namespace(&prefix);

    let db_item = InputDbItem::try_from(&packet);
    assert!(
        db_item.is_ok(),
        "Failed to convert packet to db item: {:?}",
        packet
    );

    let db_item = db_item.unwrap();
    let db_record = store.insert_record(&db_item).await?;
    assert_eq!(db_record.subject, packet.subject_str());

    close_db(&store.db).await;
    Ok(())
}

fn create_tx(inputs: Vec<Input>) -> (Transaction, TxId) {
    let tx = MockTransaction::script(inputs, vec![], vec![]);
    let tx_id = tx.to_owned().id;
    (tx, tx_id)
}

fn create_packets(
    tx: &Transaction,
    tx_id: &TxId,
    prefix: &str,
) -> Vec<RecordPacket> {
    tx.clone()
        .inputs
        .into_iter()
        .enumerate()
        .map(|(input_index, input)| {
            let subject = DynInputSubject::from((
                &input,
                1.into(),
                tx_id.clone(),
                0,
                input_index as u32,
            ));
            let msg_payload = MockMsgPayload::build(1, prefix);
            let timestamps = msg_payload.timestamp();
            input
                .to_packet(&subject.into(), timestamps)
                .with_namespace(prefix)
        })
        .collect::<Vec<_>>()
}

#[tokio::test]
async fn store_can_record_coin_input() -> anyhow::Result<()> {
    insert_input(MockInput::coin_signed()).await
}

#[tokio::test]
async fn store_can_record_contract_input() -> anyhow::Result<()> {
    insert_input(MockInput::contract()).await
}

#[tokio::test]
async fn store_can_record_message_input() -> anyhow::Result<()> {
    insert_input(MockInput::message_coin_signed()).await
}

#[tokio::test]
async fn find_many_by_subject_with_sql_columns() -> anyhow::Result<()> {
    let prefix = create_random_db_name();
    let mut store = setup_store::<Input>().await?;
    store.with_namespace(&prefix);

    // Create a transaction with all types of inputs
    let (tx, tx_id) = create_tx(vec![
        MockInput::coin_signed(),
        MockInput::contract(),
        MockInput::message_coin_signed(),
    ]);
    let packets = create_packets(&tx, &tx_id, &prefix);
    for packet in packets {
        let payload = packet.subject_payload.clone();
        let subject: Subjects = payload.try_into()?;
        let subject = subject.into();
        let _ = store
            .find_many_by_subject(&subject, QueryOptions::default())
            .await?;
    }

    close_db(&store.db).await;
    Ok(())
}

#[tokio::test]
async fn test_input_subject_to_db_item_conversion() -> anyhow::Result<()> {
    let prefix = create_random_db_name();
    let db = setup_db().await?;
    let mut store = Store::<Input>::new(&db);
    store.with_namespace(&prefix);

    let inputs = vec![
        MockInput::coin_signed(),
        MockInput::contract(),
        MockInput::message_coin_signed(),
    ];

    let (tx, tx_id) = create_tx(inputs);
    let packets = create_packets(&tx, &tx_id, &prefix);

    for packet in packets {
        let payload = packet.subject_payload.clone();
        let subject: Subjects = payload.try_into()?;
        let db_item = InputDbItem::try_from(&packet)?;
        let inserted = store.insert_record(&db_item).await?;

        // Verify common fields
        assert_eq!(db_item.block_height, inserted.block_height);
        assert_eq!(db_item.tx_id, inserted.tx_id);
        assert_eq!(db_item.tx_index, inserted.tx_index);
        assert_eq!(db_item.input_index, inserted.input_index);
        assert_eq!(db_item.subject, inserted.subject);
        assert_eq!(db_item.value, inserted.value);
        assert_eq!(db_item.created_at, inserted.created_at);
        assert!(inserted.published_at.is_after(&db_item.published_at));

        match subject {
            Subjects::InputsCoin(subject) => {
                assert_eq!(db_item.input_type, "coin");
                assert_eq!(
                    db_item.owner_id,
                    Some(subject.owner.unwrap().to_string())
                );
                assert_eq!(
                    db_item.asset_id,
                    Some(subject.asset.unwrap().to_string())
                );
                assert_eq!(db_item.contract_id, None);
                assert_eq!(db_item.sender_address, None);
                assert_eq!(db_item.recipient_address, None);
            }
            Subjects::InputsContract(subject) => {
                assert_eq!(db_item.input_type, "contract");
                assert_eq!(
                    db_item.contract_id,
                    Some(subject.contract.unwrap().to_string())
                );
                assert_eq!(db_item.owner_id, None);
                assert_eq!(db_item.asset_id, None);
                assert_eq!(db_item.sender_address, None);
                assert_eq!(db_item.recipient_address, None);
            }
            Subjects::InputsMessage(subject) => {
                assert_eq!(db_item.input_type, "message");
                assert_eq!(
                    db_item.sender_address,
                    Some(subject.sender.unwrap().to_string())
                );
                assert_eq!(
                    db_item.recipient_address,
                    Some(subject.recipient.unwrap().to_string())
                );
                assert_eq!(db_item.owner_id, None);
                assert_eq!(db_item.asset_id, None);
                assert_eq!(db_item.contract_id, None);
            }
            _ => panic!("Unexpected subject type"),
        }
    }

    close_db(&store.db).await;
    Ok(())
}
