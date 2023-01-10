extern crate serde;

mod test_utils;

use flexi_logger::LoggerHandle;
use hdbconnect::types::BLob;
use hdbconnect::{Connection, HdbError, HdbResult, HdbValue};
use log::{debug, info};
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use serde_bytes::{ByteBuf, Bytes};
use sha2::{Digest, Sha256};

// cargo test test_032_blobs -- --nocapture
#[test]
fn test_032_blobs() -> HdbResult<()> {
    let mut loghandle = test_utils::init_logger();
    let start = std::time::Instant::now();
    let mut connection = test_utils::get_authenticated_connection()?;

    let (random_bytes, fingerprint) = get_random_bytes();
    test_blobs(&mut loghandle, &mut connection, &random_bytes, &fingerprint)?;
    test_streaming(&mut loghandle, &mut connection, random_bytes, &fingerprint)?;

    test_utils::closing_info(connection, start)
}

const SIZE: usize = 5 * 1024 * 1024;

fn get_random_bytes() -> (Vec<u8>, Vec<u8>) {
    // create random byte data
    let mut raw_data = vec![0; SIZE];
    raw_data.resize(SIZE, 0_u8);
    thread_rng().fill_bytes(&mut raw_data);
    assert_eq!(raw_data.len(), SIZE);

    let mut hasher = Sha256::default();
    hasher.update(&raw_data);
    (raw_data, hasher.finalize().to_vec())
}

fn test_blobs(
    _loghandle: &mut LoggerHandle,
    connection: &mut Connection,
    random_bytes: &[u8],
    fingerprint: &[u8],
) -> HdbResult<()> {
    info!("create a 5MB BLOB in the database, and read it in various ways");
    connection.set_lob_read_length(1_000_000)?;

    connection.multiple_statements_ignore_err(vec!["drop table TEST_BLOBS"]);
    let stmts = vec![
        "\
         create table TEST_BLOBS \
         (desc NVARCHAR(10) not null, bindata BLOB, bindata_NN BLOB NOT NULL)",
    ];
    connection.multiple_statements(stmts)?;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct MyData {
        #[serde(rename = "DESC")]
        desc: String,
        #[serde(rename = "BL1")]
        bytes: ByteBuf, // Vec<u8>,
        #[serde(rename = "BL2")]
        o_bytes: Option<ByteBuf>, // Option<Vec<u8>>,
        #[serde(rename = "BL3")]
        bytes_nn: ByteBuf, // Vec<u8>,
    }

    // insert it into HANA
    let mut insert_stmt =
        connection.prepare("insert into TEST_BLOBS (desc, bindata, bindata_NN) values (?,?,?)")?;
    insert_stmt.add_batch(&("5MB", Bytes::new(random_bytes), Bytes::new(random_bytes)))?;
    insert_stmt.execute_batch()?;

    // and read it back
    let before = connection.get_call_count()?;
    let query = "select desc, bindata as BL1, bindata as BL2 , bindata_NN as BL3 from TEST_BLOBS";
    let resultset = connection.query(query)?;
    let mydata: MyData = resultset.try_into()?;
    info!(
        "reading 2x5MB BLOB with lob-read-length {} required {} roundtrips",
        connection.get_lob_read_length()?,
        connection.get_call_count()? - before
    );

    // verify we get the same bytes back
    assert_eq!(SIZE, mydata.bytes.len());
    let mut hasher = Sha256::default();
    hasher.update(&mydata.bytes);
    assert_eq!(SIZE, mydata.bytes_nn.len());
    let mut hasher = Sha256::default();
    hasher.update(&mydata.bytes_nn);
    let fingerprint2 = hasher.finalize().to_vec();
    assert_eq!(fingerprint, fingerprint2.as_slice());

    let mut hasher = Sha256::default();
    hasher.update(mydata.o_bytes.as_ref().unwrap());
    let fingerprint2 = hasher.finalize().to_vec();
    assert_eq!(fingerprint, fingerprint2.as_slice());

    // try again with small lob-read-length
    connection.set_lob_read_length(10_000)?;
    let before = connection.get_call_count()?;
    let resultset = connection.query(query)?;
    let second: MyData = resultset.try_into()?;
    info!(
        "reading 2x5MB BLOB with lob-read-length {} required {} roundtrips",
        connection.get_lob_read_length()?,
        connection.get_call_count()? - before
    );
    assert_eq!(mydata, second);

    // stream a blob from the database into a sink
    info!("read big blob in streaming fashion");

    connection.set_lob_read_length(500_000)?;

    let query = "select bindata as BL1, bindata as BL2, bindata_NN as BL3 from TEST_BLOBS";
    let mut row = connection.query(query)?.into_single_row()?;
    let mut blob: BLob = row.next_value().unwrap().try_into_blob()?;
    let mut blob2: BLob = row.next_value().unwrap().try_into_blob()?;

    let mut streamed = Vec::<u8>::new();
    std::io::copy(&mut blob2, &mut streamed).map_err(HdbError::LobStreaming)?;
    assert_eq!(random_bytes.len(), streamed.len());
    let mut hasher = Sha256::default();
    hasher.update(&streamed);

    let mut streamed = Vec::<u8>::new();
    std::io::copy(&mut blob, &mut streamed).map_err(HdbError::LobStreaming)?;

    assert_eq!(random_bytes.len(), streamed.len());
    let mut hasher = Sha256::default();
    hasher.update(&streamed);
    let fingerprint4 = hasher.finalize().to_vec();
    assert_eq!(fingerprint, fingerprint4.as_slice());

    debug!("blob.max_buf_len(): {}", blob.max_buf_len());
    // std::io::copy works with 8MB, our buffer remains at about 200_000:
    assert!(blob.max_buf_len() < 510_000);

    info!("read from somewhere within");
    let mut blob: BLob = connection
        .query("select bindata from TEST_BLOBS")?
        .into_single_row()?
        .into_single_value()?
        .try_into_blob()?;
    for i in 1000..1040 {
        let _blob_slice = blob.read_slice(i, 100)?;
    }

    Ok(())
}

fn test_streaming(
    _log_handle: &mut flexi_logger::LoggerHandle,
    connection: &mut Connection,
    random_bytes: Vec<u8>,
    fingerprint: &[u8],
) -> HdbResult<()> {
    info!("write and read big blob in streaming fashion");

    connection.set_auto_commit(true)?;
    connection.dml("delete from TEST_BLOBS")?;

    debug!("write big blob in streaming fashion");

    let mut stmt = connection.prepare("insert into TEST_BLOBS (desc, bindata_NN) values(?, ?)")?;

    // old style lob streaming: autocommit off before, explicit commit after:
    connection.set_auto_commit(false)?;
    let reader = std::sync::Arc::new(std::sync::Mutex::new(std::io::Cursor::new(
        random_bytes.clone(),
    )));
    stmt.execute_row(vec![
        HdbValue::STRING("streaming1".to_string()),
        HdbValue::LOBSTREAM(Some(reader)),
    ])?;
    connection.commit()?;

    // new style lob streaming: with autocommit
    connection.set_auto_commit(true)?;
    let reader = std::sync::Arc::new(std::sync::Mutex::new(std::io::Cursor::new(
        random_bytes.clone(),
    )));
    stmt.execute_row(vec![
        HdbValue::STRING("streaming2".to_string()),
        HdbValue::LOBSTREAM(Some(reader)),
    ])?;

    debug!("read big blob in streaming fashion");
    connection.set_lob_read_length(200_000)?;

    let mut blob = connection
        .query("select bindata_NN from TEST_BLOBS where desc = 'streaming2'")?
        .into_single_row()?
        .into_single_value()?
        .try_into_blob()?;
    let mut buffer = Vec::<u8>::new();
    std::io::copy(&mut blob, &mut buffer).map_err(HdbError::LobStreaming)?;

    assert_eq!(random_bytes.len(), buffer.len());
    let mut hasher = Sha256::default();
    hasher.update(&buffer);
    let fingerprint4 = hasher.finalize().to_vec();
    assert_eq!(fingerprint, fingerprint4.as_slice());
    assert!(
        blob.max_buf_len() < 210_000,
        "blob.max_buf_len() too big: {}",
        blob.max_buf_len()
    );

    connection.set_auto_commit(true)?;
    Ok(())
}
