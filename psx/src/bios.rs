// TODO load as u32? misaligned rw possible?

use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct BIOS
{
    data: Vec<u8> // TODO 32?
}

impl BIOS
{
    pub fn new(path: &Path) -> Self
    {
        println!("Loading BIOS: \"{}\"", path.display());

        let mut buffer = Vec::new();

        let mut file = File::open(path).unwrap(); //TODO err
        file.read_to_end(&mut buffer).unwrap(); // TODO err

        BIOS
        {
            data: buffer
        }
    }

    pub fn read8(&self, addr: u32) -> u8
    {
        println!("BIOS read8 @ {:08x}", addr);

        let offset = addr as usize;
        self.data[offset]
    }

    pub fn read(&self, addr: u32) -> u32
    {
        println!("BIOS read @ {:08x}", addr);

        let offset = addr as usize;

        let b0 = self.data[offset] as u32;
        let b1 = self.data[offset + 1] as u32;
        let b2 = self.data[offset + 2] as u32;
        let b3 = self.data[offset + 3] as u32;

        (b3 << 24) | (b2 << 16) | (b1 << 8) | b0
    }
}