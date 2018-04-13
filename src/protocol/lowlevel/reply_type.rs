use {HdbError, HdbResult};

// Identifies the nature of the statement or functionality that has been
// prepared or executed. Is documented as Function Code.
// Irrelevant numbers (ABAP stuff, "reserved") are not listed.
#[derive(Debug)]
pub enum ReplyType {
    Nil,                       // Nil
    Ddl,                       // DDL statement
    Insert,                    // INSERT statement
    Update,                    // UPDATE statement
    Delete,                    // DELETE statement
    Select,                    // SELECT statement
    SelectForUpdate,           // SELECT … FOR UPDATE statement
    Explain,                   // EXPLAIN statement
    DbProcedureCall,           // CALL statement
    DbProcedureCallWithResult, // CALL statement returning one or more results
    Fetch,                     // FETCH message
    Commit,                    // COMMIT message or statement
    Rollback,                  // ROLLBACK message or statement
    Connect,                   // CONNECT or AUTHENTICATION message
    WriteLob,                  // WRITELOB message
    ReadLob,                   // READLOB message
    Disconnect,                // DISCONNECT message
    CloseCursor,               // CLOSECURSOR message
    FindLob,                   // FINDLOB message
    XaStart,                   // XA_START message
    XaJoin,                    // XA_JOIN message
    XAControl,                 // undocumented
    XAPrepare,                 // undocumented
    XARecover,                 // undocumented
}
impl ReplyType {
    pub fn from_i16(val: i16) -> HdbResult<ReplyType> {
        match val {
            0 => Ok(ReplyType::Nil),
            1 => Ok(ReplyType::Ddl),
            2 => Ok(ReplyType::Insert),
            3 => Ok(ReplyType::Update),
            4 => Ok(ReplyType::Delete),
            5 => Ok(ReplyType::Select),
            6 => Ok(ReplyType::SelectForUpdate),
            7 => Ok(ReplyType::Explain),
            8 => Ok(ReplyType::DbProcedureCall),
            9 => Ok(ReplyType::DbProcedureCallWithResult),
            10 => Ok(ReplyType::Fetch),
            11 => Ok(ReplyType::Commit),
            12 => Ok(ReplyType::Rollback),
            14 => Ok(ReplyType::Connect),
            15 => Ok(ReplyType::WriteLob),
            16 => Ok(ReplyType::ReadLob),
            // 17 -> Ping
            18 => Ok(ReplyType::Disconnect),
            19 => Ok(ReplyType::CloseCursor),
            20 => Ok(ReplyType::FindLob),
            // 21 ABAPSTREAM
            22 => Ok(ReplyType::XaStart),
            23 => Ok(ReplyType::XaJoin),
            // 24 -> ItabWrite
            25 => Ok(ReplyType::XAControl),
            26 => Ok(ReplyType::XAPrepare),
            27 => Ok(ReplyType::XARecover),
            _ => Err(HdbError::Impl(format!(
                "found unexpected value {} for ReplyType",
                val
            ))),
        }
    }

    pub fn to_i16(&self) -> i16 {
        match *self {
            ReplyType::Nil => 0,
            ReplyType::Ddl => 1,
            ReplyType::Insert => 2,
            ReplyType::Update => 3,
            ReplyType::Delete => 4,
            ReplyType::Select => 5,
            ReplyType::SelectForUpdate => 6,
            ReplyType::Explain => 7,
            ReplyType::DbProcedureCall => 8,
            ReplyType::DbProcedureCallWithResult => 9,
            ReplyType::Fetch => 10,
            ReplyType::Commit => 11,
            ReplyType::Rollback => 12,
            ReplyType::Connect => 14,
            ReplyType::WriteLob => 15,
            ReplyType::ReadLob => 16,
            ReplyType::Disconnect => 18,
            ReplyType::CloseCursor => 19,
            ReplyType::FindLob => 20,
            ReplyType::XaStart => 22,
            ReplyType::XaJoin => 23,
            ReplyType::XAControl => 25,
            ReplyType::XAPrepare => 26,
            ReplyType::XARecover => 27,
        }
    }
}
