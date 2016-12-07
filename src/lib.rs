//! Experimental native rust database driver for SAP HANA(TM).
//!
//! This crate compiles with rust stable, but for using the interesting part of its API
//! you need serde's serialization and deserialization. You either add
//!
//! ```ignore
//! #![feature(proc_macro)]
//!
//! #[macro_use]
//! extern crate serde_derive;
//! ```
//!
//! to your crate root and
//!
//! ```ignore
//! chrono = { version = "0.2", features = ["serde"] }
//! serde_derive ="*"
//! ```
//!
//! to your dependencies-list in Cargo.toml, and use rust nightly for compiling.
//! Or you go with the somewhat cumbersome
//! [workaround](https://serde.rs/codegen-stable.html)
//! described by serde to stick with rust stable.
//!
//! The reason for publishing this driver in its immature state is to
//! <b>demonstrate how [serde](https://serde.rs/)
//! can be used to simplify the API of such a database driver</b>.
//!
//! Concretely, we use serde to simplify the data exchange between your code and the driver,
//! both for input parameters to prepared statements
//! and for results that you get from the database:
//! there is no need to iterate over a complex resultset by rows and columns!
//!
//! This approach allows, in contrast to many ORM mapping variants, using
//! the full flexibility of SQL (projection lists, all kinds of joins, unions, etc, etc).
//! Whatever query you need, you just use it, and whatever result structure you need,
//! you just use a corresponding rust structure into which you deserialize the data.
//!
//! See
//! [code examples](code_examples/index.html)
//! for an overview.

#![warn(missing_docs)]

extern crate byteorder;
extern crate chrono;
extern crate crypto;

#[macro_use]
extern crate log;

extern crate rand;

extern crate serde;
extern crate vec_map;
extern crate user;

mod connection;
mod hdb_response;
mod hdb_error;
mod prepared_statement;
mod protocol;
mod rs_serde;

pub mod code_examples;

pub use connection::Connection;
pub use prepared_statement::PreparedStatement;
pub use hdb_response::{HdbResponse, HdbReturnValue};
pub use protocol::lowlevel::parts::resultset::ResultSet;
pub use hdb_error::{HdbError, HdbResult};

/// Types for describing metadata.
pub mod metadata {
    pub use protocol::lowlevel::parts::output_parameters::OutputParameters;
    pub use protocol::lowlevel::parts::parameter_metadata::{ParameterDescriptor, ParameterOption,
                                                            ParMode};
    pub use protocol::lowlevel::parts::resultset::factory::new_for_tests as new_resultset_for_tests;
    pub use protocol::lowlevel::parts::resultset_metadata::{FieldMetadata, ResultSetMetadata};
}


/// Types that are used within the content part of a ResultSet.
///
/// A ResultSet contains a sequence of Rows. A row is a sequence of HdbValues.
/// Some of the HdbValues are implemented using LongDate, BLOB, etc
///
pub mod types {
    pub use protocol::lowlevel::parts::lob::{BLOB, CLOB, new_clob_to_db};
    pub use protocol::lowlevel::parts::longdate::LongDate;
    pub use protocol::lowlevel::parts::resultset::Row;
    pub use protocol::lowlevel::parts::typed_value::TypedValue as HdbValue;
}
