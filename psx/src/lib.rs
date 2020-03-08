pub mod psx;
pub mod opcode;

mod bios;
mod cpu;
mod dma;
mod debugger;
mod gpu;
mod memory;
mod ram;
mod spu;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate bitfield;
