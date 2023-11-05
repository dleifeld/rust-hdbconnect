extern crate serde;

mod test_utils;

use bigdecimal::BigDecimal;
use flexi_logger::LoggerHandle;
use hdbconnect_async::{Connection, HdbResult, HdbValue};
use log::{debug, info};
use num::FromPrimitive;
use serde::Deserialize;

//cargo test --test test_025_decimals -- --nocapture
#[tokio::test]
async fn test_025_decimals() -> HdbResult<()> {
    let mut log_handle = test_utils::init_logger();
    let start = std::time::Instant::now();
    let connection = test_utils::get_authenticated_connection().await?;

    if connection.data_format_version_2().await > 7 {
        info!("=== run test for FIXED8 ===");
        test_025_decimals_impl(TS::Fixed8, &mut log_handle, &connection).await?;

        info!("=== run test for FIXED12 ===");
        test_025_decimals_impl(TS::Fixed12, &mut log_handle, &connection).await?;

        info!("=== run test for FIXED16 ===");
        test_025_decimals_impl(TS::Fixed16, &mut log_handle, &connection).await?;
    } else {
        info!("=== run test for old wire DECIMAL ===");
        test_025_decimals_impl(TS::Decimal, &mut log_handle, &connection).await?;
    }

    test_utils::closing_info(connection, start).await
}

enum TS {
    Fixed8,
    Fixed12,
    Fixed16,
    Decimal,
}

async fn test_025_decimals_impl(
    ts: TS,
    _log_handle: &mut LoggerHandle,
    connection: &Connection,
) -> HdbResult<()> {
    info!("setup ...");
    connection
        .multiple_statements_ignore_err(vec!["drop table TEST_DECIMALS"])
        .await;
    let stmts = vec![
        match ts {
            TS::Decimal =>
        "create table TEST_DECIMALS (s NVARCHAR(100) primary key, d1 DECIMAL(7,5), d2 DECIMAL(7,5), dummy integer)",
            TS::Fixed8 =>
        "create table TEST_DECIMALS (s NVARCHAR(100) primary key, d1 DECIMAL(7,5), d2 DECIMAL(7,5), dummy integer)",
            TS::Fixed12 =>
        "create table TEST_DECIMALS (s NVARCHAR(100) primary key, d1 DECIMAL(28,5), d2 DECIMAL(28,5), dummy integer)",
            TS::Fixed16 =>
        "create table TEST_DECIMALS (s NVARCHAR(100) primary key, d1 DECIMAL(38,5), d2 DECIMAL(38,5), dummy integer)",
        },
        "insert into TEST_DECIMALS (s, d1, d2) values('0.00000', '0.00000', 0.000)",
        "insert into TEST_DECIMALS (s, d1, d2) values('0.00100', '0.00100', 0.001)",
        "insert into TEST_DECIMALS (s, d1, d2) values('-0.00100', '-0.00100', -0.001)",
        "insert into TEST_DECIMALS (s, d1, d2) values('0.00300', '0.00300', 0.003)",
        "insert into TEST_DECIMALS (s, d1, d2) values('0.00700', '0.00700', 0.007)",
        "insert into TEST_DECIMALS (s, d1, d2) values('0.25500', '0.25500', 0.255)",
        "insert into TEST_DECIMALS (s, d1, d2) values('65.53500', '65.53500', 65.535)",
        "insert into TEST_DECIMALS (s, d1, d2) values('-65.53500', '-65.53500', -65.535)",
    ];
    connection.multiple_statements(stmts).await?;

    #[derive(Deserialize)]
    struct TestData {
        #[serde(rename = "S")]
        s: String,
        #[serde(rename = "D1")]
        d1: BigDecimal,
        #[serde(rename = "D2")]
        d2: BigDecimal,
    }

    let insert_stmt_str = "insert into TEST_DECIMALS (s, d1, d2) values(?, ?, ?)";

    info!("prepare & execute");
    let mut insert_stmt = connection.prepare(insert_stmt_str).await?;
    insert_stmt.add_batch(&(
        "75.53500",
        "75.53500",
        BigDecimal::from_f32(75.535).unwrap(),
    ))?;

    // had to change the next two lines when moving from bigdecimal 0.3 to 0.4
    // insert_stmt.add_batch(&("87.65432", "87.65432", 87.654_32_f32))?;
    // insert_stmt.add_batch(&("0.00500", "0.00500", 0.005_f32))?;
    #[allow(clippy::excessive_precision)]
    insert_stmt.add_batch(&("87.65432", "87.65432", 87.654_325_f32))?;
    insert_stmt.add_batch(&("0.00500", "0.00500", 0.005001_f32))?;

    insert_stmt.add_batch(&("-0.00600", "-0.00600", -0.006_00_f64))?;
    insert_stmt.add_batch(&("-7.65432", "-7.65432", -7.654_32_f64))?;
    insert_stmt.add_batch(&("99.00000", "99.00000", 99))?;
    insert_stmt.add_batch(&("-50.00000", "-50.00000", -50_i16))?;
    insert_stmt.add_batch(&("22.00000", "22.00000", 22_i64))?;
    insert_stmt.execute_batch().await?;

    insert_stmt.add_batch(&("-0.05600", "-0.05600", "-0.05600"))?;
    insert_stmt.add_batch(&("-8.65432", "-8.65432", "-8.65432"))?;
    insert_stmt.execute_batch().await?;

    info!("Read and verify decimals");
    let resultset = connection
        .query("select s, d1, d2 from TEST_DECIMALS order by d1")
        .await?;
    for row in resultset.into_rows().await? {
        if let HdbValue::DECIMAL(ref bd) = &row[1] {
            assert_eq!(format!("{}", &row[0]), format!("{bd}"));
        } else {
            panic!("Unexpected value type");
        }
    }

    info!("Read and verify decimals to struct");
    let resultset = connection
        .query("select s, d1, d2 from TEST_DECIMALS order by d1")
        .await?;
    let scale = resultset.metadata()[1].scale() as usize;
    let result: Vec<TestData> = resultset.try_into().await?;
    for td in result {
        debug!("{:?}, {:?}, {:?}", td.s, td.d1, td.d2);
        assert_eq!(td.s, format!("{0:.1$}", td.d1, scale));
        assert_eq!(td.s, format!("{0:.1$}", td.d2, scale));
    }

    info!("Read and verify decimals to tuple");
    let result: Vec<(String, String, String)> = connection
        .query("select * from TEST_DECIMALS")
        .await?
        .try_into()
        .await?;
    for row in result {
        debug!("{}, {}, {}", row.0, row.1, row.2);
        assert_eq!(row.0, row.1);
        assert_eq!(row.0, row.2);
    }

    info!("Read and verify decimal to single value");
    let resultset = connection
        .query("select AVG(dummy) from TEST_DECIMALS")
        .await?;
    let mydata: Option<BigDecimal> = resultset.try_into().await?;
    assert_eq!(mydata, None);

    let mydata: Option<i64> = connection
        .query("select AVG(D2) from TEST_DECIMALS where d1 = '65.53500'")
        .await?
        .try_into()
        .await?;
    assert_eq!(mydata, Some(65));

    info!("test failing conversion");
    let mydata: HdbResult<i8> = connection
        .query("select SUM(ABS(D2)) from TEST_DECIMALS")
        .await?
        .try_into()
        .await;
    assert!(mydata.is_err());

    info!("test working conversion");
    let mydata: i64 = connection
        .query("select SUM(ABS(D2)) from TEST_DECIMALS")
        .await?
        .try_into()
        .await?;
    assert_eq!(mydata, 481);

    Ok(())
}
