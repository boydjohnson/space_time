#![no_std]
#![feature(trait_alias)]
#![deny(missing_docs)]
//! Library for representing geometric shapes as strings using space filling curves.

pub mod binned_time;

#[cfg(test)]
extern crate quickcheck;

#[cfg(test)]
#[macro_use]
extern crate quickcheck_macros;
