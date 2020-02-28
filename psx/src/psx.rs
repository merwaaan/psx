use crate::cpu::CPU;
use crate::memory::Memory;

use std::path::Path;

pub struct PSX
{
    pub mem: Memory,
    pub cpu: CPU
}

impl PSX
{
    pub fn new(bios_path: &Path) -> Self
    {
        env_logger::init();

        let mem = Memory::new(bios_path);

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

    pub fn run(&mut self)
    {
        self.cpu.run(&mut self.mem);
    }
}
