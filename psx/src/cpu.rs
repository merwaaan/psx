use crate::memory::Memory;

pub struct CPU
{
    pub pc: u32,
    pub r: [u32; 32]
}

struct Opcode(u32);

impl Opcode
{
    // Bits 31 to 26
    fn instr(&self) -> u32
    {
        let Opcode(code) = self;
        code >> 26
    }

    // Bits 25 to 21
    fn rs(&self) -> u32
    {
        let Opcode(code) = self;
        (code >> 21) & 0x1F as u32
    }

    // Bits 20 to 16
    fn rt(&self) -> u32
    {
        let Opcode(code) = self;
        (code >> 16) & 0x1F as u32
    }

    // Bits 15 to 0
    fn imm(&self) -> u32
    {
        let Opcode(code) = self;
        code & 0xFFFF
    }
}

impl CPU
{
    pub fn new() -> Self
    {
        CPU
        {
            pc: 0xBFC00000, // The PC starts with the BIOS address
            r: [0; 32]
        }
    }

    pub fn step(&mut self, mem: &mut Memory)
    {
        let opcode = mem.read(self.pc); // TODO directly Opcode, need to impl to str
        self.pc += 4;

        println!("opcode {:08x}", opcode);

        match Opcode(opcode).instr()
        {
            0b001101 => self.ori(mem, &Opcode(opcode)),
            0b001111 => self.lui(mem, &Opcode(opcode)),
            _        => panic!("Unsupported opcode: {:08x} {:08x}", opcode, Opcode(opcode).imm())
        }
    }

    fn lui(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        self.r[opcode.rt() as usize] = opcode.imm() << 16;
    }

    fn ori(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        self.r[opcode.rt() as usize] = opcode.imm() << 16;
    }
}