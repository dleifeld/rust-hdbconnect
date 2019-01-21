mod test_utils;

use flexi_logger::ReconfigurationHandle;
use hdbconnect::{Connection, HdbValue, HdbResult};
use log::info;
use serde_bytes::Bytes;

#[test] // cargo test --test test_040_datatypes_a -- --nocapture
pub fn test_040_datatypes_a() -> HdbResult<()> {
    let mut log_handle = test_utils::init_logger();
    let mut connection = test_utils::get_authenticated_connection()?;

    prepare(&mut log_handle, &mut connection)?;
    write(&mut log_handle, &mut connection)?;
    read(&mut log_handle, &mut connection)?;

    info!("{} calls to DB were executed", connection.get_call_count()?);

    Ok(())
}

fn prepare(
    _log_handle: &mut ReconfigurationHandle,
    connection: &mut Connection,
) -> HdbResult<()> {
    info!("prepare the db table");
    connection.multiple_statements_ignore_err(vec!["drop table TEST_TYPES_A"]);
    connection.multiple_statements(vec![
        "create table TEST_TYPES_A ( \
            id BIGINT GENERATED BY DEFAULT AS IDENTITY primary key , \
            FIELD_TINYINT TINYINT, \
            FIELD_SMALLINT SMALLINT, \
            FIELD_INT INT, \
            FIELD_BIGINT BIGINT, \
            FIELD_SMALLDECIMAL SMALLDECIMAL, \
            FIELD_DECIMAL DECIMAL, \
            FIELD_REAL REAL, \
            FIELD_DOUBLE DOUBLE, \
            FIELD_CHAR CHAR(12), \
            FIELD_VARCHAR VARCHAR(12), \
            FIELD_NCHAR NCHAR(12), \
            FIELD_NVARCHAR NVARCHAR(12), \
            FIELD_BINARY BINARY(8), \
            FIELD_VARBINARY VARBINARY(8) \
        )",
    ])?;
    Ok(())
}

fn write(
    _log_handle: &mut ReconfigurationHandle,
    connection: &mut Connection,
) -> HdbResult<()> {
    info!("insert values directly");
    connection.dml("\
        insert into TEST_TYPES_A \
        ( \
            FIELD_TINYINT, FIELD_SMALLINT, FIELD_INT, FIELD_BIGINT, \
            FIELD_SMALLDECIMAL, FIELD_DECIMAL, FIELD_REAL, FIELD_DOUBLE, \
            FIELD_CHAR, FIELD_VARCHAR, FIELD_NCHAR, FIELD_NVARCHAR, \
            FIELD_BINARY, FIELD_VARBINARY \
        ) values( \
            1, 1, 1, 1, \
            1.0, 1.0, 1.0, 1.0, \
            'Hello world!', 'Hello world!', 'Hello world!', 'Hello world!', \
            '0123456789abcdef', '0123456789abcdef' \
        )")?;

    info!("insert values via prep-statement");
    let mut stmt = connection.prepare("\
        insert into TEST_TYPES_A \
        ( \
            FIELD_TINYINT, FIELD_SMALLINT, FIELD_INT, FIELD_BIGINT, \
            FIELD_SMALLDECIMAL, FIELD_DECIMAL, FIELD_REAL, FIELD_DOUBLE, \
            FIELD_CHAR, FIELD_VARCHAR, FIELD_NCHAR, FIELD_NVARCHAR, \
            FIELD_BINARY, FIELD_VARBINARY \
        ) values(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")?;
    stmt.execute(&(
        1, 1, 1, 1, 
        1.0, 1.0, 1.0_f32, 1.0, 
        "Hello world!", "Hello world!", "Hello world!", "Hello world!", 
        Bytes::new(&parse_hex("0123456789abcdef")), Bytes::new(&parse_hex("0123456789abcdef")),
    ))?;

    info!("insert nulls directly");
    connection.dml("insert into TEST_TYPES_A \
    ( \
        FIELD_TINYINT, FIELD_SMALLINT, FIELD_INT, FIELD_BIGINT, \
        FIELD_SMALLDECIMAL, FIELD_DECIMAL, FIELD_REAL, FIELD_DOUBLE, \
        FIELD_CHAR, FIELD_VARCHAR, FIELD_NCHAR, FIELD_NVARCHAR, \
        FIELD_BINARY, FIELD_VARBINARY \
    ) values( \
        NULL, NULL, NULL, NULL, \
        NULL, NULL, NULL, NULL, \
        NULL, NULL, NULL, NULL, \
        NULL, NULL \
    )")?;

    info!("insert nulls via prep-statement");
    stmt.execute(&(
        HdbValue::N_TINYINT(None), HdbValue::N_SMALLINT(None), 
        HdbValue::N_INT(None), HdbValue::N_BIGINT(None), 
        HdbValue::N_DECIMAL(None), HdbValue::N_DECIMAL(None), 
        HdbValue::N_REAL(None), HdbValue::N_DOUBLE(None), 
        HdbValue::N_CHAR(None), HdbValue::N_VARCHAR(None), 
        HdbValue::N_NCHAR(None), HdbValue::N_NVARCHAR(None), 
        HdbValue::N_BINARY(None), HdbValue::N_VARBINARY(None),
    ))?;

    Ok(())
}

fn read(
    _log_handle: &mut ReconfigurationHandle,
    _connection: &mut Connection,
) -> HdbResult<()> {
    Ok(())
}




fn parse_hex(hex_asm: &str) -> Vec<u8> {
    let mut hex_bytes = hex_asm.as_bytes().iter().filter_map(|b| {
        match b {
            b'0'...b'9' => Some(b - b'0'),
            b'a'...b'f' => Some(b - b'a' + 10),
            b'A'...b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }).fuse();

    let mut bytes = Vec::new();
    while let (Some(h), Some(l)) = (hex_bytes.next(), hex_bytes.next()) {
        bytes.push(h << 4 | l)
    }
    bytes
}