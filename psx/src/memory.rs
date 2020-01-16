use crate::bios::{ BIOS };

use std::path::Path;

pub struct Memory
{
    pub bios: BIOS
}

impl Memory
{
    pub fn new(biosPath: &Path) -> Self
    {
        Memory
        {
            bios: BIOS::new(biosPath)
        }
    }

    pub fn read(&self, addr: u32) -> u32
    {
        match addr
        {
            0xBFC00000 ..= 0xBFC00200 => self.bios.read(addr - 0xBFC00000), // TODO exclusive range
            _                         => panic!("Unsupported address: {:08x}", addr)
        }
    }
}
