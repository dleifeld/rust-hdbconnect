use crate::protocol::util;
#[cfg(feature = "async")]
use crate::protocol::util_async;
use crate::types_impl::hdb_decimal::HdbDecimal;
use crate::{HdbValue, TypeId};
use bigdecimal::BigDecimal;
#[cfg(feature = "sync")]
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use num::{FromPrimitive, ToPrimitive};
use num_bigint::BigInt;

#[cfg(feature = "sync")]
pub fn parse_sync(
    nullable: bool,
    type_id: TypeId,
    scale: i16,
    rdr: &mut dyn std::io::Read,
) -> std::io::Result<HdbValue<'static>> {
    match type_id {
        TypeId::DECIMAL => HdbDecimal::parse_hdb_decimal_sync(nullable, scale, rdr),

        TypeId::FIXED8 => Ok(if parse_null_sync(nullable, rdr)? {
            HdbValue::NULL
        } else {
            trace!("parse FIXED8");
            let i = rdr.read_i64::<LittleEndian>()?;
            let bigint = BigInt::from_i64(i)
                .ok_or_else(|| util::io_error("invalid value of type FIXED8"))?;
            let bd = BigDecimal::new(bigint, i64::from(scale));
            HdbValue::DECIMAL(bd)
        }),

        TypeId::FIXED12 => Ok(if parse_null_sync(nullable, rdr)? {
            HdbValue::NULL
        } else {
            trace!("parse FIXED12");
            let bytes = crate::protocol::util_sync::parse_bytes(12, rdr)?;
            let bigint = BigInt::from_signed_bytes_le(&bytes);
            let bd = BigDecimal::new(bigint, i64::from(scale));
            HdbValue::DECIMAL(bd)
        }),

        TypeId::FIXED16 => Ok(if parse_null_sync(nullable, rdr)? {
            HdbValue::NULL
        } else {
            trace!("parse FIXED16");
            let i = rdr.read_i128::<LittleEndian>()?;
            let bi = BigInt::from_i128(i)
                .ok_or_else(|| util::io_error("invalid value of type FIXED16"))?;
            let bd = BigDecimal::new(bi, i64::from(scale));
            HdbValue::DECIMAL(bd)
        }),
        _ => Err(util::io_error("unexpected type id for decimal")),
    }
}

#[cfg(feature = "async")]
pub async fn parse_async<R: std::marker::Unpin + tokio::io::AsyncReadExt>(
    nullable: bool,
    type_id: TypeId,
    scale: i16,
    rdr: &mut R,
) -> std::io::Result<HdbValue<'static>> {
    match type_id {
        TypeId::DECIMAL => HdbDecimal::parse_hdb_decimal_async(nullable, scale, rdr).await,

        TypeId::FIXED8 => Ok(if parse_null_async(nullable, rdr).await? {
            HdbValue::NULL
        } else {
            trace!("parse FIXED8");
            let i = util_async::read_i64(rdr).await?;
            let bigint = BigInt::from_i64(i)
                .ok_or_else(|| util::io_error("invalid value of type FIXED8"))?;
            let bd = BigDecimal::new(bigint, i64::from(scale));
            HdbValue::DECIMAL(bd)
        }),

        TypeId::FIXED12 => Ok(if parse_null_async(nullable, rdr).await? {
            HdbValue::NULL
        } else {
            trace!("parse FIXED12");
            let bytes = util_async::parse_bytes(12, rdr).await?;
            let bigint = BigInt::from_signed_bytes_le(&bytes);
            let bd = BigDecimal::new(bigint, i64::from(scale));
            HdbValue::DECIMAL(bd)
        }),

        TypeId::FIXED16 => Ok(if parse_null_async(nullable, rdr).await? {
            HdbValue::NULL
        } else {
            trace!("parse FIXED16");
            let i = util_async::read_i128(rdr).await?;
            let bi = BigInt::from_i128(i)
                .ok_or_else(|| util::io_error("invalid value of type FIXED16"))?;
            let bd = BigDecimal::new(bi, i64::from(scale));
            HdbValue::DECIMAL(bd)
        }),
        _ => Err(util::io_error("unexpected type id for decimal")),
    }
}

#[cfg(feature = "sync")]
fn parse_null_sync(nullable: bool, rdr: &mut dyn std::io::Read) -> std::io::Result<bool> {
    let is_null = rdr.read_u8()? == 0;
    if is_null && !nullable {
        Err(util::io_error("found null value for not-null column"))
    } else {
        Ok(is_null)
    }
}

#[cfg(feature = "async")]
async fn parse_null_async<R: std::marker::Unpin + tokio::io::AsyncReadExt>(
    nullable: bool,
    rdr: &mut R,
) -> std::io::Result<bool> {
    let is_null = rdr.read_u8().await? == 0;
    if is_null && !nullable {
        Err(util::io_error("found null value for not-null column"))
    } else {
        Ok(is_null)
    }
}

#[cfg(feature = "sync")]
pub(crate) fn sync_emit(
    bd: &BigDecimal,
    type_id: TypeId,
    scale: i16,
    w: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    match type_id {
        TypeId::DECIMAL => {
            trace!("emit DECIMAL");
            let hdb_decimal =
                HdbDecimal::from_bigdecimal(bd).map_err(|e| util::io_error(e.to_string()))?;
            w.write_all(&hdb_decimal.into_raw())?;
        }
        TypeId::FIXED8 => {
            trace!("emit FIXED8");
            let bd = bd.with_scale(i64::from(scale));
            let (bigint, _exponent) = bd.as_bigint_and_exponent();
            w.write_i64::<LittleEndian>(
                bigint
                    .to_i64()
                    .ok_or_else(|| util::io_error("conversion to FIXED8 fails"))?,
            )?;
        }
        TypeId::FIXED12 => {
            trace!("emit FIXED12");
            // if we get less than 12 bytes, we need to append bytes with either value
            // 0_u8 or 255_u8, depending on the value of the highest bit of the last byte.
            let bd = bd.with_scale(i64::from(scale));
            let (bigint, _exponent) = bd.as_bigint_and_exponent();
            let mut bytes = bigint.to_signed_bytes_le();
            let l = bytes.len();
            if l < 12 {
                let filler = if bytes[l - 1] & 0b_1000_0000_u8 == 0 {
                    0_u8
                } else {
                    255_u8
                };
                bytes.reserve(12 - l);
                for _ in l..12 {
                    bytes.push(filler);
                }
            }
            w.write_all(&bytes)?;
        }
        TypeId::FIXED16 => {
            trace!("emit FIXED16");
            let bd = bd.with_scale(i64::from(scale));
            let (bigint, _exponent) = bd.as_bigint_and_exponent();
            w.write_i128::<LittleEndian>(
                bigint
                    .to_i128()
                    .ok_or_else(|| util::io_error("conversion to FIXED16 fails"))?,
            )?;
        }
        _ => return Err(util::io_error("unexpected type id for decimal")),
    }
    Ok(())
}

#[cfg(feature = "async")]
pub(crate) async fn async_emit<W: std::marker::Unpin + tokio::io::AsyncWriteExt>(
    bd: &BigDecimal,
    type_id: TypeId,
    scale: i16,
    w: &mut W,
) -> std::io::Result<()> {
    match type_id {
        TypeId::DECIMAL => {
            trace!("emit DECIMAL");
            let hdb_decimal =
                HdbDecimal::from_bigdecimal(bd).map_err(|e| util::io_error(e.to_string()))?;
            w.write_all(&hdb_decimal.into_raw()).await?;
        }
        TypeId::FIXED8 => {
            trace!("emit FIXED8");
            let bd = bd.with_scale(i64::from(scale));
            let (bigint, _exponent) = bd.as_bigint_and_exponent();
            w.write_i64_le(
                bigint
                    .to_i64()
                    .ok_or_else(|| util::io_error("conversion to FIXED8 fails"))?,
            )
            .await?;
        }
        TypeId::FIXED12 => {
            trace!("emit FIXED12");
            // if we get less than 12 bytes, we need to append bytes with either value
            // 0_u8 or 255_u8, depending on the value of the highest bit of the last byte.
            let bd = bd.with_scale(i64::from(scale));
            let (bigint, _exponent) = bd.as_bigint_and_exponent();
            let mut bytes = bigint.to_signed_bytes_le();
            let l = bytes.len();
            if l < 12 {
                let filler = if bytes[l - 1] & 0b_1000_0000_u8 == 0 {
                    0_u8
                } else {
                    255_u8
                };
                bytes.reserve(12 - l);
                for _ in l..12 {
                    bytes.push(filler);
                }
            }
            w.write_all(&bytes).await?;
        }
        TypeId::FIXED16 => {
            trace!("emit FIXED16");
            let bd = bd.with_scale(i64::from(scale));
            let (bigint, _exponent) = bd.as_bigint_and_exponent();
            w.write_i128_le(
                bigint
                    .to_i128()
                    .ok_or_else(|| util::io_error("conversion to FIXED16 fails"))?,
            )
            .await?;
        }
        _ => return Err(util::io_error("unexpected type id for decimal")),
    }
    Ok(())
}
