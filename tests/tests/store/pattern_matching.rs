use pedronauck_streams_core::{subjects::*, types::Block};
use pedronauck_streams_test::{
    close_db,
    create_multiple_records,
    create_random_db_name,
    insert_records,
    setup_store,
};
use pedronauck_streams_domains::blocks::subjects::BlocksSubject;
use pedronauck_streams_store::record::QueryOptions;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn test_asterisk_subject_string() -> anyhow::Result<()> {
    let prefix = create_random_db_name();
    let mut store = setup_store::<Block>().await?;
    store.with_namespace(&prefix);

    // Create and insert test blocks with different subjects
    let records = create_multiple_records(3, 1, &prefix);
    insert_records(&store, &prefix, &records).await?;

    // Test subject matching
    let subject = BlocksSubject::new().with_height(None).dyn_arc();
    let records = store
        .find_many_by_subject(&subject, QueryOptions::default())
        .await?;
    assert_eq!(records.len(), 3);

    close_db(&store.db).await;
    Ok(())
}

#[tokio::test]
async fn test_nonexistent_subjects() -> anyhow::Result<()> {
    let prefix = create_random_db_name();
    let mut store = setup_store::<Block>().await?;
    store.with_namespace(&prefix);

    // Test finding with a subject that doesn't exist
    let nonexistent_subject =
        BlocksSubject::new().with_height(Some(999.into())).dyn_arc();
    let records = store
        .find_many_by_subject(&nonexistent_subject, QueryOptions::default())
        .await?;
    assert!(records.is_empty());

    close_db(&store.db).await;
    Ok(())
}
