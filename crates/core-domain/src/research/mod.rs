//! Research domain primitives.
//!
//! This module holds pure, stateless research-domain helpers that are used by
//! multiple consumers (report-builder, app-service, CLI).  They do not fetch
//! data and they do not depend on presentation concerns.

pub mod breadth;
pub mod classification;
pub mod percentile;
pub mod rotation;
pub mod signal;
pub mod stretch;
