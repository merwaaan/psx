use crate::cpu::{ CPU };

pub struct PSX
{
    cpu: CPU
}

impl PSX
{
    pub fn new() -> Self
    {
        PSX
        {
            cpu: CPU::new()
        }
    }

    pub fn load_bios()
    {

    }

    pub fn step(&mut self)
    {
        self.cpu.step();
    }
}
