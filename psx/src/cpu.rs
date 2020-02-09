// Notes
//
// Execution diffs after ADDIU is called a few times @ bfc01a60 (3rd time?)
//
// BIOS @BFC06F0C = ori in nopsx but lui here?!
//
// CONTINUE: seems OK until 83815

use crate::memory::Memory;

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

    // Bits 15 to 0, sign-extended
    fn imm_se(&self) -> u32
    {
        let Opcode(code) = self;
        (code & 0xFFFF) as i16 as u32
    }
}

// TODO make sure R0 always 0

pub struct CPU
{
    pc: u32,

    r: [u32; 32],
    r_out: [u32; 32], // To simulate the load-delay slot

    fetched_opcode: (u32, u32), //(opcode, pc)
    pending_load: (u32, u32), // (register, value)

    status: u32, // TODO all cop registers?

    counter: u32
}

impl CPU
{
    pub fn new() -> Self
    {
        CPU
        {
            pc: 0xBFC00000, // The PC starts with the BIOS address
            r: [0; 32],
            r_out: [0; 32],
            fetched_opcode: (0, 0), // NOP
            pending_load: (0, 0),
            status: 0,

            counter: 0
        }
    }

    pub fn step(&mut self, mem: &mut Memory)
    {
        // Pending load

        self.set_reg(self.pending_load.0, self.pending_load.1);
        self.pending_load = (0, 0);

        // Fetch the next instruction

        let (opcode, pc) = self.fetched_opcode;

        self.fetched_opcode = (mem.read(self.pc), self.pc); // TODO directly Opcode, need to impl to str

        for i in 0 .. 32
        {
            println!("\tR{} = {:08x}", i, self.r[i]);
        }

        println!("\nopcode {:08x} @ {:08x} | {:b} | {}", opcode, pc, self.status, self.counter);

        self.pc += 4;

        self.counter += 1;

        match Opcode(opcode).instr()
        {
            0b000000 =>
            {
                match Opcode(opcode).sub()
                {
                    0b000000 => self.sll(&Opcode(opcode)),
                    0b000011 => self.sra(&Opcode(opcode)),
                    0b001000 => self.jr(&Opcode(opcode)),
                    0b001001 => self.jalr(&Opcode(opcode)),
                    0b100000 => self.add(&Opcode(opcode)),
                    0b100001 => self.addu(&Opcode(opcode)),
                    0b100100 => self.and(&Opcode(opcode)),
                    0b100101 => self.or(&Opcode(opcode)),
                    0b101011 => self.sltu(&Opcode(opcode)),
                    _        => panic!("Unsupported opcode: {:08x}", opcode)
                }
            },
            0b000001 =>
            {
                match Opcode(opcode).rt()
                {
                    0b00000 => self.bltz(&Opcode(opcode)),
                    0b00001 => self.bgez(&Opcode(opcode)),
                    0b10000 => self.bltzal(&Opcode(opcode)),
                    0b10001 => self.bgezal(&Opcode(opcode)),
                    _        => panic!("Unsupported opcode: {:08x}", opcode)
                }
            },
            0b000010 => self.j(&Opcode(opcode)),
            0b000011 => self.jal(&Opcode(opcode)),
            0b000100 => self.beq(&Opcode(opcode)),
            0b000101 => self.bne(&Opcode(opcode)),
            0b000110 => self.blez(&Opcode(opcode)),
            0b000111 => self.bgtz(&Opcode(opcode)),
            0b001000 => self.addi(& Opcode(opcode)),
            0b001001 => self.addiu(& Opcode(opcode)),
            0b001010 => self.slti(& Opcode(opcode)),
            0b001100 => self.andi(&Opcode(opcode)),
            0b001101 => self.ori(&Opcode(opcode)),
            0b001111 => self.lui(&Opcode(opcode)),
            0b010000 =>
            {
                match Opcode(opcode).rs()
                {
                    0b00000 => self.cop0_mfc(&Opcode(opcode)),
                    0b00100 => self.cop0_mtc(&Opcode(opcode)),
                    _        => panic!("Unsupported opcode: {:08x}", opcode)
                }
            },
            0b100000 => self.lb(mem, &Opcode(opcode)),
            0b100011 => self.lw(mem, &Opcode(opcode)),
            0b100100 => self.lbu(mem, &Opcode(opcode)),
            0b101000 => self.sb(mem, &Opcode(opcode)),
            0b101001 => self.sh(mem, &Opcode(opcode)),
            0b101011 => self.sw(mem, &Opcode(opcode)),
            _        => panic!("Unsupported opcode: {:08x}", opcode)
        }

        // Update the registers to account for the load-delay slot

        self.r = self.r_out;
    }

    fn reg(&mut self, index: u32) -> u32
    {
        self.r[index as usize]
    }

    fn set_reg(&mut self, index: u32, value: u32)
    {
        self.r_out[index as usize] = value;
        self.r_out[0] = 0; // R0 is always zero
    }

    // ADDIU truncates on overflow
    // ADDI generates an exception on overflow

    fn addi(&mut self, opcode: &Opcode)
    {
        println!("ADDI _ R{}={:08x} + {:08x} = {:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.imm_se(), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()), opcode.rt());

        let rs = self.reg(opcode.rs()) as i32;
        let imm = opcode.imm_se() as i32;

        let result = match rs.checked_add(imm)
        {
            Some(result) => result as u32,
            None         => panic!("ADDI overflow")
        };

        self.set_reg(opcode.rt(), result);
    }

    fn addiu(&mut self, opcode: &Opcode)
    {
        println!("ADDIU _ R{}={:08x} + {:08x} = {:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.imm_se(), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()), opcode.rt());

        let result = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());
        self.set_reg(opcode.rt(), result);
    }

    fn add(&mut self, opcode: &Opcode)
    {
        println!("ADD _ R{}={:08x} + R{}={:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()), opcode.rd());

        let rs = self.reg(opcode.rs()) as i32;
        let rt = self.reg(opcode.rt()) as i32;

        let result = match rs.checked_add(rt)
        {
            Some(result) => result as u32,
            None         => panic!("ADD overflow")
        };

        self.set_reg(opcode.rd(), result);
    }

    fn addu(&mut self, opcode: &Opcode)
    {
        println!("ADDU _ R{}={:08x} + R{}={:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()), opcode.rd());

        let result = self.reg(opcode.rs()).wrapping_add(self.reg(opcode.rt()));
        self.set_reg(opcode.rd(), result);
    }

    fn and(&mut self, opcode: &Opcode)
    {
        println!("AND _ R{}={:08x} & R{}={:08x} = {:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()), self.reg(opcode.rs()) & opcode.rt(), opcode.rd());

        let result = self.reg(opcode.rs()) & self.reg(opcode.rt());
        self.set_reg(opcode.rd(), result);
    }

    fn andi(&mut self, opcode: &Opcode)
    {
        println!("ANDI _ R{}={:08x} & {:08x} = {:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.imm(), self.reg(opcode.rs()) & opcode.imm(), opcode.rt());

        let result = self.reg(opcode.rs()) & opcode.imm();
        self.set_reg(opcode.rt(), result);
    }

    fn beq(&mut self, opcode: &Opcode)
    {
        println!("BEQ _ branch to PC + {:08x} << 2 = {:08x} if R{}={:08x} == R{}={:08x}", opcode.imm_se(), self.pc.wrapping_add(opcode.imm_se() << 2), opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()));

        if self.reg(opcode.rs()) == self.reg(opcode.rt())
        {
            self.pc = self.pc.wrapping_add(opcode.imm_se() << 2).wrapping_sub(4);
        }
    }

    fn bne(&mut self, opcode: &Opcode)
    {
        println!("BNE _ branch to PC + {:08x} << 2 = {:08x} if R{}={:08x} != R{}={:08x}", opcode.imm_se(), self.pc.wrapping_add(opcode.imm_se() << 2), opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()));

        if self.reg(opcode.rs()) != self.reg(opcode.rt())
        {
            self.pc = self.pc.wrapping_add(opcode.imm_se() << 2).wrapping_sub(4);
        }
    }

    fn bgtz(&mut self, opcode: &Opcode)
    {
        println!("BGTZ _ branch to PC + {:08x} << 2 = {:08x} if R{}={:08x} > 0", opcode.imm_se(), self.pc.wrapping_add(opcode.imm_se() << 2), opcode.rs(), self.reg(opcode.rs()));

        let rs = self.reg(opcode.rs()) as i32;

        if rs > 0
        {
            self.pc = self.pc.wrapping_add(opcode.imm_se() << 2).wrapping_sub(4);
        }
    }

    fn blez(&mut self, opcode: &Opcode)
    {
        println!("BLEZ _ branch to PC + {:08x} << 2 = {:08x} if R{}={:08x} <= 0", opcode.imm_se(), self.pc.wrapping_add(opcode.imm_se() << 2), opcode.rs(), self.reg(opcode.rs()));

        let rs = self.reg(opcode.rs()) as i32;

        if rs <= 0
        {
            self.pc = self.pc.wrapping_add(opcode.imm_se() << 2).wrapping_sub(4);
        }
    }

    fn bgez(&mut self, opcode: &Opcode)
    {
        println!("BGEZ _ branch to PC + {:08x} << 2 = {:08x} if R{}={:08x} >= 0", opcode.imm_se(), self.pc.wrapping_add(opcode.imm_se() << 2), opcode.rs(), self.reg(opcode.rs()));

        let rs = self.reg(opcode.rs()) as i32;

        if rs >= 0
        {
            self.pc = self.pc.wrapping_add(opcode.imm_se() << 2).wrapping_sub(4);
        }
    }

    fn bgezal(&mut self, opcode: &Opcode)
    {
        println!("BGEZAL _ branch to PC + {:08x} << 2 = {:08x} if R{}={:08x} >= 0", opcode.imm_se(), self.pc.wrapping_add(opcode.imm_se() << 2), opcode.rs(), self.reg(opcode.rs()));

        let rs = self.reg(opcode.rs()) as i32;

        if rs >= 0
        {
            self.set_reg(31, self.pc);
            self.pc = self.pc.wrapping_add(opcode.imm_se() << 2).wrapping_sub(4);
        }
    }

    fn bltz(&mut self, opcode: &Opcode)
    {
        println!("BLTZ _ branch to PC + {:08x} << 2 = {:08x} if R{}={:08x} < 0", opcode.imm_se(), self.pc.wrapping_add(opcode.imm_se() << 2), opcode.rs(), self.reg(opcode.rs()));

        let rs = self.reg(opcode.rs()) as i32;

        if rs < 0
        {
            self.pc = self.pc.wrapping_add(opcode.imm_se() << 2).wrapping_sub(4);
        }
    }

    fn bltzal(&mut self, opcode: &Opcode)
    {
        println!("BLTZAL _ branch to PC + {:08x} << 2 = {:08x} if R{}={:08x} < 0", opcode.imm_se(), self.pc.wrapping_add(opcode.imm_se() << 2), opcode.rs(), self.reg(opcode.rs()));

        let rs = self.reg(opcode.rs()) as i32;

        if rs < 0
        {
            self.set_reg(31, self.pc);
            self.pc = self.pc.wrapping_add(opcode.imm_se() << 2).wrapping_sub(4);
        }
    }

    fn cop0_mfc(&mut self, opcode: &Opcode)
    {
        println!("COP0 MFC | COP R{} -> R{}", opcode.rd(), opcode.rt());

        let value = match opcode.rd()
        {
            12 => self.status,
            _  => panic!("Unsupported cop0 register: {:08x}", opcode.rd())
        };

        // Put in the load-delay slot
        self.pending_load = (opcode.rt(), value);
    }

    fn cop0_mtc(&mut self, opcode: &Opcode)
    {
        println!("COP0 MTC | R{} = {:08x} -> COP R{}", opcode.rt(), self.reg(opcode.rt()), opcode.rd());

        match opcode.rd()
        {
            3 | 5 | 6 | 7| 9 | 11 | 13 => println!("Ignored write to CR{}", opcode.rd()),
            12 => self.status = self.reg(opcode.rt()),
            _  => panic!("Unsupported cop0 register: {:08x}", opcode.rd())
        };
    }

    fn j(&mut self, opcode: &Opcode)
    {
        println!("J _ {:08x}", opcode.imm26());

        self.pc = (self.pc & 0xF0000000) | (opcode.imm26() << 2);
    }

    fn jal(&mut self, opcode: &Opcode)
    {
        println!("JAL _ {:08x}", opcode.imm26());

        self.set_reg(31, self.pc);
        self.pc = (self.pc & 0xF0000000) | (opcode.imm26() << 2);
    }

    fn jalr(&mut self, opcode: &Opcode)
    {
        println!("JALR _ R{}={:08x}", opcode.rs(), self.reg(opcode.rs()));

        self.set_reg(opcode.rd(), self.pc);
        self.pc = self.reg(opcode.rs());
    }

    fn jr(&mut self, opcode: &Opcode)
    {
        println!("JR _ R{}={:08x}", opcode.rs(), self.reg(opcode.rs()));

        self.pc = self.reg(opcode.rs());
    }

    fn lui(&mut self, opcode: &Opcode)
    {
        println!("LUI _ {:08x} << 16 = {:08x} -> R{}", opcode.imm(), opcode.imm() << 16, opcode.rt());

        self.set_reg(opcode.rt(), opcode.imm() << 16);
    }

    fn lb(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        println!("LB _ {:08x}(R{})={:08x} -> R{}", opcode.imm_se(), opcode.rs(), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()), opcode.rt());

        if self.status & 0x10000 != 0
        {
            println!("Cache is isolated, ignoring");
            return;
        }

        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());
        let result = mem.read8(address) as i8;

        // Put in the load-delay slot
        self.pending_load = (opcode.rt(), result as u32);
    }

    fn lbu(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        println!("LBU _ {:08x}(R{})={:08x} -> R{}", opcode.imm_se(), opcode.rs(), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()), opcode.rt());

        if self.status & 0x10000 != 0
        {
            println!("Cache is isolated, ignoring");
            return;
        }

        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());
        let result = mem.read8(address);

        // Put in the load-delay slot
        self.pending_load = (opcode.rt(), result as u32);
    }

    fn lw(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        println!("LW _ {:08x}(R{})={:08x} -> R{}", opcode.imm_se(), opcode.rs(), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()), opcode.rt());

        if self.status & 0x10000 != 0
        {
            println!("Cache is isolated, ignoring");
            return;
        }

        let result = mem.read(self.reg(opcode.rs()).wrapping_add(opcode.imm_se()));

        // Put in the load-delay slot
        self.pending_load = (opcode.rt(), result);
    }

    fn or(&mut self, opcode: &Opcode)
    {
        println!("OR _ R{}={:08x} | R{}={:08x} = {:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()), self.reg(opcode.rt()) | self.reg(opcode.rs()), opcode.rd());

        let result = self.reg(opcode.rt()) | self.reg(opcode.rs());
        self.set_reg(opcode.rd(), result);
    }

    fn ori(&mut self, opcode: &Opcode)
    {
        println!("ORI");

        let result = self.reg(opcode.rs()) | opcode.imm();
        self.set_reg(opcode.rt(), result);
    }

    fn sll(&mut self, opcode: &Opcode)
    {
        println!("SLL _ R{} << {} -> R{}", opcode.rt(), opcode.imm5(), opcode.rd());

        let result = self.reg(opcode.rt()) << opcode.imm5();
        self.set_reg(opcode.rd(), result);

    }

    fn sra(&mut self, opcode: &Opcode)
    {
        println!("SRA _ R{} >> {} -> R{}", opcode.rt(), opcode.imm5(), opcode.rd());

        let rt = self.reg(opcode.rt()) as i32;
        let result = rt >> opcode.imm5();
        self.set_reg(opcode.rd(), result as u32);
    }

    fn slti(&mut self, opcode: &Opcode)
    {
        println!("SLTI _ R{}={:08x} < {:08x} ? -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.imm_se(), opcode.rt());

        let rs = self.reg(opcode.rs()) as i32;
        let imm = self.reg(opcode.imm_se()) as i32;

        let result = rs < imm;
        self.set_reg(opcode.rt(), result as u32);
    }

    fn sltu(&mut self, opcode: &Opcode)
    {
        println!("SLTU _ R{} < R{} ? -> R{}", opcode.rs(), opcode.rt(), opcode.rd());

        let result = self.reg(opcode.rs()) < self.reg(opcode.rt());
        self.set_reg(opcode.rd(), result as u32);
    }

    fn sb(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        println!("SB _ R{}={:08x} -> {:08x}(R{}={:08x})={:08x}", opcode.rt(), self.reg(opcode.rt()), opcode.imm_se(), opcode.rs(), self.reg(opcode.rs()), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()));

        if self.status & 0x10000 != 0
        {
            println!("Cache is isolated, ignoring");
            return;
        }

        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());
        let value = self.reg(opcode.rt()) as u8;
        mem.write8(address, value);
    }

    fn sh(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        println!("SH _ R{}={:08x} -> {:08x}(R{}={:08x})={:08x}", opcode.rt(), self.reg(opcode.rt()), opcode.imm_se(), opcode.rs(), self.reg(opcode.rs()), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()));

        if self.status & 0x10000 != 0
        {
            println!("Cache is isolated, ignoring");
            return;
        }

        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());
        let value = self.reg(opcode.rt()) as u16;
        mem.write16(address, value);
    }

    fn sw(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        println!("SW _ R{}={:08x} -> {:08x}(R{}={:08x})={:08x}", opcode.rt(), self.reg(opcode.rt()), opcode.imm_se(), opcode.rs(), self.reg(opcode.rs()), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()));

        if self.status & 0x10000 != 0
        {
            println!("Cache is isolated, ignoring");
            return;
        }

        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());
        mem.write(address, self.reg(opcode.rt()));
    }
}