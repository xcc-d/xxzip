// Public modules
pub mod compress;
pub mod extract;
pub mod list;
pub mod error;
pub mod utils;
pub mod cli;
//1
// GUI module is conditionally compiled
#[cfg(feature = "gui")]
pub mod gui; 