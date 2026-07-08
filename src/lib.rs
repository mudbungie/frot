//! frot — take an impression of a web page.
//!
//! See `VISION.md`. The library exposes the envelope schema, the CLI parser,
//! the DOM facade, the views, the HTTP fetcher, and a top-level [`run`] entry
//! point that the binary delegates to.

pub mod ax;
pub mod cli;
pub mod css;
pub mod dom;
pub mod envelope;
pub mod fetch;
pub mod needs;
pub mod run;
pub mod tags;
pub mod views;

pub use run::run;
