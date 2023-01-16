#[cfg(feature = "async")]
use crate::conn::AsyncAmConnCore;
#[cfg(feature = "sync")]
use crate::conn::SyncAmConnCore;

use crate::protocol::parts::{ReadLobReply, ReadLobRequest};
use crate::protocol::{Part, ReplyType, Request, RequestType, ServerUsage};
use crate::{HdbError, HdbResult};

// Note that requested_length and offset count either bytes (BLOB, CLOB), or 1-2-3-chars (NCLOB)
#[cfg(feature = "sync")]
pub(crate) fn sync_fetch_a_lob_chunk(
    am_conn_core: &mut SyncAmConnCore,
    locator_id: u64,
    offset: u64,
    length: u32,
    server_usage: &mut ServerUsage,
) -> HdbResult<(Vec<u8>, bool)> {
    let mut request = Request::new(RequestType::ReadLob, 0);
    let offset = offset + 1;
    request.push(Part::ReadLobRequest(ReadLobRequest::new(
        locator_id, offset, length,
    )));

    let reply = am_conn_core.send(request)?;
    reply.assert_expected_reply_type(ReplyType::ReadLob)?;

    let mut o_read_lob_reply = None;
    for part in reply.parts {
        match part {
            Part::ReadLobReply(read_lob_reply) => {
                if *read_lob_reply.locator_id() != locator_id {
                    return Err(HdbError::Impl("locator ids do not match"));
                }
                o_read_lob_reply = Some(read_lob_reply);
            }

            Part::StatementContext(stmt_ctx) => server_usage.update(
                stmt_ctx.server_processing_time(),
                stmt_ctx.server_cpu_time(),
                stmt_ctx.server_memory_usage(),
            ),
            _ => warn!("Unexpected part received - and ignored"),
        }
    }

    o_read_lob_reply
        .map(ReadLobReply::into_data_and_last)
        .ok_or_else(|| HdbError::Impl("fetching a lob chunk failed"))
}

// Note that requested_length and offset count either bytes (BLOB, CLOB), or 1-2-3-chars (NCLOB)
#[cfg(feature = "async")]
pub(crate) async fn async_fetch_a_lob_chunk(
    am_conn_core: &mut AsyncAmConnCore,
    locator_id: u64,
    offset: u64,
    length: u32,
    server_usage: &mut ServerUsage,
) -> HdbResult<(Vec<u8>, bool)> {
    let mut request = Request::new(RequestType::ReadLob, 0);
    let offset = offset + 1;
    request.push(Part::ReadLobRequest(ReadLobRequest::new(
        locator_id, offset, length,
    )));

    let reply = am_conn_core.send(request).await?;
    reply.assert_expected_reply_type(ReplyType::ReadLob)?;

    let mut o_read_lob_reply = None;
    for part in reply.parts {
        match part {
            Part::ReadLobReply(read_lob_reply) => {
                if *read_lob_reply.locator_id() != locator_id {
                    return Err(HdbError::Impl("locator ids do not match"));
                }
                o_read_lob_reply = Some(read_lob_reply);
            }

            Part::StatementContext(stmt_ctx) => server_usage.update(
                stmt_ctx.server_processing_time(),
                stmt_ctx.server_cpu_time(),
                stmt_ctx.server_memory_usage(),
            ),
            _ => warn!("Unexpected part received - and ignored"),
        }
    }

    o_read_lob_reply
        .map(ReadLobReply::into_data_and_last)
        .ok_or_else(|| HdbError::Impl("fetching a lob chunk failed"))
}
