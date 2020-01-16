use crate::cpu::CPU;
use crate::memory::Memory;

use std::path::Path;

pub struct PSX
{
    mem: Memory,
    cpu: CPU
}

impl PSX
{
    pub fn new(biosPath: &Path) -> Self
    {
        let mem = Memory::new(biosPath);

        PSX
        {
            mem: mem,
            cpu: CPU::new()
        }
    }

    pub fn load_bios()
    {

    }

    pub fn step(&mut self)
    {
        self.cpu.step(&mut self.mem);
    }
}
