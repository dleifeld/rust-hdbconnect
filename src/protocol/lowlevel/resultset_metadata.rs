use super::util;

use byteorder::{LittleEndian,ReadBytesExt};
use std::io::Result as IoResult;
use std::io::BufRead;
use std::u32;
use vec_map::VecMap;


/// contains a table of field metadata;
/// the variable-length Strings are extracted into the names vecmap, which uses an integer as key
#[derive(Clone,Debug)]
pub struct ResultSetMetadata {
    pub fields: Vec<FieldMetadata>,
    names: VecMap<String>
}
impl ResultSetMetadata {
    pub fn parse(count: i32, arg_size:u32, rdr: &mut BufRead) -> IoResult<ResultSetMetadata> {
        let mut rsm = ResultSetMetadata {
            fields: Vec::<FieldMetadata>::new(),
            names: VecMap::<String>::new(),
        };
        trace!("Got count {}",count);
        for _ in 0..count {
            let co = ColumnOption::from_u8(try!(rdr.read_u8()));                // U1 (documented as I1)
            let vt = try!(rdr.read_u8());                                       // I1
            let fr = try!(rdr.read_i16::<LittleEndian>());                      // I2
            let pr = try!(rdr.read_i16::<LittleEndian>());                      // I2
            try!(rdr.read_i16::<LittleEndian>());                               // I2
            let tn = try!(rdr.read_u32::<LittleEndian>());                      // I4
            rsm.add_to_names(tn);
            let sn = try!(rdr.read_u32::<LittleEndian>());                      // I4
            rsm.add_to_names(sn);
            let cn = try!(rdr.read_u32::<LittleEndian>());                      // I4
            rsm.add_to_names(cn);
            let cdn = try!(rdr.read_u32::<LittleEndian>());                     // I4
            rsm.add_to_names(cdn);

            let fm = FieldMetadata::new(co,vt,fr,pr,tn,sn,cn,cdn);
            rsm.fields.push(fm);
        }
        trace!("Read ResultSetMetadata phase 1: {:?}",rsm);
        // now we read the names
        let mut offset = 0;
        let limit = arg_size - (count as u32) * 22;
        trace!("arg_size = {}, count = {}, limit = {} ", arg_size, count, limit);
        for _ in 0..rsm.names.len() {
            if offset >= limit {panic!("Error in reading ResultSetMetadata")};
            let nl = try!(rdr.read_u8());                                       // UI1
            let buffer: Vec<u8> = try!(util::parse_bytes(nl as usize,rdr));     // variable
            let name = try!(util::cesu8_to_string(&buffer));
            trace!("offset = {}, name = {}",offset, name);
            rsm.names.insert(offset as usize,name);
            offset += (nl as u32) + 1;
        }
        Ok(rsm)
    }

    fn add_to_names(&mut self, offset: u32) {
        if offset != u32::MAX {
            let tn = offset as usize;
            if !self.names.contains_key(&tn) {
                self.names.insert(tn,"".to_string());
            };
        }
    }


    pub fn count(&self) -> i16 {
        self.fields.len() as i16
    }

    pub fn size(&self) -> usize {
        let mut size = self.fields.len() * 22;
        for name in self.names.values() {
            size += 1 + name.len();
        }
        size
    }
}

#[derive(Clone,Debug)]
pub struct FieldMetadata {
    pub column_options: ColumnOption,
    pub value_type: u8,
    pub fraction: i16,
    pub precision: i16,
    pub tablename: u32,
    pub schemaname: u32,
    pub columnname: u32,
    pub column_displayname: u32,
}
impl FieldMetadata {
    fn new(co: ColumnOption, vt: u8, fr: i16, pr: i16, tn: u32, sn: u32, cn: u32, cdn: u32,) -> FieldMetadata {
        FieldMetadata {
            column_options: co, value_type: vt, fraction: fr, precision: pr, tablename: tn,
            schemaname: sn, columnname: cn, column_displayname: cdn
        }
    }
}

#[derive(Clone,Debug)]
pub struct ColumnOption {
    co: u8
}
impl ColumnOption {
    #[allow(dead_code)]
    fn is_mandatory(&self) -> bool {
        (self.co & 0b1000_0000) != 0
    }
    #[allow(dead_code)]
    fn is_optional(&self) -> bool {
        (self.co & 0b0100_0000) != 0
    }
    fn from_u8(val: u8) -> ColumnOption {
        ColumnOption{co: val}
    }
}
