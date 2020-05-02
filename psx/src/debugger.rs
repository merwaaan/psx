use crate::cpu::CPU;
use crate::memory::Memory;
use crate::opcode::Opcode;

use serde::{ Serialize, Deserialize };
use std::fs::File;
use std::io::{ Read, Write };

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegisterCondition
{
    pub register: u8,
    pub value: u32,
    //pub comparison: Comparison
}

impl RegisterCondition
{
    pub fn is_matched(&self, cpu: &CPU) -> bool
    {
        cpu.r[self.register as usize] == self.value
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Breakpoint
{
    pub address: u32,
    pub enabled: bool,
    pub register_conditions: Vec<RegisterCondition>
}

impl Breakpoint
{
    pub fn new(address: u32) -> Self
    {
        Breakpoint
        {
            address,
            enabled: true,
            register_conditions: Vec::new()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataBreakpoint
{
    pub address: u32,
    pub on_write: bool,
    pub on_read: bool
}

impl DataBreakpoint
{
    pub fn new(address: u32) -> Self
    {
        DataBreakpoint
        {
            address,
            on_write: true,
            on_read: true
        }
    }
}

pub struct Disassembly
{
    // Raw bits
    pub bits: u32,

    // Disassembled instruction
    pub mnemonics: String,

    // Additional description such as the actual jump address
    pub hint: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Debugger
{
    breakpoints: Vec<Breakpoint>,
    data_breakpoints: Vec<DataBreakpoint>,

    // Data breakpoints hit since the latest check
    #[serde(skip)]
    data_breakpoints_hit: Vec<u32>
}

impl Debugger
{
    pub fn new() -> Self
    {
        Debugger
        {
            breakpoints: Vec::new(),
            data_breakpoints: Vec::new(),
            data_breakpoints_hit: Vec::new()
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
        self.data_breakpoints = deserialized.data_breakpoints;

        Ok(())
    }

    // Breakpoints

    pub fn add_breakpoint(&mut self, address: u32)
    {
        if !self.breakpoints.iter().any(|b| b.address == address)
        {
            self.breakpoints.push(Breakpoint::new(address));
        }
    }

    pub fn remove_breakpoint(&mut self, address: u32)
    {
        self.breakpoints.retain(|b| b.address != address);
    }

    pub fn toggle_breakpoint(&mut self, address: u32, enabled: bool)
    {
        match self.breakpoints.iter_mut().find(|b| b.address == address)
        {
            Some(b) => b.enabled = enabled,
            None => error!("cannot toggle breakpoint {:08X} as it does not exist", address)
        }
    }

    pub fn add_breakpoint_condition(&mut self, address: u32)
    {
        match self.breakpoints.iter_mut().find(|b| b.address == address)
        {
            Some(b) => b.register_conditions.push(RegisterCondition{ register: 1, value: 0 }),
            None => error!("cannot add condition to breakpoint {:08X} as it does not exist", address)
        }
    }

    pub fn get_breakpoint_conditions_mut(&mut self, address: u32) -> &mut[RegisterCondition]
    {
        match self.breakpoints.iter_mut().find(|b| b.address == address)
        {
            Some(b) => &mut b.register_conditions,
            None => panic!("cannot get conditions for breakpoint {:08X} as it does not exist", address)
        }
    }

    pub fn set_breakpoint_condition(&mut self, address: u32, condition_index: usize, reg: u8, value: u32)
    {
        match self.breakpoints.iter_mut().find(|b| b.address == address)
        {
            Some(b) =>
            {
                // TODO check index valid
                let condition = &mut b.register_conditions[condition_index];
                condition.register = reg;
                condition.value = value;
            }
            None => error!("cannot set condition for breakpoint {:08X} as it does not exist", address)
        }
    }

    pub fn is_breakpoint(&self, address: u32, cpu: &CPU) -> bool
    {
        self.breakpoints.iter().any(|b|
            b.address == address &&
            b.enabled &&
            (b.register_conditions.is_empty() || b.register_conditions.iter().any(|c| c.is_matched(cpu))))
    }

    pub fn get_breakpoints(&self) -> &[Breakpoint]
    {
        self.breakpoints.as_slice()
    }

    // Data breakpoints

    pub fn add_data_breakpoint(&mut self, address: u32)
    {
        if !self.data_breakpoints.iter().any(|b| b.address == address)
        {
            self.data_breakpoints.push(DataBreakpoint::new(address));
        }
    }

    pub fn remove_data_breakpoint(&mut self, address: u32)
    {
        self.data_breakpoints.retain(|b| b.address != address);
    }

    pub fn get_data_breakpoints_mut(&mut self) -> &mut[DataBreakpoint]
    {
        &mut self.data_breakpoints
    }

    pub fn get_data_breakpoints_hit(&self) -> &[u32]
    {
        &self.data_breakpoints_hit
    }

    pub fn register_data_access(&mut self, address: u32, read: bool)
    {
        if self.data_breakpoints.iter().any(|b| b.address == address && ((read && b.on_read) || (!read && b.on_write)))
        {
            self.data_breakpoints_hit.push(address);
        }
    }

    pub fn clear_data_access(&mut self)
    {
        self.data_breakpoints_hit.clear();
    }

    pub fn has_data_breakpoint(&self) -> bool
    {
        !self.data_breakpoints_hit.is_empty()
    }

    pub fn is_data_breakpoint(&self, address: u32) -> bool
    {
        self.data_breakpoints_hit.iter().any(|a| *a == address)
    }

    // Disassembly

    pub fn disassemble(&self, pc: u32, cpu: &CPU, mem: &mut Memory) -> Disassembly
    {
        let bits = mem.read(pc);
        let opcode = Opcode(bits);

        let template = match opcode.instr()
        {
            0b000000 =>
            {
                match opcode.sub()
                {
                    0b000000 => "SLL $rd, $rt, $shift",
                    0b000010 => "SRL $rd, $rt, $shift",
                    0b000011 => "SRA $rd, $rt, $shift",
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
                    _        => "[UNKNOWN]"
                }
            },
            0b000001 =>
            {
                match opcode.rt() & 0b10001
                {
                    0b00000 => "BLTZ $rs, $jumpoffset",
                    0b00001 => "BGEZ $rs, $jumpoffset",
                    0b10000 => "BLTZAL $rs, $jumpoffset",
                    0b10001 => "BGEZAL $rs, $jumpoffset",
                    _       => "[UNKNOWN]"
                }
            },
            0b000010 => "J $target",
            0b000011 => "JAL $target",
            0b000100 => "BEQ $rs, $rt, $jumpoffset",
            0b000101 => "BNE $rs, $rt, $jumpoffset",
            0b000110 => "BLEZ $rs, $rt, $jumpoffset",
            0b000111 => "BGTZ $rs, $rt, $jumpoffset",
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
                    _       => "[UNKNOWN]"
                }
            },
            0b100000 => "LB $rt, $regoffset",
            0b100001 => "LH $rt, $regoffset",
            0b100010 => "LWL $rt, $regoffset",
            0b100011 => "LW $rt, $regoffset",
            0b100100 => "LBU $rt, $regoffset",
            0b100101 => "LHU $rt, $regoffset",
            0b100110 => "LWR $rt, $regoffset",
            0b101000 => "SB $rt, $regoffset",
            0b101001 => "SH $rt, $regoffset",
            0b101010 => "SWL $rt, $regoffset",
            0b101011 => "SW $rt, $regoffset",
            0b101110 => "SWR $rt, $regoffset",
            0b110000 => "LCW0",
            0b110001 => "LCW1",
            0b110010 => "LCW2",
            0b110011 => "LCW3",
            0b111000 => "SCW0",
            0b111001 => "SCW1",
            0b111010 => "SCW2",
            0b111011 => "SCW3",
            _        => "[UNKNOWN]"
        };

        // Expand hints

        let mut hint = String::new();

        if template.contains("$regoffset")
        {
            let target = opcode.imm_se().wrapping_add(cpu.reg(opcode.rs()));
            hint.push_str(&format!("{:08X}", target));
        }

        if template.contains("$jumpoffset")
        {
            let target = pc.wrapping_add(4 + (opcode.imm_se() << 2));
            hint.push_str(&format!("{:08X} ({}) ", target, if target < pc {"UP"} else {"DOWN"})); // TODO use unicode arrows
        }

        // Replace markers with their actual values

        let mut mnemonics = String::from(template);
        mnemonics = mnemonics.replace("$rd", &format!("R{}", opcode.rd()));
        mnemonics = mnemonics.replace("$rs", &format!("R{}", opcode.rs()));
        mnemonics = mnemonics.replace("$rt", &format!("R{}", opcode.rt()));
        mnemonics = mnemonics.replace("$imm", &format!("${:04X}", opcode.imm()));
        mnemonics = mnemonics.replace("$regoffset", &format!("${:04X}(R{})", opcode.imm(), opcode.rs()));
        mnemonics = mnemonics.replace("$jumpoffset", &format!("${:04X}", opcode.imm_se() << 2));
        mnemonics = mnemonics.replace("$shift", &format!("${}", opcode.imm5()));
        mnemonics = mnemonics.replace("$target", &format!("${:08X}", (pc & 0xF0000000) | (opcode.imm26() << 2)));

        Disassembly
        {
            bits,
            mnemonics,
            hint
        }
    }
}
