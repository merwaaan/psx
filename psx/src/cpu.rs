pub struct CPU
{
    pub pc: u32,
    pub r: [u32; 32]
}

impl CPU
{
    pub fn new() -> Self
    {
        CPU
        {
            pc: 0,
            r: [0; 32]
        }
    }

    pub fn step(&mut self)
    {

    }
}