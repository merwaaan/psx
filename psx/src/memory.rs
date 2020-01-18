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
        // TODO check misaligned access

        match addr
        {
            0xBFC00000 ..= 0xBFC00400 => self.bios.read(addr - 0xBFC00000), // TODO exclusive range
            _                         => panic!("Unsupported read address: {:08x}", addr)
        }
    }

    pub fn write(&self, addr: u32, val: u32)
    {
        // TODO check misaligned access

        match addr
        {
            0x1F801000 ..= 0x1F801024 => {},
            0x1F801060 ..= 0x1F801060 => {},
            0xFFFE0130 ..= 0xFFFE0130 => {},
            _                         => panic!("Unsupported write address: {:08x}", addr)
        }
    }
}
