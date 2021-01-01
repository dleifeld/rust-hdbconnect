use crate::conn::AmConnCore;
use crate::protocol::parts::XatOptions;
use crate::protocol::{Part, PartKind, Reply, Request, RequestType};
use crate::{HdbError, HdbResult};
use dist_tx::rm::{CResourceManager, CRmWrapper, ErrorCode, Flags, RmError, RmRc, RmResult};
use dist_tx::tm::XaTransactionId;

/// Handle for dealing with distributed transactions that is to be used by a
/// transaction manager.
///
/// Is based on the connection from which it is obtained
/// (see [`Connection::get_resource_manager`](crate::Connection::get_resource_manager)).
///
#[derive(Debug)]
pub(crate) struct HdbCResourceManager {
    am_conn_core: AmConnCore,
}

pub(crate) fn new_resource_manager(am_conn_core: AmConnCore) -> CRmWrapper<HdbCResourceManager> {
    CRmWrapper(HdbCResourceManager { am_conn_core })
}

impl CResourceManager for HdbCResourceManager {
    fn start(&mut self, id: &XaTransactionId, flags: Flags) -> RmResult<RmRc> {
        debug!("CResourceManager::start()");
        if !flags.contains_only(Flags::JOIN | Flags::RESUME) {
            return Err(usage_error("start", flags));
        }

        // These two seem redundant: the server has to know this anyway
        // error if self.isDistributedTransaction()
        // error if self.is_xat_in_progress()

        // TODO: xa seems only to work on primary!!
        // ClientConnectionID ccid = getPrimaryConnection();

        self.xa_send_receive(RequestType::XAStart, id, flags)
    }

    fn end(&mut self, id: &XaTransactionId, flags: Flags) -> RmResult<RmRc> {
        debug!("CResourceManager::end()");
        if !flags.contains_only(Flags::SUCCESS | Flags::FAIL | Flags::SUSPEND) {
            return Err(usage_error("end", flags));
        }

        self.xa_send_receive(RequestType::XAEnd, id, flags)
    }

    fn prepare(&mut self, id: &XaTransactionId) -> RmResult<RmRc> {
        debug!("CResourceManager::prepare()");
        self.xa_send_receive(RequestType::XAPrepare, id, Flags::empty())
    }

    fn commit(&mut self, id: &XaTransactionId, flags: Flags) -> RmResult<RmRc> {
        debug!("CResourceManager::commit()");
        if !flags.contains_only(Flags::ONE_PHASE) {
            return Err(usage_error("commit", flags));
        }
        self.xa_send_receive(RequestType::XACommit, id, flags)
    }

    fn rollback(&mut self, id: &XaTransactionId) -> RmResult<RmRc> {
        debug!("CResourceManager::rollback()");
        self.xa_send_receive(RequestType::XARollback, id, Flags::empty())
    }

    fn forget(&mut self, id: &XaTransactionId) -> RmResult<RmRc> {
        debug!("CResourceManager::forget()");
        self.xa_send_receive(RequestType::XAForget, id, Flags::empty())
    }

    fn recover(&mut self, flags: Flags) -> RmResult<Vec<XaTransactionId>> {
        debug!("HdbCResourceManager::recover()");
        if !flags.contains_only(Flags::START_RECOVERY_SCAN | Flags::END_RECOVERY_SCAN) {
            return Err(usage_error("recover", flags));
        }

        let mut request = Request::new(RequestType::XARecover, 0);

        let mut xat_options = XatOptions::default();
        xat_options.set_flags(flags);
        request.push(Part::XatOptions(xat_options));

        let mut reply: Reply = self.am_conn_core.send_sync(request)?;
        while !reply.parts.is_empty() {
            reply.parts.drop_parts_of_kind(PartKind::StatementContext);
            match reply.parts.pop() {
                Some(Part::XatOptions(xat_options)) => {
                    return Ok(xat_options.get_transactions()?);
                }
                Some(part) => warn!("recover: found unexpected part {:?}", part),
                None => warn!("recover: did not find next part"),
            }
        }

        Err(RmError::new(
            ErrorCode::ProtocolError,
            "recover did not get a list of xids, not even an empty one".to_owned(),
        ))
    }
}

fn usage_error(method: &'static str, flags: Flags) -> RmError {
    RmError::new(
        ErrorCode::ProtocolError,
        format!(
            "CResourceManager::{}(): Invalid transaction flags {:?}",
            method, flags
        ),
    )
}

// only few seem to be used by HANA
fn error_code_from_hana_code(code: i32) -> ErrorCode {
    match code {
        210 => ErrorCode::DuplicateTransactionId,
        211 => ErrorCode::InvalidArguments,
        212 => ErrorCode::InvalidTransactionId,
        214 => ErrorCode::ProtocolError,
        215 => ErrorCode::RmError,
        216 => ErrorCode::RmFailure,
        i => ErrorCode::UnknownErrorCode(i),
    }
}

impl HdbCResourceManager {
    fn xa_send_receive(
        &mut self,
        request_type: RequestType,
        id: &XaTransactionId,
        flag: Flags,
    ) -> RmResult<RmRc> {
        self.xa_send_receive_impl(request_type, id, flag)
            .map(|opt| opt.unwrap_or(RmRc::Ok))
            .map_err(|hdb_error| {
                if let Some(ref server_error) = hdb_error.server_error() {
                    return RmError::new(
                        error_code_from_hana_code(server_error.code()),
                        server_error.text().to_string(),
                    );
                } else if let HdbError::ExecutionResults(_) = hdb_error {
                    return RmError::new(
                        ErrorCode::RmError,
                        "HdbError::ExecutionResults".to_string(),
                    );
                };
                From::<HdbError>::from(hdb_error)
            })
    }

    fn xa_send_receive_impl(
        &mut self,
        request_type: RequestType,
        id: &XaTransactionId,
        flags: Flags,
    ) -> HdbResult<Option<RmRc>> {
        if self.am_conn_core.lock()?.is_auto_commit() {
            return Err(HdbError::Usage(
                "xa_*() not possible, connection is set to auto_commit",
            ));
        }

        let mut xat_options = XatOptions::default();
        xat_options.set_xatid(id);
        if !flags.is_empty() {
            xat_options.set_flags(flags);
        }

        let mut request = Request::new(request_type, 0);
        request.push(Part::XatOptions(xat_options));

        let mut reply = self.am_conn_core.send_sync(request)?;

        reply.parts.drop_parts_of_kind(PartKind::StatementContext);
        if let Some(Part::XatOptions(xat_options)) = reply.parts.pop_if_kind(PartKind::XatOptions) {
            debug!("received xat_options: {:?}", xat_options);
            return Ok(xat_options.get_returncode());
        }
        Ok(None)
    }
}
