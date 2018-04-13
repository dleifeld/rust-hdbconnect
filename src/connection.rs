use protocol::lowlevel::parts::server_error::ServerError;
use {HdbError, HdbResponse, HdbResult};
use connect_params::ConnectParams;

use prepared_statement::PreparedStatement;
use prepared_statement::factory as PreparedStatementFactory;

use protocol::authenticate;
use protocol::lowlevel::argument::Argument;
use protocol::lowlevel::message::Request;
use protocol::lowlevel::request_type::RequestType;
use protocol::lowlevel::part::Part;
use protocol::lowlevel::partkind::PartKind;
use protocol::lowlevel::conn_core::{AmConnCore, ConnectionCore};
use protocol::lowlevel::parts::resultset::ResultSet;
use xa_impl::new_resource_manager;

use chrono::Local;
use dist_tx::rm::ResourceManager;
use std::error::Error;
use std::net::TcpStream;
use std::fmt::Write;
use std::sync::Arc;

/// Connection object.
///
/// The connection to the database.
///
/// # Example
///
/// ```ignore
/// use hdbconnect::{Connection, IntoConnectParams};
/// let params = "hdbsql://my_user:my_passwd@the_host:2222"
///     .into_connect_params()
///     .unwrap();
/// let mut connection = Connection::new(params).unwrap();
/// ```
#[derive(Debug)]
pub struct Connection {
    params: ConnectParams,
    am_conn_core: AmConnCore,
}
#[allow(unknown_lints)]
#[allow(unit_arg)]
impl Connection {
    /// Factory method for authenticated connections.
    pub fn new(params: ConnectParams) -> HdbResult<Connection> {
        trace!("Entering connect()");
        let start = Local::now();

        let mut connect_string = String::with_capacity(200);
        write!(connect_string, "{}:{}", params.hostname(), params.port())?;

        trace!("Connecting to \"{}\"", connect_string);
        let tcp_stream = TcpStream::connect(&connect_string as &str)?;
        trace!("tcp_stream is open");

        let mut am_conn_core = ConnectionCore::initialize(tcp_stream)?;
        debug!(
            "connection to {} is initialized ({} µs)",
            connect_string,
            Local::now()
                .signed_duration_since(start)
                .num_microseconds()
                .unwrap_or(-1)
        );

        let a = authenticate::user_pw(&mut (am_conn_core), params.dbuser(), params.password());
        debug!("auth: {:?}", a);
        a?;

        debug!(
            "user \"{}\" successfully logged on ({} µs)",
            params.dbuser(),
            Local::now()
                .signed_duration_since(start)
                .num_microseconds()
                .unwrap_or(-1)
        );
        Ok(Connection {
            params,
            am_conn_core,
        })
    }

    /// Returns the DB which was used to authenticate at HANA.
    pub fn get_user(&self) -> &str {
        self.params.dbuser()
    }

    /// Returns the HANA's product version info.
    pub fn get_major_and_minor_product_version(&self) -> HdbResult<(i8, i16)> {
        Ok(self.am_conn_core
            .lock()?
            .get_major_and_minor_product_version())
    }

    /// Sets the connection's auto-commit behavior for future calls.
    pub fn set_auto_commit(&mut self, ac: bool) -> HdbResult<()> {
        Ok(self.am_conn_core.lock()?.set_auto_commit(ac))
    }

    /// Returns the connection's auto-commit behavior.
    pub fn is_auto_commit(&self) -> HdbResult<bool> {
        Ok(self.am_conn_core.lock()?.is_auto_commit())
    }

    /// Configures the connection's fetch size for future calls.
    pub fn set_fetch_size(&mut self, fetch_size: u32) -> HdbResult<()> {
        Ok(self.am_conn_core.lock()?.set_fetch_size(fetch_size))
    }
    /// Configures the connection's lob read length for future calls.
    pub fn get_lob_read_length(&self) -> HdbResult<i32> {
        Ok(self.am_conn_core.lock()?.get_lob_read_length())
    }
    /// Configures the connection's lob read length for future calls.
    pub fn set_lob_read_length(&mut self, lob_read_length: i32) -> HdbResult<()> {
        Ok(self.am_conn_core
            .lock()?
            .set_lob_read_length(lob_read_length))
    }

    /// Returns the number of roundtrips to the database that
    /// have been done through this connection.
    pub fn get_call_count(&self) -> HdbResult<i32> {
        Ok(self.am_conn_core.lock()?.last_seq_number())
    }

    /// Executes a statement on the database.
    ///
    /// This generic method can handle all kinds of calls,
    /// and thus has the most complex return type.
    /// In many cases it will be more appropriate to use
    /// one of the methods query(), dml(), exec(), which have the
    /// adequate simple result type you usually want.
    pub fn statement(&mut self, stmt: &str) -> HdbResult<HdbResponse> {
        execute(&mut self.am_conn_core, String::from(stmt))
    }

    /// Executes a statement and expects a single ResultSet.
    pub fn query(&mut self, stmt: &str) -> HdbResult<ResultSet> {
        self.statement(stmt)?.into_resultset()
    }
    /// Executes a statement and expects a single number of affected rows.
    pub fn dml(&mut self, stmt: &str) -> HdbResult<usize> {
        let vec = &(self.statement(stmt)?.into_affected_rows()?);
        match vec.len() {
            1 => Ok(vec[0]),
            _ => Err(HdbError::Usage(
                "number of affected-rows-counts <> 1".to_owned(),
            )),
        }
    }
    /// Executes a statement and expects a plain success.
    pub fn exec(&mut self, stmt: &str) -> HdbResult<()> {
        self.statement(stmt)?.into_success()
    }

    /// Prepares a statement and returns a handle to it.
    /// Note that the handle keeps using the same connection.
    pub fn prepare(&self, stmt: &str) -> HdbResult<PreparedStatement> {
        Ok(PreparedStatementFactory::prepare(
            Arc::clone(&self.am_conn_core),
            String::from(stmt),
        )?)
    }

    /// Commits the current transaction.
    pub fn commit(&mut self) -> HdbResult<()> {
        self.statement("commit")?.into_success()
    }

    /// Rolls back the current transaction.
    pub fn rollback(&mut self) -> HdbResult<()> {
        self.statement("rollback")?.into_success()
    }

    /// Creates a new connection object with the same settings and
    /// authentication.
    pub fn spawn(&self) -> HdbResult<Connection> {
        let mut other_conn = Connection::new(self.params.clone())?;
        {
            let am_conn_core = self.am_conn_core.lock()?;
            other_conn.set_auto_commit(am_conn_core.is_auto_commit())?;
            other_conn.set_fetch_size(am_conn_core.get_fetch_size())?;
            other_conn.set_lob_read_length(am_conn_core.get_lob_read_length())?;
        }
        Ok(other_conn)
    }

    /// Utility method to fire a couple of statements, ignoring errors and
    /// return values
    pub fn multiple_statements_ignore_err(&mut self, stmts: Vec<&str>) {
        for s in stmts {
            debug!("multiple_statements_ignore_err: firing \"{}\"", s);
            let result = self.statement(s);
            match result {
                Ok(_) => {}
                Err(e) => info!("Error intentionally ignored: {}", e.description()),
            }
        }
    }

    /// Utility method to fire a couple of statements, ignoring their return values;
    /// the method returns with the first error, or with  ()
    pub fn multiple_statements(&mut self, stmts: Vec<&str>) -> HdbResult<()> {
        for s in stmts {
            self.statement(s)?;
        }
        Ok(())
    }

    /// Returns warnings that were returned from the server since the last call to this
    /// method.
    pub fn pop_warnings(&self) -> HdbResult<Option<Vec<ServerError>>> {
        self.am_conn_core.lock()?.pop_warnings()
    }

    /// Returns an implementation of `dist_tx::rm::ResourceManager` that is based on this
    /// connection.
    pub fn get_resource_manager(&self) -> Box<ResourceManager> {
        Box::new(new_resource_manager(Arc::clone(&self.am_conn_core)))
    }
}

fn execute(am_conn_core: &mut AmConnCore, stmt: String) -> HdbResult<HdbResponse> {
    debug!("connection::execute({})", stmt);
    let command_options = 0b_1000;
    let fetch_size: u32 = { am_conn_core.lock()?.get_fetch_size() };
    let mut request = Request::new(RequestType::ExecuteDirect, command_options);
    request.push(Part::new(
        PartKind::FetchSize,
        Argument::FetchSize(fetch_size),
    ));
    request.push(Part::new(PartKind::Command, Argument::Command(stmt)));
    request.send_and_get_response(None, None, am_conn_core, None)
}
