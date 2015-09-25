use byteorder::{BigEndian,LittleEndian,ReadBytesExt,WriteBytesExt};
use std::io::Result as IoResult;
use std::io::{BufRead,BufReader,Write};
use std::net::TcpStream;

pub fn send_and_receive(stream: &mut TcpStream) -> IoResult<(i8,i16)> {
    trace!("Entering send_and_receive()");
    try!(send(stream));

    let mut rdr = BufReader::new(stream);
    receive(&mut rdr)
}

fn send(w: &mut Write) -> IoResult<()> {
    trace!("Entering send()");
    let mut b = Vec::<u8>::with_capacity(14); // FIXME b appears to be unneccessary!
    try!(b.write_i32::<BigEndian>(-1));     // I4    Filler xFFFFFFFF
    try!(b.write_i8(4));                    // I1    Major Product Version
    try!(b.write_i16::<BigEndian>(20));     // I2    Minor Product Version
    try!(b.write_i8(4));                    // I1    Major Protocol Version
    try!(b.write_i16::<BigEndian>(1));      // I2    Minor Protocol Version
    try!(b.write_i8(0));                    // I1    Reserved

    try!(b.write_i8(1));                    // I1    Number of Options
    try!(b.write_i8(1));                    // I1    Option-id "Swap-kind"
    try!(b.write_i8(1));                    // I1    value "LittleEndian" (Big endian would be 0)
    try!(w.write(&b));
    w.flush()
    // debug!("serialize_request: request successfully sent");
}

fn receive(rdr: &mut BufRead) -> IoResult<(i8,i16)> {
    trace!("Entering receive()");
    let major: i8 = try!(rdr.read_i8());                    // I1    Major Product Version
    let minor: i16 = try!(rdr.read_i16::<LittleEndian>());  // I2    Minor Product Version
    rdr.consume(5);                                         // ignore the rest (04:01:00:00:00)?
    debug!("successfully initialized");
    Ok((major,minor))
}
