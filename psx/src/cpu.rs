use crate::debugger::Debugger;
use crate::exefile::ExeFile;
use crate::interrupt_controller::InterruptController;
use crate::memory::{ Addressable, Memory };
use crate::opcode::Opcode;

use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;

// TODO make sure R0 always 0

#[derive(Clone, Debug)]
enum Exception
{
    ExternalInterrupt = 0,
    LoadAddress = 0x4,
    StoreAddress = 0x5,
    Syscall = 0x8,
    Break = 0x9,
    IllegalInstruction = 0xA,
    CoprocessorError = 0xB,
    Overflow = 0xC
}

pub struct CPU
{
    pub pc: u32,
    pub next_pc: u32,

    pub r: [u32; 32],
    r_next: [u32; 32], // To simulate the load-delay slot, contains the output of the current instruction

    pub hi: u32,
    pub lo: u32,

    pending_load: (u32, u32), // (register, value)
    previous_pending_load: (u32, u32), // to watch for double delayed loads on the same register

    status: u32, // TODO all cop registers?

    // Exception handling
    current_pc: u32,
    cop0_badaddr: u32, // TODO cop0 struct
    cop0_cause: u32,
    cop0_epc: u32,

    //
    branching: bool,
    in_delay_slot: bool,

    pub counter: u32, // debug helper

    log_file: File,
    logging: bool,

    pub debugger: Debugger, // TODO move out? or write memread/write function in cpu to wrap debugger registration

    interrupt_controller: Rc<RefCell<InterruptController>>,

    exe_path: Option<PathBuf>
}

impl CPU
{
    pub fn new(
        interrupt_controller: &Rc<RefCell<InterruptController>>,
        exe_path: Option<PathBuf>)
        -> Self
    {
        CPU
        {
            pc: 0xBFC00000, // The PC starts with the BIOS address,
            next_pc: 0xBFC00004,

            r: [0; 32],
            r_next: [0; 32],
            hi: 0,
            lo: 0,

            pending_load: (0, 0),
            previous_pending_load: (0, 0),

            status: 0,

            current_pc: 0,

            cop0_badaddr: 0,
            cop0_cause: 0,
            cop0_epc: 0,

            branching: false,
            in_delay_slot: false,

            counter: 0,

            log_file: File::create("custom_log_own.txt").unwrap(),
            logging: false,

            debugger: Debugger::new(),

            interrupt_controller: interrupt_controller.clone(),

            exe_path
        }
    }

    // Returns false if interrupted by a breakpoint
    pub fn run(&mut self, instructions: u32, mem: &mut Memory) -> bool
    {
        for _ in 0 .. instructions
        {
            if !self.step(mem)
            {
                return false;
            }
        }

        true
    }

    pub fn step(&mut self, mem: &mut Memory) -> bool
    {
        self.debugger.clear_data_access();

        // Apply any pending load

        self.set_reg(self.pending_load.0, self.pending_load.1);

        self.previous_pending_load = self.pending_load;
        self.pending_load = (0, 0);

        //

        self.in_delay_slot = self.branching;
        self.branching = false;

        //

        if self.update_pending_interrupt()
        {
            error!("CPU interrupt!");
            self.exception(Exception::ExternalInterrupt);
        }

        //

        if self.pc == 0x8003_0000 && self.exe_path.is_some()
        {
            let exe = ExeFile::new_from_file(self.exe_path.take().unwrap());
            exe.load(self, mem);
        }

        // Fetch the next instruction

        self.current_pc = self.pc;

        // TODO remove? dealt with in the jump instructions now
        if self.current_pc % 4 != 0
        {
            panic!("unexpected unaligned address {:08X}", self.current_pc);
        }

        let opcode = Opcode(mem.read(self.pc));

        self.pc = self.next_pc;
        self.next_pc = self.pc.wrapping_add(4);

        let debug_addr = 129925482 - 10000;
        let mut debug = false;
        if self.counter >= debug_addr && self.counter < debug_addr + 400000
        {
            debug = true;

            // debug logging

            //write!(&mut self.log_file, "{} {:08x} {:08x} \n", self.counter, self.current_pc, opcode).unwrap();

            let mut s = String::new();

            //s += &format!("{:08x} {:08x} \n", self.current_pc, opcode);
            s += &format!("{:08x} {:08x} ", self.current_pc, opcode);
            //s += &format!("{} {:08x} {:08x} ", self.counter, self.current_pc, opcode);

            for i in 0 .. 32
            {
               s += &format!("R{}={:08x} ", i, self.r[i]);
            }
            s += &format!("HI {:08x} ", self.hi);
            s += &format!("LO {:08x} ", self.lo);
            s += &format!("S {:08x} ", self.status);
            s += &format!("\n");

            write!(&mut self.log_file, "{}", s).unwrap();

            /*for i in 0 .. 32
            {
                debug!("\tR{} = {:08x}", i, self.r[i]);
            }*/

            self.logging = true;
        }

        self.counter += 1;

        /*if self.logging
        {
            debug!("\nopcode {:08x} @ {:08x} | {:b} | {}", opcode, self.current_pc, self.status, self.counter);
        }*/

        match opcode.instr()
        {
            0b000000 =>
            {
                match opcode.sub()
                {
                    0b000000 => self.sll(&opcode),
                    0b000010 => self.srl(&opcode),
                    0b000011 => self.sra(&opcode),
                    0b000100 => self.sllv(&opcode),
                    0b000110 => self.srlv(&opcode),
                    0b000111 => self.srav(&opcode),
                    0b001000 => self.jr(&opcode),
                    0b001001 => self.jalr(&opcode),
                    0b001100 => self.syscall(),
                    0b001101 => self.break_(),
                    0b010000 => self.mfhi(&opcode),
                    0b010001 => self.mthi(&opcode),
                    0b010010 => self.mflo(&opcode),
                    0b010011 => self.mtlo(&opcode),
                    0b011000 => self.mult(&opcode),
                    0b011001 => self.multu(&opcode),
                    0b011010 => self.div(&opcode),
                    0b011011 => self.divu(&opcode),
                    0b100000 => self.add(&opcode),
                    0b100001 => self.addu(&opcode),
                    0b100010 => self.sub(&opcode),
                    0b100011 => self.subu(&opcode),
                    0b100100 => self.and(&opcode),
                    0b100101 => self.or(&opcode),
                    0b100110 => self.xor(&opcode),
                    0b100111 => self.nor(&opcode),
                    0b101010 => self.slt(&opcode),
                    0b101011 => self.sltu(&opcode),
                    _        => self.illegal(&opcode),
                }
            },
            0b000001 => self.bcond(&opcode),
            0b000010 => self.j(&opcode),
            0b000011 => self.jal(&opcode),
            0b000100 => self.beq(&opcode),
            0b000101 => self.bne(&opcode),
            0b000110 => self.blez(&opcode),
            0b000111 => self.bgtz(&opcode),
            0b001000 => self.addi(&opcode),
            0b001001 => self.addiu(&opcode),
            0b001010 => self.slti(&opcode),
            0b001011 => self.sltiu(&opcode),
            0b001100 => self.andi(&opcode),
            0b001101 => self.ori(&opcode),
            0b001110 => self.xori(&opcode),
            0b001111 => self.lui(&opcode),
            0b010000 =>
            {
                match opcode.rs()
                {
                    0b00000 => self.cop0_mfc(&opcode),
                    0b00100 => self.cop0_mtc(&opcode),
                    0b10000 => self.cop0_rfe(),
                    _       => self.illegal(&opcode)
                }
            },
            0b010001 => self.cop1(),
            0b010010 => self.cop2(),
            0b010011 => self.cop3(),
            0b100000 => self.lb(mem, &opcode),
            0b100001 => self.lh(mem, &opcode),
            0b100010 => self.lwl(mem, &opcode),
            0b100011 => self.lw(mem, &opcode),
            0b100100 => self.lbu(mem, &opcode),
            0b100101 => self.lhu(mem, &opcode),
            0b100110 => self.lwr(mem, &opcode),
            0b101000 => self.sb(mem, &opcode),
            0b101001 => self.sh(mem, &opcode),
            0b101010 => self.swl(mem, &opcode),
            0b101011 => self.sw(mem, &opcode),
            0b101110 => self.swr(mem, &opcode),
            0b110000 => self.cop0_lwc(),
            0b110001 => self.cop1_lwc(),
            0b110010 => self.cop2_lwc(),
            0b110011 => self.cop3_lwc(),
            0b111000 => self.cop0_swc(),
            0b111001 => self.cop1_swc(),
            0b111010 => self.cop2_swc(),
            0b111011 => self.cop3_swc(),
            _        => self.illegal(&opcode)
        }

        // Update the registers to account for the load-delay slot

        self.r = self.r_next;

        // Check breakpoints

        let stop = self.debugger.is_breakpoint(self.next_pc, self) || self.debugger.has_data_breakpoint();

        !stop
    }

    pub fn reg(&self, index: u32) -> u32
    {
        self.r[index as usize]
    }

    // Instructions must use this function to set registers so that
    // load-delay slots are properly emulated.
    pub fn set_reg(&mut self, index: u32, value: u32)
    {
        self.r_next[index as usize] = value;
        self.r_next[0] = 0; // R0 is always zero
    }

    // Updates the CAUSE register with the interrupt controller's state.
    // Returns true if an interrupt is pending.
    fn update_pending_interrupt(&mut self) -> bool
    {
        // Update bit 10 of the CAUSE register to reflect that an interrupt is pending or not

        if self.interrupt_controller.borrow().pending(false)
        {
            self.cop0_cause |= 1 << 10; // bit 10 = external interrupt
            //error!("SETTING CAUSE BIT: {:032b}", self.cop0_cause);
        }
        else
        {
            self.cop0_cause &= !(1 << 10);
        }

        /*if self.counter > 129925482 - 10000
        {
            error!("CAUSE {:032b}", self.cop0_cause);
        }*/

        (self.status & 1) != 0 && // Interrupt enabled globally
        (((self.cop0_cause & self.status) >> 8) & 0xFF) != 0 // pending (cause) & mask (status)
    }

    fn read<T: Addressable>(&mut self, mem: &mut Memory, address: u32) -> T// TODO mut because of debuffer??
    {
        self.debugger.register_data_access(address, true);
        mem.read::<T>(address)
    }

    fn write<T: Addressable>(&mut self, mem: &mut Memory, address: u32, value: T)
    {
        self.debugger.register_data_access(address, false);
        mem.write::<T>(address, value)
    }

    fn illegal(&mut self, opcode: &Opcode)
    {
        error!("Illegal instruction: {:08X}", opcode);
        self.exception(Exception::IllegalInstruction);
    }

    // ADDIU truncates on overflow
    // ADDI generates an exception on overflow

    fn addi(&mut self, opcode: &Opcode)
    {
        trace!("ADDI _ R{}={:08x} + {:08x} = {:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.imm_se(), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()), opcode.rt());

        let rs = self.reg(opcode.rs()) as i32;
        let imm = opcode.imm_se() as i32;

        match rs.checked_add(imm)
        {
            Some(result) => self.set_reg(opcode.rt(), result as u32),
            None         => self.exception(Exception::Overflow)
        };
    }

    fn addiu(&mut self, opcode: &Opcode)
    {
        trace!("ADDIU _ R{}={:08x} + {:08x} = {:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.imm_se(), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()), opcode.rt());

        let result = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());
        self.set_reg(opcode.rt(), result);
    }

    fn add(&mut self, opcode: &Opcode)
    {
        trace!("ADD _ R{}={:08x} + R{}={:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()), opcode.rd());

        let rs = self.reg(opcode.rs()) as i32;
        let rt = self.reg(opcode.rt()) as i32;

        match rs.checked_add(rt)
        {
            Some(result) => self.set_reg(opcode.rd(), result as u32),
            None         => self.exception(Exception::Overflow)
        };
    }

    fn mult(&mut self, opcode: &Opcode)
    {
        trace!("MULT _ R{}={:08x} + R{}={:08x} -> HI/LO", opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()));

        let rs = (self.reg(opcode.rs()) as i32) as i64; // "as i64" alone does not sign-extend
        let rt = (self.reg(opcode.rt()) as i32) as i64;

        let mul = (rs * rt) as u64;

        self.hi = (mul >> 32) as u32;
        self.lo = mul as u32;
    }

    fn multu(&mut self, opcode: &Opcode)
    {
        trace!("MULTU _ R{}={:08x} + R{}={:08x} -> HI/LO", opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()));

        let rs = self.reg(opcode.rs()) as u64;
        let rt = self.reg(opcode.rt()) as u64;

        let mul = rs * rt;

        self.hi = (mul >> 32) as u32;
        self.lo = mul as u32;
    }

    fn div(&mut self, opcode: &Opcode)
    {
        trace!("DIV _ R{}={:08x} / R{}={:08x}", opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()));

        let num = self.reg(opcode.rs()) as i32;
        let den = self.reg(opcode.rt()) as i32;

        if den == 0
        {
            self.hi = num as u32;
            self.lo = if num < 0 { 1 }  else { 0xFFFFFFFF };
        }
        else if num as u32 == 0x80000000 && den == -1
        {
            self.hi = 0;
            self.lo = 0x80000000;
        }
        else
        {
            self.hi = (num % den) as u32;
            self.lo = (num / den) as u32;
        }
    }

    fn divu(&mut self, opcode: &Opcode)
    {
        trace!("DIVU _ R{}={:08x} / R{}={:08x}", opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()));

        let num = self.reg(opcode.rs());
        let den = self.reg(opcode.rt());

        if den == 0
        {
            self.hi = num;
            self.lo = 0xFFFFFFFF;
        }
        else
        {
            self.hi = num % den;
            self.lo = num / den;
        }
    }

    fn addu(&mut self, opcode: &Opcode)
    {
        trace!("ADDU _ R{}={:08x} + R{}={:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()), opcode.rd());

        let result = self.reg(opcode.rs()).wrapping_add(self.reg(opcode.rt()));
        self.set_reg(opcode.rd(), result);
    }

    fn sub(&mut self, opcode: &Opcode)
    {
        trace!("SUB _ R{}={:08x} - R{}={:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()), opcode.rd());

        let rs = self.reg(opcode.rs()) as i32;
        let rt = self.reg(opcode.rt()) as i32;

        match rs.checked_sub(rt)
        {
            Some(result) => self.set_reg(opcode.rd(), result as u32),
            None         => self.exception(Exception::Overflow)
        }
    }

    fn subu(&mut self, opcode: &Opcode)
    {
        trace!("SUBU _ R{}={:08x} - R{}={:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()), opcode.rd());

        let result = self.reg(opcode.rs()).wrapping_sub(self.reg(opcode.rt()));
        self.set_reg(opcode.rd(), result);
    }

    fn and(&mut self, opcode: &Opcode)
    {
        trace!("AND _ R{}={:08x} & R{}={:08x} = {:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()), self.reg(opcode.rs()) & opcode.rt(), opcode.rd());

        let result = self.reg(opcode.rs()) & self.reg(opcode.rt());
        self.set_reg(opcode.rd(), result);
    }

    fn andi(&mut self, opcode: &Opcode)
    {
        trace!("ANDI _ R{}={:08x} & {:08x} = {:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.imm(), self.reg(opcode.rs()) & opcode.imm(), opcode.rt());

        let result = self.reg(opcode.rs()) & opcode.imm();
        self.set_reg(opcode.rt(), result);
    }

    fn beq(&mut self, opcode: &Opcode)
    {
        trace!("BEQ _ branch to PC + {:08x} << 2 = {:08x} if R{}={:08x} == R{}={:08x}", opcode.imm_se(), self.pc.wrapping_add(opcode.imm_se() << 2), opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()));

        if self.reg(opcode.rs()) == self.reg(opcode.rt())
        {
            self.next_pc = self.pc.wrapping_add(opcode.imm_se() << 2);
        }

        self.branching = true;
    }

    fn bne(&mut self, opcode: &Opcode)
    {
        trace!("BNE _ branch to PC + {:08x} << 2 = {:08x} if R{}={:08x} != R{}={:08x}", opcode.imm_se(), self.pc.wrapping_add(opcode.imm_se() << 2), opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()));

        if self.reg(opcode.rs()) != self.reg(opcode.rt())
        {
            self.next_pc = self.pc.wrapping_add(opcode.imm_se() << 2);
        }

        self.branching = true;
    }

    fn bcond(&mut self, opcode: &Opcode)
    {
        // Contrarily to what the nocash doc implies, there are more than
        // 4 fixed opcodes for the conditional branching instructions.
        //
        // They rather act as a single instruction with a different outcome
        // depending on the condition (bit 16) and linking (bit 20-17).

        let mode = opcode.rt();

        // Evaluate the condition

        let rs = self.reg(opcode.rs()) as i32;

        let branch = if mode & 1 != 0 { rs >= 0 } else { rs < 0 }; // BGEZxx / BLTZxx

        if branch
        {
            self.next_pc = self.pc.wrapping_add(opcode.imm_se() << 2);
        }

        // If linking, out the return address in register 31 whatever the outcome

        if mode & 0b11110 == 0b10000
        {
            // TODO delayed or not?
            self.set_reg(31, self.pc.wrapping_add(4)); // (+4 to go after the branch-delay slot)
        }

        self.branching = true;
    }

    fn bgtz(&mut self, opcode: &Opcode)
    {
        trace!("BGTZ _ branch to PC + {:08x} << 2 = {:08x} if R{}={:08x} > 0", opcode.imm_se(), self.pc.wrapping_add(opcode.imm_se() << 2), opcode.rs(), self.reg(opcode.rs()));

        let rs = self.reg(opcode.rs()) as i32;

        if rs > 0
        {
            self.next_pc = self.pc.wrapping_add(opcode.imm_se() << 2);
        }

        self.branching = true;
    }

    fn blez(&mut self, opcode: &Opcode)
    {
        trace!("BLEZ _ branch to PC + {:08x} << 2 = {:08x} if R{}={:08x} <= 0", opcode.imm_se(), self.pc.wrapping_add(opcode.imm_se() << 2), opcode.rs(), self.reg(opcode.rs()));

        let rs = self.reg(opcode.rs()) as i32;

        if rs <= 0
        {
            self.next_pc = self.pc.wrapping_add(opcode.imm_se() << 2);
        }

        self.branching = true;
    }

    fn cop0_mfc(&mut self, opcode: &Opcode)
    {
        trace!("COP0 MFC | COP R{} -> R{}", opcode.rd(), opcode.rt());

        let value = match opcode.rd()
        {
            6 => { warn!("COP JUMPDEST READ not implemented"); 0 },
            7 => { warn!("COP DCIC READ not implemented"); 0 },
            8 => self.cop0_badaddr,
            12 => self.status,
            13 =>
            {
                //write!(&mut self.log_file, "CAUSE READ {} {:08x}\n", self.counter, self.cop0_cause).unwrap();
                self.cop0_cause
            },
            14 => self.cop0_epc,
            15 => 0x00000002, // Processor ID
            _  => panic!("Unsupported cop0 register: {:08x}", opcode.rd())
        };

        // Put in the load-delay slot
        self.pending_load = (opcode.rt(), value);
    }

    fn cop0_mtc(&mut self, opcode: &Opcode)
    {
        trace!("COP0 MTC | R{} = {:08x} -> COP R{}", opcode.rt(), self.reg(opcode.rt()), opcode.rd());

        match opcode.rd()
        {
            3 | 5 | 6 | 7| 9 | 11 => warn!("Ignoring write to CR{}", opcode.rd()),

            12 => {
                self.status = self.reg(opcode.rt());
                //write!(&mut self.log_file, "write STATUS {:08x} {:08x} {:x}\n", self.status, self.current_pc, self.counter);
            },

            13 =>
            {
                //let v = self.reg(opcode.rt());
                //write!(&mut self.log_file, "CAUSE WRITE {} {:08x}\n", self.counter, v).unwrap();

                // Only SW (9-8) are writeable
                error!("CAUSE {:08x}", opcode.rd());
                self.cop0_cause = (self.cop0_cause & !0x300) | (self.reg(opcode.rt()) & 0x300);
            },

            _  => panic!("Unsupported cop0 register: {:08x}", opcode.rd())
        };
    }

    fn cop0_rfe(&mut self)
    {
        trace!("COP0 RFE");

        // Shift the interrupt stack to the right (but the "old" values at the left remain unchanged)
        self.status = (self.status & !0b001111) | ((self.status >> 2) & 0b001111);
        //write!(&mut self.log_file, "RFE STATUS {:08x} {:08x} {:x}\n", self.status, self.current_pc, self.counter);
    }

    fn cop1(&mut self)
    {
        trace!("COP1");

        self.exception(Exception::CoprocessorError);
    }

    fn cop2(&mut self)
    {
        trace!("COP1");

        panic!("GTE not implemented");
    }

    fn cop3(&mut self)
    {
        trace!("COP3");

        self.exception(Exception::CoprocessorError);
    }

    fn j(&mut self, opcode: &Opcode)
    {
        trace!("J _ {:08x}", opcode.imm26());

        self.next_pc = (self.pc & 0xF0000000) | (opcode.imm26() << 2);
        self.branching = true;
    }

    fn jal(&mut self, opcode: &Opcode)
    {
        trace!("JAL _ shifted {:08x} = {:08x}", opcode.imm26(), (self.pc & 0xF0000000) | (opcode.imm26() << 2));

        self.set_reg(31, self.next_pc);
        self.next_pc = (self.pc & 0xF0000000) | (opcode.imm26() << 2);
        self.branching = true;

        if self.next_pc % 4 != 0 // TODO bit
        {
            self.alignment_exception(Exception::LoadAddress, self.next_pc, true);
        }
    }

    fn jalr(&mut self, opcode: &Opcode)
    {
        trace!("JALR _ R{}={:08x}", opcode.rs(), self.reg(opcode.rs()));

        self.set_reg(opcode.rd(), self.next_pc);
        self.next_pc = self.reg(opcode.rs());
        self.branching = true;

        if self.next_pc % 4 != 0 // TODO bit
        {
            self.alignment_exception(Exception::LoadAddress, self.next_pc, true);
        }
    }

    fn jr(&mut self, opcode: &Opcode)
    {
        trace!("JR _ R{}={:08x}", opcode.rs(), self.reg(opcode.rs()));

        self.next_pc = self.reg(opcode.rs());
        self.branching = true;

        if self.next_pc % 4 != 0 // TODO bit
        {
            self.alignment_exception(Exception::LoadAddress, self.next_pc, true);
        }
    }

    fn lui(&mut self, opcode: &Opcode)
    {
        trace!("LUI _ {:08x} << 16 = {:08x} -> R{}", opcode.imm(), opcode.imm() << 16, opcode.rt());

        self.set_reg(opcode.rt(), opcode.imm() << 16);
    }

    fn lb(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        trace!("LB _ {:08x}(R{})={:08x} -> R{}", opcode.imm_se(), opcode.rs(), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()), opcode.rt());

        if self.status & 0x10000 != 0
        {
            trace!("Cache is isolated, ignoring");
            return;
        }

        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());
        let result = self.read::<u8>(mem, address) as i8;

        let rt = opcode.rt();

        // Double delayed loads on the same register: cancel the previous load
        if self.previous_pending_load.0 == rt
        {
            self.r_next[rt as usize] = self.r[rt as usize];
        }

        self.pending_load = (rt, result as u32); // in the load-delay slot
    }

    fn lbu(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        trace!("LBU _ {:08x}(R{})={:08x} -> R{}", opcode.imm_se(), opcode.rs(), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()), opcode.rt());

        if self.status & 0x10000 != 0
        {
            trace!("Cache is isolated, ignoring");
            return;
        }



        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());
        let result = self.read::<u8>(mem, address);

        let rt = opcode.rt();

        // Double delayed loads on the same register: cancel the previous load
        if self.previous_pending_load.0 == rt
        {
            self.r_next[rt as usize] = self.r[rt as usize];
        }

        self.pending_load = (rt, result as u32); // in the load-delay slot
    }

    fn lh(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        trace!("LH _ {:08x}(R{})={:08x} -> R{}", opcode.imm_se(), opcode.rs(), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()), opcode.rt());

        if self.status & 0x10000 != 0
        {
            trace!("Cache is isolated, ignoring");
            return;
        }

        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());

        if address % 2 == 0
        {
            let result = self.read::<u16>(mem, address) as i16;

            let rt = opcode.rt();

            // Double delayed loads on the same register: cancel the previous load
            if self.previous_pending_load.0 == rt
            {
                self.r_next[rt as usize] = self.r[rt as usize];
            }

            self.pending_load = (rt, result as u32); // in the load-delay slot
        }
        else
        {
            self.alignment_exception(Exception::LoadAddress, address, false);
        }
    }

    fn lhu(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        trace!("LHU _ {:08x}(R{})={:08x} -> R{}", opcode.imm_se(), opcode.rs(), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()), opcode.rt());

        if self.status & 0x10000 != 0
        {
            trace!("Cache is isolated, ignoring");
            return;
        }

        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());

        if address % 2 == 0
        {
            let result = self.read::<u16>(mem, address);

            let rt = opcode.rt();

            // Double delayed loads on the same register: cancel the previous load
            if self.previous_pending_load.0 == rt
            {
                self.r_next[rt as usize] = self.r[rt as usize];
            }

            self.pending_load = (rt, result as u32); // in the load-delay slot
        }
        else
        {
            self.alignment_exception(Exception::LoadAddress, address, false);
        }
    }

    fn lw(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        trace!("LW _ {:08x}(R{})={:08x} -> R{}", opcode.imm_se(), opcode.rs(), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()), opcode.rt());

        if self.status & 0x10000 != 0
        {
            trace!("Cache is isolated, ignoring");
            return;
        }

        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());

        if address % 4 == 0
        {
            let rt = opcode.rt();

            // Double delayed loads on the same register: cancel the previous load
            if self.previous_pending_load.0 == rt
            {
                self.r_next[rt as usize] = self.r[rt as usize];
            }

            let value = self.read::<u32>(mem, address);
            self.pending_load = (rt, value); // in the load-delay slot
        }
        else
        {
            self.alignment_exception(Exception::LoadAddress, address, false);
        }
    }

    fn lwl(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        trace!("LWL _ {:08x}(R{})={:08x} -> R{}", opcode.imm_se(), opcode.rs(), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()), opcode.rt());

        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());

        // Bypass the load-delay slot
        let value = self.r_next[opcode.rt() as usize];

        let aligned_value = self.read::<u32>(mem, address & !3);

        let result = match address & 3
        {
            0 => (value & 0x00FFFFFF) | (aligned_value << 24),
            1 => (value & 0x0000FFFF) | (aligned_value << 16),
            2 => (value & 0x000000FF) | (aligned_value << 8),
            3 => aligned_value,
            _ => unreachable!()
        };

        let rt = opcode.rt();

        // Double delayed loads on the same register: cancel the previous load
        if self.previous_pending_load.0 == rt
        {
            self.r_next[rt as usize] = self.r[rt as usize];
        }

        self.pending_load = (rt, result);
    }

    fn lwr(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        trace!("LWR _ {:08x}(R{})={:08x} -> R{}", opcode.imm_se(), opcode.rs(), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()), opcode.rt());

        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());

        // Load-delay slot
        let value = self.r_next[opcode.rt() as usize];

        let aligned_value = self.read::<u32>(mem, address & !3);

        let result = match address & 3
        {
            0 => aligned_value,
            1 => (value & 0xFF000000) | (aligned_value >> 8),
            2 => (value & 0xFFFF0000) | (aligned_value >> 16),
            3 => (value & 0xFFFFFF00) | (aligned_value >> 24),
            _ => unreachable!()
        };

        let rt = opcode.rt();

        // Double delayed loads on the same register: cancel the previous load
        if self.previous_pending_load.0 == rt
        {
            self.r_next[rt as usize] = self.r[rt as usize];
        }

        self.pending_load = (rt, result);
    }

    fn or(&mut self, opcode: &Opcode)
    {
        trace!("OR _ R{}={:08x} | R{}={:08x} = {:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()), self.reg(opcode.rt()) ^ self.reg(opcode.rs()), opcode.rd());

        let result = self.reg(opcode.rt()) | self.reg(opcode.rs());
        self.set_reg(opcode.rd(), result);
    }

    fn xor(&mut self, opcode: &Opcode)
    {
        trace!("XOR _ R{}={:08x} ^ R{}={:08x} = {:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()), self.reg(opcode.rt()) | self.reg(opcode.rs()), opcode.rd());

        let result = self.reg(opcode.rt()) ^ self.reg(opcode.rs());
        self.set_reg(opcode.rd(), result);
    }

    fn nor(&mut self, opcode: &Opcode)
    {
        trace!("NOR _ ! R{}={:08x} | R{}={:08x} = {:08x} -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.rt(), self.reg(opcode.rt()), self.reg(opcode.rt()) | self.reg(opcode.rs()), opcode.rd());

        let result = !(self.reg(opcode.rt()) | self.reg(opcode.rs()));
        self.set_reg(opcode.rd(), result);
    }

    fn ori(&mut self, opcode: &Opcode)
    {
        trace!("ORI");

        let result = self.reg(opcode.rs()) | opcode.imm();
        self.set_reg(opcode.rt(), result);
    }

    fn xori(&mut self, opcode: &Opcode)
    {
        trace!("XORI");

        let result = self.reg(opcode.rs()) ^ opcode.imm();
        self.set_reg(opcode.rt(), result);
    }

    fn sll(&mut self, opcode: &Opcode)
    {
        trace!("SLL _ R{} << {} -> R{}", opcode.rt(), opcode.imm5(), opcode.rd());

        let result = self.reg(opcode.rt()) << opcode.imm5();
        self.set_reg(opcode.rd(), result);

    }

    fn sllv(&mut self, opcode: &Opcode)
    {
        trace!("SLLV _ R{} << {} -> R{}", opcode.rt(), self.reg(opcode.rs()) & 0b11111, opcode.rd());

        let shift = self.reg(opcode.rs()) & 0b11111;
        let result = self.reg(opcode.rt()) << shift;
        self.set_reg(opcode.rd(), result);

    }

    fn srlv(&mut self, opcode: &Opcode)
    {
        trace!("SRLV _ R{} >> {} -> R{}", opcode.rt(), self.reg(opcode.rs()) & 0b11111, opcode.rd());

        let shift = self.reg(opcode.rs()) & 0b11111;
        let result = self.reg(opcode.rt()) >> shift;
        self.set_reg(opcode.rd(), result);
    }

    fn srav(&mut self, opcode: &Opcode)
    {
        trace!("SRAV _ R{} >> {} -> R{}", opcode.rt(), self.reg(opcode.rs()) & 0b11111, opcode.rd());

        let shift = self.reg(opcode.rs()) & 0b11111;
        let result = (self.reg(opcode.rt()) as i32) >> shift;
        self.set_reg(opcode.rd(), result as u32);

    }

    fn sra(&mut self, opcode: &Opcode)
    {
        trace!("SRA _ R{} >> {} -> R{}", opcode.rt(), opcode.imm5(), opcode.rd());

        let rt = self.reg(opcode.rt()) as i32;
        let result = rt >> opcode.imm5();
        self.set_reg(opcode.rd(), result as u32);
    }

    fn srl(&mut self, opcode: &Opcode)
    {
        trace!("SRL _ R{} >> {} -> R{}", opcode.rt(), opcode.imm5(), opcode.rd());

        let rt = self.reg(opcode.rt());
        let result = rt >> opcode.imm5();
        self.set_reg(opcode.rd(), result);
    }

    fn slti(&mut self, opcode: &Opcode)
    {
        trace!("SLTI _ R{}={:08x} < {:08x} ? -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.imm_se(), opcode.rt());

        let rs = self.reg(opcode.rs()) as i32;
        let imm = opcode.imm_se() as i32;

        let result = rs < imm;

        self.set_reg(opcode.rt(), result as u32);
    }

    fn sltiu(&mut self, opcode: &Opcode)
    {
        trace!("SLTIU _ R{}={:08x} < {:08x} ? -> R{}", opcode.rs(), self.reg(opcode.rs()), opcode.imm_se(), opcode.rt());

        let rs = self.reg(opcode.rs());
        let imm = opcode.imm_se();

        let result = rs < imm;

        self.set_reg(opcode.rt(), result as u32);
    }

    fn slt(&mut self, opcode: &Opcode)
    {
        trace!("SLT _ R{} < R{} ? -> R{}", opcode.rs(), opcode.rt(), opcode.rd());

        let rs = self.reg(opcode.rs()) as i32;
        let rt = self.reg(opcode.rt()) as i32;

        let result = rs < rt;
        self.set_reg(opcode.rd(), result as u32);
    }

    fn sltu(&mut self, opcode: &Opcode)
    {
        trace!("SLTU _ R{} < R{} ? -> R{}", opcode.rs(), opcode.rt(), opcode.rd());

        let result = self.reg(opcode.rs()) < self.reg(opcode.rt());
        self.set_reg(opcode.rd(), result as u32);
    }

    fn sb(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        trace!("SB _ R{}={:08x} -> {:08x}(R{}={:08x})={:08x}", opcode.rt(), self.reg(opcode.rt()), opcode.imm_se(), opcode.rs(), self.reg(opcode.rs()), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()));

        if self.status & 0x10000 != 0
        {
            debug!("Cache is isolated, ignoring");
            return;
        }

        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());
        let value = self.reg(opcode.rt()) as u8;
        self.write::<u8>(mem, address, value);
    }

    fn sh(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        trace!("SH _ R{}={:08x} -> {:08x}(R{}={:08x})={:08x}", opcode.rt(), self.reg(opcode.rt()), opcode.imm_se(), opcode.rs(), self.reg(opcode.rs()), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()));

        if self.status & 0x10000 != 0
        {
            trace!("Cache is isolated, ignoring");
            return;
        }

        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());

        if address % 2 == 0
        {
            let value = self.reg(opcode.rt()) as u16;
            self.write::<u16>(mem, address, value);
        }
        else
        {
            self.alignment_exception(Exception::StoreAddress, address, false);
        }
    }

    fn sw(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        trace!("SW _ R{}={:08x} -> {:08x}(R{}={:08x})={:08x}", opcode.rt(), self.reg(opcode.rt()), opcode.imm_se(), opcode.rs(), self.reg(opcode.rs()), self.reg(opcode.rs()).wrapping_add(opcode.imm_se()));

        if self.status & 0x10000 != 0
        {
            trace!("Cache is isolated, ignoring");
            return;
        }

        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());

        if address % 4 == 0
        {
            let value = self.reg(opcode.rt());
            self.write::<u32>(mem, address, value);
        }
        else
        {
            self.alignment_exception(Exception::StoreAddress, address, false);
        }
    }

    fn swl(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        let value = self.reg(opcode.rt());

        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());
        let aligned_address = address & !3;
        let aligned_value = self.read::<u32>(mem, aligned_address);

        let result = match address & 3
        {
            0 => (aligned_value & 0xFFFFFF00) | (value >> 24),
            1 => (aligned_value & 0xFFFF0000) | (value >> 16),
            2 => (aligned_value & 0xFF000000) | (value >> 8),
            3 => value,
            _ => unreachable!()
        };

        //write!(&mut self.log_file, "SWL addr {:08x} = {:08x}, reg {:08x} -> {:08x}\n", address, aligned_value, value, result).unwrap();

        self.write::<u32>(mem, aligned_address, result);
    }

    fn swr(&mut self, mem: &mut Memory, opcode: &Opcode)
    {
        let value = self.reg(opcode.rt());

        let address = self.reg(opcode.rs()).wrapping_add(opcode.imm_se());
        let aligned_address = address & !3;
        let aligned_value = self.read::<u32>(mem, aligned_address);

        let result = match address & 3
        {
            0 => value,
            1 => (aligned_value & 0x000000FF) | (value << 8),
            2 => (aligned_value & 0x0000FFFF) | (value << 16),
            3 => (aligned_value & 0x00FFFFFF) | (value << 24),
            _ => unreachable!()
        };

        //write!(&mut self.log_file, "SWR addr {:08x} = {:08x}, reg {:08x} -> {:08x}\n", address, aligned_value, value, result).unwrap();

        mem.write(aligned_address, result);
    }

    fn mflo(&mut self, opcode: &Opcode)
    {
        trace!("MFLO _ LO={:08x} -> R{}", self.lo, opcode.rd());

        self.set_reg(opcode.rd(), self.lo);
    }

    fn mfhi(&mut self, opcode: &Opcode)
    {
        trace!("MFHI _ HI={:08x} -> R{}", self.hi, opcode.rd());

        self.set_reg(opcode.rd(), self.hi);
    }

    fn mtlo(&mut self, opcode: &Opcode)
    {
        trace!("MTLO _ R{} -> LO={:08x}", opcode.rs(), self.lo);

        self.lo = self.reg(opcode.rs());
    }

    fn mthi(&mut self, opcode: &Opcode)
    {
        trace!("MTHI _ R{} -> HI={:08x}", opcode.rs(), self.hi);

        self.hi = self.reg(opcode.rs());
    }

    fn alignment_exception(&mut self, exception: Exception, address: u32, branch: bool)
    {
        error!("BAD ADDR = {:08X} @ {:08X}", address, self.current_pc);
        self.cop0_badaddr = address;
        self.raise_exception(exception, if branch {address} else {self.current_pc});
    }

    fn exception(&mut self, exception: Exception)
    {
        self.raise_exception(exception, self.current_pc);
    }

    fn raise_exception(&mut self, exception: Exception, epc: u32)
    {
        //write!(&mut self.log_file, "EXCEPTION {}\n", exception.clone() as i32).unwrap();
        error!("  EXCEPTION {:?}", &exception);
        //error!("  PC {:08X}", self.pc);

        self.cop0_epc = epc;
error!("EPC = {:08X}", self.cop0_epc);
        // Update the exception's cause in the CAUSE register
        self.cop0_cause = (self.cop0_cause & !0x7C) | ((exception as u32) << 2);

        // TODO cause
        // BD if we were in the branch delay slot (31)
        // CE coprocessor number? (29-28)

        // Special case when branching:
        //   - the branch instruction is put in EPC instead of the current one
        //   - bit 31 of CAUSE is set
        if self.in_delay_slot
        {
            self.cop0_epc = self.cop0_epc.wrapping_sub(4);
            self.cop0_cause |= 1 << 31;
        }

        // Stack the exception
        self.status = (self.status & !0x3F) | ((self.status << 2) & 0x3F);
        //write!(&mut self.log_file, "exception STATUS {:08x} {:08x} {:x}\n", self.status, self.current_pc, self.counter);

        // Two possible handler addresses depending on the status' BEV bit
        let handler = if (self.status & (1 << 22)) != 0 { 0xBFC00180 } else { 0x80000080 };

        //error!("  NPC {:08X}", handler);

        self.pc = handler;
        self.next_pc = handler.wrapping_add(4);
    }

    fn syscall(&mut self)
    {
        error!("SYSCALL");

        self.exception(Exception::Syscall);
    }

    fn break_(&mut self)
    {
        trace!("BREAK");

        self.exception(Exception::Break);
    }

    fn cop0_lwc(&mut self)
    {
        self.exception(Exception::CoprocessorError);
    }

    fn cop1_lwc(&mut self)
    {
        self.exception(Exception::CoprocessorError);
    }

    fn cop2_lwc(&mut self)
    {
        panic!("unsupported cop2_lwc");
    }

    fn cop3_lwc(&mut self)
    {
        self.exception(Exception::CoprocessorError);
    }

    fn cop0_swc(&mut self)
    {
        self.exception(Exception::CoprocessorError);
    }

    fn cop1_swc(&mut self)
    {
        self.exception(Exception::CoprocessorError);
    }

    fn cop2_swc(&mut self)
    {
        panic!("unsupported cop2_swc");
    }

    fn cop3_swc(&mut self)
    {
        self.exception(Exception::CoprocessorError);
    }
}