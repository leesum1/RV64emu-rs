#![cfg_attr(not(feature = "std"), no_std)]
#[macro_use]
extern crate alloc;

pub mod device;
pub mod difftest;
pub mod rv64core;
pub mod rvsim;
pub mod tools;
pub mod config;

#[cfg(feature = "rv_debug_trace")]
pub mod trace;


// cfg_if::cfg_if! {
//     if #[cfg(feature = "alloc")] {
//         #[macro_use]
//         extern crate alloc;
//         pub mod device;
//         pub mod difftest;
//         pub mod rv64core;
//         pub mod rvsim;
//         pub mod tools;
//         #[cfg(feature = "rv_debug_trace")]
//         pub mod trace;
//     }
// }
