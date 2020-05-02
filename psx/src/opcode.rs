use std::fmt;

pub struct Opcode(pub u32);

impl Opcode
{
    // Bits 31 - 26
    pub fn instr(&self) -> u32
    {
        let Opcode(bits) = self;
        bits >> 26
    }

    // Bits 25 - 21
    pub fn rs(&self) -> u32
    {
        let Opcode(bits) = self;
        (bits >> 21) & 0x1F as u32
    }

    // Bits 20 - 16
    pub fn rt(&self) -> u32
    {
        let Opcode(bits) = self;
        (bits >> 16) & 0x1F as u32
    }

    // Bits 15 - 11
    pub fn rd(&self) -> u32
    {
        let Opcode(bits) = self;
        (bits >> 11) & 0x1F as u32
    }

    // Bits 15 - 0
    pub fn imm(&self) -> u32
    {
        let Opcode(bits) = self;
        bits & 0xFFFF
    }

    // Bits 15 - 0, sign-extended
    pub fn imm_se(&self) -> u32
    {
        let Opcode(bits) = self;
        (bits & 0xFFFF) as i16 as u32
    }

    // Bits 25 - 0
    pub fn imm26(&self) -> u32
    {
        let Opcode(bits) = self;
        bits & 0x3FFFFFF as u32
    }

    // Bits 10 - 6
    pub fn imm5(&self) -> u32
    {
        let Opcode(bits) = self;
        (bits >> 6) & 0x1F as u32
    }

    // Bits 5 - 0
    pub fn sub(&self) -> u32
    {
        let Opcode(bits) = self;
        bits & 0x3F as u32
    }
}

impl fmt::LowerHex for Opcode
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        let Opcode(bits) = self;
        fmt::LowerHex::fmt(&bits, f)
    }
}

impl fmt::UpperHex for Opcode
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        let Opcode(bits) = self;
        fmt::UpperHex::fmt(&bits, f)
    }
}
