use {HdbError, HdbResult};
use protocol::lowlevel::cesu8;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io;
use std::mem;

#[derive(Debug)]
pub struct AuthField(Vec<u8>);

impl AuthField {
    pub fn new(vec: Vec<u8>) -> AuthField {
        AuthField(vec)
    }

    pub fn into_data(self) -> Vec<u8> {
        self.0
    }

    pub fn swap_data(&mut self, vec: &mut Vec<u8>) {
        mem::swap(&mut self.0, vec);
    }

    pub fn serialize(&self, w: &mut io::Write) -> HdbResult<()> {
        match self.0.len() {
            l if l <= 250_usize => w.write_u8(l as u8)?, // B1: length of value
            l if l <= 65_535_usize => {
                w.write_u8(255)?; // B1: 247
                w.write_u16::<LittleEndian>(l as u16)?; // U2: length of value
            }
            l => {
                return Err(HdbError::Impl(format!(
                    "Value of AuthField is too big: {}",
                    l
                )));
            }
        }
        cesu8::serialize_bytes(&self.0, w) // B (varying) value
    }

    pub fn size(&self) -> usize {
        1 + self.0.len()
    }

    pub fn parse(rdr: &mut io::BufRead) -> HdbResult<AuthField> {
        let mut len = rdr.read_u8()? as usize; // B1
        match len {
            255 => {
                len = rdr.read_u16::<LittleEndian>()? as usize; // (B1+)I2
            }
            251...254 => {
                return Err(HdbError::Impl(format!(
                    "Unknown length indicator for AuthField: {}",
                    len
                )));
            }
            _ => {}
        }
        Ok(AuthField(cesu8::parse_bytes(len, rdr)?))
    }
}
