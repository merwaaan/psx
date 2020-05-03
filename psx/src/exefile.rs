use crate::cpu::CPU;
use crate::memory::Memory;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

// Documentation
//
// https://problemkaputt.de/psx-spx.htm#cdromfileformats
// http://www.emulatronia.com/doctec/consolas/psx/exeheader.txt

pub struct ExeFile
{
    data: Vec<u8>
}

impl ExeFile
{
    pub fn new_from_file(path: PathBuf) -> Self
    {
        println!("Loading EXE \"{}\"", path.display());

        let mut data = Vec::new();
        let mut file = File::open(path).unwrap(); //TODO err
        file.read_to_end(&mut data).unwrap(); // TODO err

        let exe = ExeFile { data };

        //

        if exe.id() != [0x50, 0x53, 0x2D, 0x58, 0x20, 0x45, 0x58, 0x45] // "PS-X EXE"
        {
            panic!("EXE header does not start with \"PSX-EXE\"");
        }

        exe
    }

    pub fn load(&self, cpu: &mut CPU, mem: &mut Memory)
    {
        println!("Zeroing @ {:08X}, size = {:08X}", self.memfill_address(), self.memfill_size());

        if self.memfill_size() > 0
        {
            panic!("EXE memfill not implemented");
        }

        // TODO make sure it's aligned?
        /*
        for address in self.memfill_address() .. self.memfill_address() + self.memfill_size()
        {
            mem.write8(address, self.data[address as usize]);
        }
        */

        let destination = self.destination_address();

        println!("Copying data @ {:08X}, size = {:08X}", self.destination_address(), self.destination_size());

        // TODO make sure it's aligned?
        for offset in 0 .. self.destination_size()
        {
            mem.write::<u8>(destination + offset, self.data[0x800 + offset as usize]);
        }

        cpu.next_pc = self.pc();

        println!("new PC @ {:08X}", cpu.pc);

        cpu.set_reg(28, self.gp());

        if self.sp_address() != 0
        {
            let sp = self.sp_address() + self.sp_size();
            cpu.set_reg(29, sp);
            cpu.set_reg(30, sp);
        }
    }

    fn word(&self, address: u32) -> u32
    {
        let offset = address as usize;

        (self.data[offset + 0] as u32) |
        (self.data[offset + 1] as u32) << 8 |
        (self.data[offset + 2] as u32) << 16 |
        (self.data[offset + 3] as u32) << 24
    }

    pub fn id(&self) -> &[u8] { &self.data[0..8] }

    pub fn pc(&self) -> u32 { self.word(0x10) }
    pub fn gp(&self) -> u32 { self.word(0x14) }

    pub fn destination_address(&self) -> u32 { self.word(0x18) }
    pub fn destination_size(&self) -> u32 { self.word(0x1C) }

    pub fn memfill_address(&self) -> u32 { self.word(0x28) }
    pub fn memfill_size(&self) -> u32 { self.word(0x2C) }

    pub fn sp_address(&self) -> u32 { self.word(0x30) }
    pub fn sp_size(&self) -> u32 { self.word(0x34) }
}
