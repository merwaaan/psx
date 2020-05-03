// TODO load as u32? misaligned rw possible?

use crate::memory::Addressable;
use crate::memory_segment::MemorySegment;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

pub struct BIOS
{
    data: MemorySegment
}

impl BIOS
{
    pub fn new(path: PathBuf) -> Self
    {
        println!("Loading BIOS: \"{}\"", path.display());

        let mut buffer = Vec::new();

        let mut file = File::open(path).unwrap(); //TODO err
        file.read_to_end(&mut buffer).unwrap(); // TODO err

        BIOS
        {
            data: MemorySegment::from_buffer(buffer)
        }
    }

    pub fn read<T: Addressable>(&self, address: u32) -> T
    {
        self.data.read(address)
    }
}