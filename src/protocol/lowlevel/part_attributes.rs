use std::fmt;

// bit pattern for some attribute parts
#[derive(Clone)]
pub struct PartAttributes(u8);
impl PartAttributes {
    pub fn new(bits: u8) -> PartAttributes {
        PartAttributes(bits)
    }

    const IS_LAST_PACKET: u8    = 0b_0000_0001;     // Last part in a sequence of parts (FETCH, array command EXECUTE)
    const HAS_NEXT_PACKET: u8   = 0b_0000_0010;     // Part in a sequence of parts
    const IS_FIRST_PACKET: u8   = 0b_0000_0100;     // First part in a sequence of parts
    const ROW_NOT_FOUND: u8     = 0b_0000_1000;     // Empty part, caused by “row not found” error
    const RESULTSET_CLOSED: u8  = 0b_0001_0000;     // The result set that produced this part is closed

    pub fn is_last_packet(&self) -> bool {
        (self.0 & PartAttributes::IS_LAST_PACKET) != 0
    }
    fn has_next_packet(&self) -> bool {
        (self.0 & PartAttributes::HAS_NEXT_PACKET) != 0
    }
    fn is_first_packet(&self) -> bool {
        (self.0 & PartAttributes::IS_FIRST_PACKET) != 0
    }
    pub fn row_not_found(&self) -> bool {
        (self.0 & PartAttributes::ROW_NOT_FOUND) != 0
    }
    pub fn is_resultset_closed(&self) -> bool {
        (self.0 & PartAttributes::RESULTSET_CLOSED) != 0
    }
}

impl fmt::Debug for PartAttributes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 == 0 {
            write!(f, "(NONE)")
        } else {
            let mut b = false;
            try!(write!(f, "("));
            if self.is_last_packet() {
                b = true;
                try!(write!(f, "IS_LAST_PACKET"));
            };
            if self.has_next_packet() {
                b = try!(w_and(b, f));
                try!(write!(f, "HAS_NEXT_PACKET"));
            };
            if self.is_first_packet() {
                b = try!(w_and(b, f));
                try!(write!(f, "IS_FIRST_PACKET_IN_A_SEQUENCE"));
            };
            if self.row_not_found() {
                b = try!(w_and(b, f));
                try!(write!(f, "ROW_NOT_FOUND"));
            };
            if self.is_resultset_closed() {
                try!(w_and(b, f));
                try!(write!(f, "RESULTSET_CLOSED"));
            };
            write!(f, ")")
        }
    }
}
impl fmt::Binary for PartAttributes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:b}", self.0)
    }
}

// write an ampersand (&) if required
fn w_and(b: bool, f: &mut fmt::Formatter) -> Result<bool, fmt::Error> {
    if b {
        try!(write!(f, " & "));
    }
    Ok(true)
}
