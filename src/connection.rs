use crate::authentication;
use crate::conn_core::connect_params::ConnectParams;
use crate::conn_core::{AmConnCore, ConnectionCore};
use crate::prepared_statement::PreparedStatement;
use crate::protocol::argument::Argument;
use crate::protocol::part::Part;
use crate::protocol::partkind::PartKind;
use crate::protocol::parts::command_info::CommandInfo;
use crate::protocol::parts::resultset::ResultSet;
use crate::protocol::parts::server_error::ServerError;
use crate::protocol::reply::SkipLastSpace;
use crate::protocol::request::Request;
use crate::protocol::request_type::RequestType;
use crate::protocol::server_resource_consumption_info::ServerResourceConsumptionInfo;
use crate::{HdbError, HdbResponse, HdbResult};
use crate::into_string::IntoString;

use chrono::Local;
use dist_tx::rm::ResourceManager;
use std::error::Error;
use std::sync::Arc;
use crate::xa_impl::new_resource_manager;

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

impl Connection {
    /// Factory method for authenticated connections.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(params: ConnectParams) -> HdbResult<Connection> {
        trace!("Entering connect()");
        let start = Local::now();

        let mut am_conn_core = ConnectionCore::initialize(params.clone())?;

        authentication::authenticate(
            &mut (am_conn_core),
            params.dbuser(),
            params.password(),
            params.clientlocale(),
        )?;

        {
            let guard = am_conn_core.lock()?;
            debug!(
                "user \"{}\" successfully logged on ({} µs) to {} of {} (HANA version: {})",
                params.dbuser(),
                Local::now()
                    .signed_duration_since(start)
                    .num_microseconds()
                    .unwrap_or(-1),
                guard.get_database_name(),
                guard.get_system_id(),
                guard.get_full_version_string()
            );
        }
        Ok(Connection {
            params,
            am_conn_core,
        })
    }

    /// Sets the connection's auto-commit behavior for future calls.
    pub fn set_auto_commit(&mut self, ac: bool) -> HdbResult<()> {
        self.am_conn_core.lock()?.set_auto_commit(ac);
        Ok(())
    }

    /// Returns the connection's auto-commit behavior.
    pub fn is_auto_commit(&self) -> HdbResult<bool> {
        Ok(self.am_conn_core.lock()?.is_auto_commit())
    }

    /// Configures the connection's fetch size for future calls.
    pub fn set_fetch_size(&mut self, fetch_size: u32) -> HdbResult<()> {
        self.am_conn_core.lock()?.set_fetch_size(fetch_size);
        Ok(())
    }
    /// Configures the connection's lob read length for future calls.
    pub fn get_lob_read_length(&self) -> HdbResult<i32> {
        Ok(self.am_conn_core.lock()?.get_lob_read_length())
    }
    /// Configures the connection's lob read length for future calls.
    pub fn set_lob_read_length(&mut self, l: i32) -> HdbResult<()> {
        self.am_conn_core.lock()?.set_lob_read_length(l);
        Ok(())
    }

    ///
    pub fn get_server_resource_consumption_info(&self) -> HdbResult<ServerResourceConsumptionInfo> {
        Ok(self
            .am_conn_core
            .lock()?
            .server_resource_consumption_info()
            .clone())
    }

    /// Returns the number of roundtrips to the database that
    /// have been done through this connection.
    pub fn get_call_count(&self) -> HdbResult<i32> {
        Ok(self.am_conn_core.lock()?.last_seq_number())
    }

    /// Sets client information into a session variable on the server.
    ///
    /// Example:
    ///
    /// ```ignore
    /// connection.set_application_user("K2209657")?;
    /// ```
    pub fn set_application_user(&self, appl_user: &str) -> HdbResult<()> {
        self.am_conn_core.lock()?.set_application_user(appl_user)
    }

    /// Sets client information into a session variable on the server.
    ///
    /// Example:
    ///
    /// ```ignore
    /// connection.set_application_version("5.3.23")?;
    /// ```
    pub fn set_application_version(&mut self, version: &str) -> HdbResult<()> {
        self.am_conn_core.lock()?.set_application_version(version)
    }

    /// Sets client information into a session variable on the server.
    ///
    /// Example:
    ///
    /// ```ignore
    /// connection.set_application_source("5.3.23","update_customer.rs")?;
    /// ```
    pub fn set_application_source(&mut self, source: &str) -> HdbResult<()> {
        self.am_conn_core.lock()?.set_application_source(source)
    }

    /// Executes a statement on the database.
    ///
    /// This generic method can handle all kinds of calls,
    /// and thus has the most complex return type.
    /// In many cases it will be more appropriate to use
    /// one of the methods query(), dml(), exec(), which have the
    /// adequate simple result type you usually want.
    pub fn statement<I: IntoString>(&mut self, stmt: I) -> HdbResult<HdbResponse> {
        execute(&mut self.am_conn_core, stmt.into(), None)
    }

    /// Executes a statement and expects a single ResultSet.
    pub fn query<I: IntoString>(&mut self, stmt: I) -> HdbResult<ResultSet> {
        self.statement(stmt)?.into_resultset()
    }

    /// Executes a statement and expects a single number of affected rows.
    pub fn dml<I: IntoString>(&mut self, stmt: I) -> HdbResult<usize> {
        let vec = &(self.statement(stmt)?.into_affected_rows()?);
        match vec.len() {
            1 => Ok(vec[0]),
            _ => Err(HdbError::Usage(
                "number of affected-rows-counts <> 1".to_owned(),
            )),
        }
    }

    /// Executes a statement and expects a plain success.
    pub fn exec<I: IntoString>(&mut self, stmt: I) -> HdbResult<()> {
        self.statement(stmt)?.into_success()
    }

    /// Prepares a statement and returns a handle to it.
    ///
    /// Note that the handle keeps using the same connection.
    pub fn prepare(&self, stmt: &str) -> HdbResult<PreparedStatement> {
        Ok(PreparedStatement::try_new(
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
    pub fn multiple_statements_ignore_err<S: IntoString + AsRef<str>>(&mut self, stmts: Vec<S>) {
        for s in stmts {
            trace!("multiple_statements_ignore_err: firing \"{}\"", s.as_ref());
            let result = self.statement(s);
            match result {
                Ok(_) => {}
                Err(e) => debug!("Error intentionally ignored: {}", e.description()),
            }
        }
    }

    /// Utility method to fire a couple of statements, ignoring their return
    /// values; the method returns with the first error, or with  ()
    pub fn multiple_statements<S: IntoString>(&mut self, stmts: Vec<S>) -> HdbResult<()> {
        for s in stmts {
            self.statement(s)?;
        }
        Ok(())
    }

    /// Returns warnings that were returned from the server since the last call
    /// to this method.
    pub fn pop_warnings(&self) -> HdbResult<Option<Vec<ServerError>>> {
        self.am_conn_core.lock()?.pop_warnings()
    }

    /// Returns an implementation of `dist_tx::rm::ResourceManager` that is
    /// based on this connection.
    pub fn get_resource_manager(&self) -> Box<ResourceManager> {
        Box::new(new_resource_manager(Arc::clone(&self.am_conn_core)))
    }

    /// Tools like debuggers can provide additional information while stepping through a source
    pub fn execute_with_debuginfo(
        &mut self,
        stmt: &str,
        module: &str,
        line: i32,
    ) -> HdbResult<HdbResponse> {
        execute(
            &mut self.am_conn_core,
            String::from(stmt),
            Some(CommandInfo::new(line, module)),
        )
    }
}

fn execute(
    am_conn_core: &mut AmConnCore,
    stmt: String,
    o_ci: Option<CommandInfo>,
) -> HdbResult<HdbResponse> {
    debug!("connection::execute({})", stmt);
    let command_options = 0b_1000;
    let fetch_size: u32 = { am_conn_core.lock()?.get_fetch_size() };
    let mut request = Request::new(RequestType::ExecuteDirect, command_options);
    request.push(Part::new(
        PartKind::FetchSize,
        Argument::FetchSize(fetch_size),
    ));
    if let Option::Some(ci) = o_ci {
        request.push(Part::new(PartKind::CommandInfo, Argument::CommandInfo(ci)));
    }
    {
        let mut guard = am_conn_core.lock()?;
        if guard.is_client_info_touched() {
            request.push(Part::new(
                PartKind::ClientInfo,
                Argument::ClientInfo(guard.get_client_info_for_sending()),
            ));
        }
    }

    request.push(Part::new(PartKind::Command, Argument::Command(stmt)));
    request.send_and_get_hdbresponse(None, None, am_conn_core, None, SkipLastSpace::Soft)
}
