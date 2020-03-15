use crate::cpu::CPU;
use crate::gpu::GPU;
use crate::memory::Memory;

use std::path::Path;

pub struct PSX
{
    pub mem: Memory,
    pub cpu: CPU
}

impl PSX
{
    pub fn new(bios_path: &Path, display: &glium::Display) -> Self
    {
        env_logger::init();

        let mem = Memory::new(bios_path, display);

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

    pub fn run(&mut self, instructions: u32) -> bool
    {
        self.cpu.run(instructions, &mut self.mem)
    }

    // TEMP
    pub fn gpu(&self) -> &GPU
    {
        &self.mem.gpu
    }
    pub fn gpu_mut(&mut self) -> &mut GPU
    {
        &mut self.mem.gpu
    }
}
