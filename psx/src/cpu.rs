use crate::memory::Memory;

// TODO make sure R0 always 0

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

    // Bits 15 to 11
    fn rd(&self) -> u32
    {
        let Opcode(code) = self;
        (code >> 11) & 0x1F as u32
    }

    // Bits 5 to 0
    fn sub(&self) -> u32
    {
        let Opcode(code) = self;
        code & 0x3F as u32
    }

    // Bits 10 to 6
    fn imm5(&self) -> u32
    {
        let Opcode(code) = self;
        (code >> 6) & 0x1F as u32
    }

    // Bits 25 to 0
    fn imm26(&self) -> u32
    {
        let Opcode(code) = self;
        code & 0x3FFFFFF as u32
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

        println!("opcode {:08x} @ {:08x}", opcode, self.pc);
        for i in 0 .. 32
        {
            //println!("\tR{} = {:08x}", i, self.r[i]);
        }

        match Opcode(opcode).instr()
        {
            //0b000000 => self.sll(mem, &Opcode(opcode)),
            0b000000 =>
            {
                match Opcode(opcode).sub()
                {
                    0b000000 => self.sll(mem, &Opcode(opcode)),
                    0b100101 => self.or(mem, &Opcode(opcode)),
                    _        => panic!("Unsupported opcode: {:08x}", opcode)
                }
            }
            0b000010 => self.jump(mem, &Opcode(opcode)),
            0b001001 => self.addiu(mem, &Opcode(opcode)),
            0b001101 => self.ori(mem, &Opcode(opcode)),
            0b001111 => self.lui(mem, &Opcode(opcode)),
            0b101011 => self.sw(mem, &Opcode(opcode)),
            0b101111 => self.jump(mem, &Opcode(opcode)),
            _        => panic!("Unsupported opcode: {:08x}", opcode)
        }
    }

    fn addiu(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        println!("ADDIU");
        self.r[opcode.rt() as usize] = self.r[opcode.rs() as usize].wrapping_add(opcode.imm());
    }

    fn jump(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        println!("JUMP opcode {:08x}", opcode.imm26());
        self.pc = (self.pc & 0xF0000000) | (opcode.imm26() << 2);
    }

    fn lui(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        println!("LUI");
        self.r[opcode.rt() as usize] = opcode.imm() << 16;
    }

    fn or(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        println!("OR {:08x}", opcode.rd());
        self.r[opcode.rd() as usize] = self.r[opcode.rt() as usize] | self.r[opcode.rs() as usize];
    }

    fn ori(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        println!("ORI");
        self.r[opcode.rt() as usize] |= opcode.imm();
    }

    fn sll(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        println!("SLL");
        self.r[opcode.rd() as usize] = self.r[opcode.rt() as usize] << opcode.imm5();
    }

    fn sw(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        println!("SW");
        // TODO sign extension?
        mem.write(self.r[opcode.rs() as usize] + opcode.imm(), self.r[opcode.rt() as usize]);
    }
}