use crate::cpu::CPU;
use crate::memory::Memory;
use crate::opcode::Opcode;

use serde::{ Serialize, Deserialize };
use std::fs::File;
use std::io::{ Read, Write };

pub struct Disassembly
{
    pub bits: u32,
    pub mnemonics: String
}

#[derive(Serialize, Deserialize, Debug)]
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

    pub fn save(&self, file_path: &str) -> std::io::Result<()>
    {
        let serialized = serde_json::to_string(&self)?;

        let mut file = File::create(file_path)?;
        file.write_all(serialized.as_bytes())?;

        Ok(())
    }

    pub fn load(&mut self, file_path: &str) -> std::io::Result<()>
    {
        let mut file = File::open(file_path)?;

        let mut serialized = String::new();
        file.read_to_string(&mut serialized)?;

        let deserialized: Debugger = serde_json::from_str(&serialized)?;

        self.breakpoints = deserialized.breakpoints;

        Ok(())
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

    pub fn disassemble(&self, pc: u32, _cpu: &CPU, mem: &Memory) -> Disassembly
    {
        let bits = mem.read(pc);
        let opcode = Opcode(bits);

        let template = match opcode.instr()
        {
            0b000000 =>
            {
                match opcode.sub()
                {
                    0b000000 => "SLL $rd, $rs, $shift",
                    0b000010 => "SRL $rd, $rs, $shift",
                    0b000011 => "SRA $rd, $rs, $shift",
                    0b000100 => "SLLV $rd, $rt, $rs",
                    0b000111 => "SRAV $rd, $rt, $rs",
                    0b001000 => "JR $rs",
                    0b001001 => "JALR $rs, $rd",
                    0b001100 => "SYSCALL",
                    0b001101 => "BREAK",
                    0b010000 => "MFHI $rd",
                    0b010001 => "MTHI $rd",
                    0b010010 => "MFLO $rd",
                    0b010011 => "MTLO $rd",
                    0b011000 => "MULT $rs, $rt",
                    0b011001 => "MULTU $rs, $rt",
                    0b011010 => "DIV $rs, $rt",
                    0b011011 => "DIVU $rs, $rt",
                    0b100000 => "ADD $rd, $rs, $rt",
                    0b100001 => "ADDU $rd, $rs, $rt",
                    0b100010 => "SUB $rd, $rs, $rt",
                    0b100011 => "SUBU $rd, $rs, $rt",
                    0b100100 => "AND $rd, $rs, $rt",
                    0b100101 => "OR $rd, $rs, $rt",
                    0b100110 => "XOR $rd, $rs, $rt",
                    0b100111 => "NOR $rd, $rs, $rt",
                    0b101010 => "SLT $rd, $rs, $rt",
                    0b101011 => "SLTU $rd, $rs, $rt",
                    _        => "UNKNOWN"
                }
            },
            0b000001 =>
            {
                match opcode.rt()
                {
                    0b00000 => "BLTZ $rs, $offset",
                    0b00001 => "BGEZ $rs, $offset",
                    0b10000 => "BLTZAL $rs, $offset",
                    0b10001 => "BGEZAL $rs, $offset",
                    _       => "UNKNOWN"
                }
            },
            0b000010 => "J $target",
            0b000011 => "JAL $target",
            0b000100 => "BEQ $rs, $rt, $offset",
            0b000101 => "BNE $rs, $rt, $offset",
            0b000110 => "BLEZ $rs, $rt, $offset",
            0b000111 => "BGTZ $rs, $rt, $offset",
            0b001000 => "ADDI $rt, $rs, $imm",
            0b001001 => "ADDIU $rt, $rs, $imm",
            0b001010 => "SLTI $rt, $rs, $imm",
            0b001011 => "SLTIU $rt, $rs, $imm",
            0b001100 => "ANDI $rt, $rs, $imm",
            0b001101 => "ORI $rt, $rs, $imm",
            0b001110 => "XORI $rt, $rs, $imm",
            0b001111 => "LUI $rt, $imm",
            0b010000 =>
            {
                match opcode.rs()
                {
                    0b00000 => "MFC $rt, cop$rd",
                    0b00100 => "MTC $rt, cop$rd",
                    0b10000 => "RFE",
                    _       => "UNKNOWN"
                }
            },
            0b100000 => "LB $rt, $offset",
            0b100001 => "LH $rt, $offset",
            0b100010 => "LWL $rt, $offset",
            0b100011 => "LW $rt, $offset",
            0b100100 => "LBU $rt, $offset",
            0b100101 => "LHU $rt, $offset",
            0b100110 => "LWR $rt, $offset",
            0b101000 => "SB $rt, $offset",
            0b101001 => "SH $rt, $offset",
            0b101010 => "SWL $rt, $offset",
            0b101011 => "SW $rt, $offset",
            0b101110 => "SWR $rt, $offset",
            0b110000 => "LCW0",
            0b110001 => "LCW1",
            0b110010 => "LCW2",
            0b110011 => "LCW3",
            0b111000 => "SCW0",
            0b111001 => "SCW1",
            0b111010 => "SCW2",
            0b111011 => "SCW3",
            _        => "UNKNOWN"
        };

        let mut mnemonics = String::from(template);
        mnemonics = mnemonics.replace("$rd", &format!("R{}", opcode.rd()));
        mnemonics = mnemonics.replace("$rs", &format!("R{}", opcode.rs()));
        mnemonics = mnemonics.replace("$rt", &format!("R{}", opcode.rt()));
        mnemonics = mnemonics.replace("$imm", &format!("${:04X}", opcode.imm()));
        mnemonics = mnemonics.replace("$offset", &format!("${:04X}(R{})", opcode.imm(), opcode.rs()));
        mnemonics = mnemonics.replace("$shift", &format!("${}", opcode.imm5()));
        mnemonics = mnemonics.replace("$target", &format!("${:08X}", (pc & 0xF0000000) | (opcode.imm26() << 2)));

        Disassembly
        {
            bits,
            mnemonics
        }
    }
}
