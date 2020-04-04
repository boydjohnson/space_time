#![no_std]
#![feature(trait_alias)]
#![deny(missing_docs)]
//! Port of the scala-based geomesa-z3 library(http://github.com/locationtech/geomesa)

pub mod binned_time;
pub mod normalized_dimension;

#[cfg(test)]
extern crate quickcheck;

#[cfg(test)]
#[macro_use]
extern crate quickcheck_macros;
