use crate::cpu::CPU;
use crate::memory::Memory;

pub struct Debugger
{
    breakpoints: Vec<u32>
}

impl Debugger
{
    pub fn new() -> Self
    {
        Debugger
        {
            breakpoints: Vec::new()
        }
    }

    pub fn add_breakpoint(&mut self, address: u32)
    {
        if !self.breakpoints.contains(&address)
        {
            self.breakpoints.push(address);
        }
    }

    pub fn remove_breakpoint(&mut self, address: u32)
    {
        self.breakpoints.retain(|&i| i != address);
    }

    pub fn is_breakpoint(&self, address: u32) -> bool
    {
        self.breakpoints.contains(&address)
    }

    pub fn get_breakpoints(&self) -> &[u32]
    {
        self.breakpoints.as_slice()
    }

    pub fn disassemble(&self, opcode: u32, cpu: &CPU, mem: &Memory) -> String
    {
        let mut template = String::from("LD $rs, $rt");
        template = template.replace("$rd", &format!("R{}", 12));
        template = template.replace("$rs", &format!("R{}", 12));
        template = template.replace("$rt", &format!("R{}", 12));
        template
    }
}
