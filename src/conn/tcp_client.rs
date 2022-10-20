mod plain_sync_tcp_client;
mod tls_sync_tcp_client;

use crate::ConnectParams;
use plain_sync_tcp_client::PlainSyncTcpClient;
use std::time::Instant;
use tls_sync_tcp_client::TlsSyncTcpClient;

// A buffered tcp connection, with or without TLS.
#[derive(Debug)]
pub(crate) enum TcpClient {
    // A buffered tcp connection without TLS.
    SyncPlain(PlainSyncTcpClient),
    // A buffered blocking tcp connection with TLS.
    SyncTls(TlsSyncTcpClient),
}
impl TcpClient {
    // Constructs a buffered tcp connection, with or without TLS,
    // depending on the given connect parameters.
    pub fn try_new(params: ConnectParams) -> std::io::Result<Self> {
        let start = Instant::now();
        trace!("TcpClient: Connecting to {:?})", params.addr());

        let tcp_conn = if params.is_tls() {
            Self::SyncTls(TlsSyncTcpClient::try_new(params)?)
        } else {
            Self::SyncPlain(PlainSyncTcpClient::try_new(params)?)
        };

        trace!(
            "Connection of type {} is initialized ({} µs)",
            tcp_conn.s_type(),
            Instant::now().duration_since(start).as_micros(),
        );
        Ok(tcp_conn)
    }

    // Returns a descriptor of the chosen type
    pub fn s_type(&self) -> &'static str {
        match self {
            Self::SyncPlain(_) => "Plain TCP",
            Self::SyncTls(_) => "TLS",
        }
    }

    pub fn connect_params(&self) -> &ConnectParams {
        match self {
            Self::SyncPlain(client) => client.connect_params(),
            Self::SyncTls(client) => client.connect_params(),
        }
    }
}
impl Drop for TcpClient {
    fn drop(&mut self) {
        trace!("Drop of TcpClient");
    }
}
