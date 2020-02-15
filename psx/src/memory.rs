use crate::bios::{ BIOS };

use std::path::Path;

pub struct Memory
{
    bios: BIOS,
    ram: Vec<u8>
}

impl Memory
{
    pub fn new(bios_path: &Path) -> Self
    {
        Memory
        {
            bios: BIOS::new(bios_path),
            ram: vec![0; 0x1F000000]
        }
    }

    pub fn read8(&self, addr: u32) -> u8
    {
        println!("MEM read8 @ {:08x}", addr);
        // TODO check misaligned access

        match addr
        {
            0x00000000 ..= 0x1F000000 =>  self.read_ram8(addr), // TODO exclusive range
            0x1F000000 ..= 0x1F800000 => 0xFF, // fake license check

            0x80000000 ..= 0x9F000000 =>  self.read_ram8(addr - 0x80000000), // TODO exclusive range
            0xBFC00000 ..= 0xBFC80000 => self.bios.read8(addr - 0xBFC00000), // TODO exclusive range

            0xA0000000 ..= 0xA0200000 => self.bios.read8(addr - 0xA0000000), // TODO exclusive range

            _                         => panic!("Unsupported read8 address: {:08x}", addr)
        }
    }

    pub fn read(&self, addr: u32) -> u32
    {
        println!("MEM read @ {:08x}", addr);
        // TODO check misaligned access

        match addr
        {
            0x00000000 ..= 0x1F000000 =>  self.read_ram(addr), // TODO exclusive range
            0x1F801000 ..= 0x1F801078 => 0,

            0x80000000 ..= 0x9F000000 =>  self.read_ram(addr - 0x80000000), // TODO exclusive range

            0xA0000000 ..= 0xBF000000 => self.read_ram(addr - 0xA0000000), // TODO exclusive range
            0xBFC00000 ..= 0xBFC80000 => self.bios.read(addr - 0xBFC00000), // TODO exclusive range

            _                         => panic!("Unsupported read32 address: {:08x}", addr)
        }
    }

    pub fn write(&mut self, addr: u32, val: u32)
    {
        println!("MEM write {:08x} @ {:08x}", val, addr);
        // TODO check misaligned access

        match addr
        {
            0x00000000 ..= 0x1F000000 =>  self.write_ram(addr, val), // TODO exclusive range
            0x1F801000 ..= 0x1F801078 => println!("Ignoring IRQ control write"),

            0x80000000 ..= 0x9F000000 =>  self.write_ram(addr - 0x80000000, val), // TODO exclusive range

            //0x1F801000 ..= 0x1F801024 => {},
            //0x1F801060 ..= 0x1F801060 => {},
            0xA0000000 ..= 0xA0200000 =>  self.write_ram(addr - 0xA0000000, val), // TODO exclusive range
            0xFFFE0130 ..= 0xFFFE0130 => {},
            _                         => panic!("Unsupported write32 address: {:08x}", addr)
        }
    }

    pub fn write8(&mut self, addr: u32, val: u8)
    {
        println!("MEM write8 {:08x} @ {:08x}", val, addr);

        // TODO check misaligned access

        match addr
        {
            0x00000000 ..= 0x1F000000 =>  self.write_ram8(addr, val), // TODO exclusive range
            0x1F802000 ..= 0x1F802042 => println!("Ignored write to Expansion 2"),

            0x80000000 ..= 0x9F000000 =>  self.write_ram8(addr - 0x80000000, val), // TODO exclusive range

            0xA0000000 ..= 0xBF000000 =>  self.write_ram8(addr - 0xA0000000, val), // TODO exclusive range

            _                         => panic!("Unsupported write8 address: {:08x}", addr)
        }
    }

    pub fn write16(&mut self, addr: u32, val: u16)
    {
        // TODO check misaligned access

        match addr
        {
            0x1F801100 ..= 0x1F801130 => println!("Ignored write to the timer registers: {:08x} @ {:08x}", val, addr),
            0x1F801C00 ..= 0x1F802240 => println!("Ignored write to the SPU register: {:08x} @ {:08x}", val, addr),
            _                         => panic!("Unsupported write16 address: {:08x}", addr)
        }
    }

    fn read_ram8(&self, addr: u32) -> u8
    {
        self.ram[addr as usize]
    }

    fn read_ram(&self, addr: u32) -> u32
    {
        let offset = addr as usize;

        let b0 = self.ram[offset] as u32;
        let b1 = self.ram[offset + 1] as u32;
        let b2 = self.ram[offset + 2] as u32;
        let b3 = self.ram[offset + 3] as u32;

        (b3 << 24) | (b2 << 16) | (b1 << 8) | b0
    }

    fn write_ram8(&mut self, addr: u32, val: u8)
    {
        self.ram[addr as usize] = val
    }

    fn write_ram(&mut self, addr: u32, val: u32)
    {
        let offset = addr as usize;
        self.ram[offset] = val as u8;
        self.ram[offset + 1] = ((val & 0xFF00) >> 8) as u8;
        self.ram[offset + 2] = ((val & 0xFF0000) >> 16) as u8;
        self.ram[offset + 3] = ((val & 0xFF000000) >> 24) as u8;
    }
}
