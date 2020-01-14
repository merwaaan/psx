use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct BIOS
{
    data: [u8; 512] // TODO 32?
}

impl BIOS
{
    pub fn new(path: &Path) -> Self
    {
        let mut buffer = [0; 512];

        let mut file = File::open(path).unwrap(); //TODO err
        file.read_exact(&mut buffer).unwrap(); // TODO err

        BIOS
        {
            data: buffer
        }
    }
}