use crate::protocol::util;
use crate::HdbValue;
use bigdecimal::{BigDecimal, Zero};
use byteorder::{ByteOrder, LittleEndian};
use num_bigint::{BigInt, Sign};
use serde_db::ser::SerializationError;

// MANTISSA     113-bit     Integer mantissa
//                          (byte 0; byte 14, lowest bit)
// EXPONENT      14-bit     Exponent, biased with 6176, leading to a range -6143 to +6144
//                          (byte 14, above lowest bit; byte 15, below highest bit)
// SIGN           1-bit     Sign: 0 is positive, 1 is negative
//                          (byte 15, highest bit)
//
// The represented number is (10^EXPONENT)*MANTISSA.
// It is expected that MANTISSA is not a multiple of 10.

// Intermediate representation of HANA's DECIMAL type; is only used when reading
// from or writing to the wire.
#[derive(Clone, Debug)]
pub struct HdbDecimal {
    raw: [u8; 16],
}
impl HdbDecimal {
    pub fn new(raw: [u8; 16]) -> Self {
        Self { raw }
    }
    pub fn parse_hdb_decimal(
        nullable: bool,
        scale: i16,
        rdr: &mut dyn std::io::Read,
    ) -> std::io::Result<HdbValue<'static>> {
        let mut raw = [0_u8; 16];
        rdr.read_exact(&mut raw[..])?;

        if raw[15] == 112 && raw[0..=14].iter().all(|el| *el == 0) {
            // it's a NULL!
            if nullable {
                Ok(HdbValue::NULL)
            } else {
                Err(util::io_error("received null value for not-null column"))
            }
        } else {
            trace!("parse DECIMAL");
            let bd = Self::new(raw).into_bigdecimal_with_scale(scale);
            Ok(HdbValue::DECIMAL(bd))
        }
    }

    // Creates an HdbDecimal from a BigDecimal.
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    pub fn from_bigdecimal(bigdecimal: &BigDecimal) -> Result<Self, SerializationError> {
        let ten = BigInt::from(10_u8);
        let (sign, mantissa, exponent) = {
            let (mut bigint, neg_exponent) = bigdecimal.as_bigint_and_exponent();
            let mut exponent = -neg_exponent;

            // HANA does not like mantissas that are multiples of 10
            while !bigint.is_zero() && (&bigint % &ten).is_zero() {
                bigint /= 10;
                exponent += 1;
            }

            // HANA accepts only mantissas up to 113 bits, so we round if necessary
            loop {
                let (_, mantissa) = bigint.to_bytes_le();
                let l = mantissa.len();
                if (l > 15) || ((l == 15) && (mantissa[14] & 0b1111_1110) != 0) {
                    bigint /= 10;
                    exponent += 1;
                } else {
                    break;
                }
            }

            if exponent < -6143 || exponent > 6144 {
                return Err(SerializationError::Serde(format!(
                    "exponent '{}' out of range",
                    exponent
                )));
            }
            let (sign, mantissa) = bigint.to_bytes_le();
            (sign, mantissa, exponent)
        };

        let mut raw = [0_u8; 16];
        (&mantissa)
            .iter()
            .enumerate()
            .for_each(|(i, b)| raw[i] = *b);

        let biased_exponent: u16 = (exponent + 6176) as u16; // bounds are checked above
        LittleEndian::write_u16(&mut raw[14..=15], biased_exponent * 2);

        if let Sign::Minus = sign {
            raw[15] |= 0b_1000_0000_u8;
        }
        Ok(Self { raw })
    }

    pub fn into_bigdecimal_with_scale(self, scale: i16) -> BigDecimal {
        let mut bd = self.into_bigdecimal();
        if scale < i16::max_value() {
            bd = bd.with_scale(i64::from(scale));
        }
        bd
    }
    fn into_bigdecimal(self) -> BigDecimal {
        let (is_negative, mantissa, exponent) = self.into_elements();
        if is_negative {
            -BigDecimal::new(mantissa, -exponent)
        } else {
            BigDecimal::new(mantissa, -exponent)
        }
    }

    // Retrieve the ingredients of the HdbDecimal
    pub fn into_elements(mut self) -> (bool, BigInt, i64) {
        let is_negative = (self.raw[15] & 0b_1000_0000_u8) != 0;
        self.raw[15] &= 0b_0111_1111_u8;
        let exponent = i64::from(LittleEndian::read_u16(&self.raw[14..=15]) >> 1) - 6176;
        self.raw[14] &= 0b_0000_0001_u8;
        let mantissa = BigInt::from_bytes_le(Sign::Plus, &self.raw[0..=14]);
        (is_negative, mantissa, exponent)
    }
    pub fn into_raw(self) -> [u8; 16] {
        self.raw
    }
}

#[cfg(test)]
mod tests {
    use super::HdbDecimal;
    use bigdecimal::BigDecimal;
    use num::bigint::BigInt;
    use std::str::FromStr;

    #[test]
    fn test_all() {
        flexi_logger::Logger::with_str("info").start().unwrap();

        str_2_big_2_hdb_2_big("1234.56780000");
        str_2_big_2_hdb_2_big("1234.5678");
        str_2_big_2_hdb_2_big("-1234.5678");

        str_2_big_2_hdb_2_big("123456789");
        str_2_big_2_hdb_2_big("123456789.0000");
        str_2_big_2_hdb_2_big("0.1234567890000");
        str_2_big_2_hdb_2_big(
            "0.000000000000000000000000000000000000000000000000000001234567890000",
        );

        str_2_big_2_hdb_2_big("-123456789");
        str_2_big_2_hdb_2_big("-123456789.0000");
        str_2_big_2_hdb_2_big("-0.1234567890000");
        str_2_big_2_hdb_2_big(
            "-0.000000000000000000000000000000000000000000000000000001234567890000",
        );

        str_2_big_2_hdb_2_big("123456789123456789");
        str_2_big_2_hdb_2_big("1234567890012345678900000");
        str_2_big_2_hdb_2_big("1234567890000000000000000123456789");

        me_2_big_2_hdb_2_big(BigInt::from_str("0").unwrap(), 0);
        me_2_big_2_hdb_2_big(BigInt::from_str("1234567890").unwrap(), -5);
        me_2_big_2_hdb_2_big(BigInt::from_str("1234567890000").unwrap(), -8);

        me_2_big_2_hdb_2_big(
            BigInt::from_str("123456789012345678901234567890").unwrap(),
            0,
        );
        me_2_big_2_hdb_2_big(
            BigInt::from_str("1234567890123456789012345678901234000").unwrap(),
            0,
        );
        me_2_big_2_hdb_2_big(
            BigInt::from_str("1234567890123456789012345678901234").unwrap(),
            3,
        );
    }

    fn str_2_big_2_hdb_2_big(input: &str) {
        debug!("input:  {}", input);
        let bigdec = BigDecimal::from_str(input).unwrap();
        big_2_hdb_2_big(&bigdec);
    }

    fn me_2_big_2_hdb_2_big(mantissa: BigInt, exponent: i64) {
        debug!("mantissa: {}, exponent: {}", mantissa, exponent);
        let bigdec = BigDecimal::new(mantissa, -exponent);
        big_2_hdb_2_big(&bigdec);
    }

    fn big_2_hdb_2_big(bigdec: &BigDecimal) {
        let hdbdec = HdbDecimal::from_bigdecimal(&bigdec).unwrap();
        let (s, m, e) = hdbdec.clone().into_elements();
        let bigdec2 = hdbdec.clone().into_bigdecimal();
        debug!("bigdec:  {:?}", bigdec);
        debug!("hdbdec:  {:?}", hdbdec);
        debug!("s: {:?}, m: {}, e: {}", s, m, e);
        debug!("bigdec2: {:?}\n", bigdec2);
        assert_eq!(*bigdec, bigdec2, "start != end");
    }
}
