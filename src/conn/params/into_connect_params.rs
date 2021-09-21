use crate::{ConnectParams, ConnectParamsBuilder, HdbError, HdbResult};
use url::Url;

/// A trait implemented by types that can be converted into a `ConnectParams`.
pub trait IntoConnectParams {
    /// Converts the value of `self` into a `ConnectParams`.
    ///
    /// # Errors
    ///
    /// `HdbError::Usage` if not enough or inconsistent information was provided
    fn into_connect_params(self) -> HdbResult<ConnectParams>;
}

impl IntoConnectParams for ConnectParams {
    fn into_connect_params(self) -> HdbResult<ConnectParams> {
        Ok(self)
    }
}

impl IntoConnectParams for &ConnectParams {
    fn into_connect_params(self) -> HdbResult<ConnectParams> {
        Ok(self.clone())
    }
}

impl IntoConnectParams for ConnectParamsBuilder {
    fn into_connect_params(self) -> HdbResult<ConnectParams> {
        self.build()
    }
}

impl IntoConnectParams for &ConnectParamsBuilder {
    fn into_connect_params(self) -> HdbResult<ConnectParams> {
        self.build()
    }
}

impl<'a> IntoConnectParams for &'a str {
    fn into_connect_params(self) -> HdbResult<ConnectParams> {
        Url::parse(self)
            .map_err(|e| HdbError::conn_params(Box::new(e)))?
            .into_connect_params()
    }
}

impl IntoConnectParams for String {
    fn into_connect_params(self) -> HdbResult<ConnectParams> {
        self.as_str().into_connect_params()
    }
}

impl IntoConnectParams for Url {
    fn into_connect_params(self) -> HdbResult<ConnectParams> {
        let builder = crate::IntoConnectParamsBuilder::into_connect_params_builder(self)?;
        builder.build()
    }
}
