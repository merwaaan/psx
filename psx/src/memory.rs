use crate::bios::BIOS;
use crate::dma::DMA;
use crate::spu::SPU;

use std::path::Path;

pub struct Memory
{
    bios: BIOS,
    pub spu: SPU,
    ram: Vec<u8>,
    dma: DMA,

    interrupt_status: u32,
    interrupt_mask: u32
}

impl Memory
{
    pub fn new(bios_path: &Path) -> Self
    {
        Memory
        {
            bios: BIOS::new(bios_path),
            spu: SPU::new(),
            ram: vec![0; 0x1F000000],
            dma: DMA::new(),

            interrupt_status: 0,
            interrupt_mask: 0
        }
    }

    pub fn read8(&self, addr: u32) -> u8
    {
        //println!("MEM read8 @ {:08x}", addr);
        // TODO check misaligned access

        match addr
        {
            0x00000000 ..= 0x1EFFFFFF =>  self.read_ram8(addr),
            0x1F000000 ..= 0x1F800000 => 0xFF, // fake license check
            0x1F801C00 ..= 0x1F801E80 => self.spu.read8(addr - 0x1F801C00),

            0x80000000 ..= 0x9F000000 =>  self.read_ram8(addr - 0x80000000), // TODO exclusive range
            0xBFC00000 ..= 0xBFC80000 => self.bios.read8(addr - 0xBFC00000), // TODO exclusive range

            0xA0000000 ..= 0xA0200000 => self.bios.read8(addr - 0xA0000000), // TODO exclusive range

            _ =>
            {
                warn!("Unsupported read8 @ {:08x}", addr);
                0
            }
        }
    }

    pub fn read16(&self, addr: u32) -> u16
    {
        //println!("MEM read16 @ {:08x}", addr);
        // TODO check misaligned access

        match addr
        {
            //0x1F801070 ..= 0x1F801078 => { info!("IRQ read16 @ {:08x}", addr); 0 },
            0x1F801070 => { error!("Interrupt status read16"); self.interrupt_status as u16 },
            0x1F801074 => { error!("Interrupt mask read16"); self.interrupt_mask as u16 },

            //0x1F801C00 ..= 0x1F802240 => { info!("Unhandled read from the SPU register @ {:08x}", addr); 0 },
            0x1F801C00 ..= 0x1F801E80 => self.spu.read16(addr - 0x1F801C00),

            0x80000000 ..= 0x9F000000 =>  self.read_ram16(addr - 0x80000000), // TODO exclusive range

            _ =>
            {
                warn!("Unsupported read16 @ {:08x}", addr);
                0
            }
        }
    }

    // TODO rename read32
    pub fn read(&self, addr: u32) -> u32
    {
        //println!("MEM read @ {:08x}", addr);
        // TODO check misaligned access

        match addr
        {
            0x00000000 ..= 0x1F000000 =>  self.read_ram(addr), // TODO exclusive range
            0x1F801000 ..= 0x1F801078 => 0,

            0x1F801080 ..= 0x1F8010FF => self.dma.read(addr - 0x1F801080),

            0x1F801810 ..= 0x1F801810 => { error!("unsupported GPUREAD read32"); 0 }
            0x1F801814 ..= 0x1F801814 => { error!("unsupported GPUSTAT read32"); 0x10000000 } // Ready for now (bit 28)

            0x1F801C00 ..= 0x1F801E80 => self.spu.read32(addr - 0x1F801C00),

            0x80000000 ..= 0x9F000000 =>  self.read_ram(addr - 0x80000000), // TODO exclusive range

            0xA0000000 ..= 0xBF000000 => self.read_ram(addr - 0xA0000000), // TODO exclusive range
            0xBFC00000 ..= 0xBFC80000 => self.bios.read(addr - 0xBFC00000), // TODO exclusive range

            _ =>
            {
                warn!("Unsupported read32 @ {:08x}", addr);
                0
            }
        }
    }

    pub fn write8(&mut self, addr: u32, val: u8)
    {
        //println!("MEM write8 {:08x} @ {:08x}", val, addr);

        // TODO check misaligned access

        match addr
        {
            0x00000000 ..= 0x1F000000 =>  self.write_ram8(addr, val), // TODO exclusive range
            0x1F802000 ..= 0x1F802042 => info!("Ignored write to Expansion 2"),
            //0x1F801D80 ..= 0x1F801DBC => error!("SPU control registers write8 {:02X} @ {:08X}", val, addr),
            0x1F801C00 ..= 0x1F801E80 => self.spu.write8(addr - 0x1F801C00, val),

            0x80000000 ..= 0x9F000000 =>  self.write_ram8(addr - 0x80000000, val), // TODO exclusive range

            0xA0000000 ..= 0xBF000000 =>  self.write_ram8(addr - 0xA0000000, val), // TODO exclusive range

            _                         => panic!("Unsupported write8 address: {:08x}", addr)
        }
    }

    pub fn write16(&mut self, addr: u32, val: u16)
    {
        // TODO check misaligned access

        match addr
        {
            0x1F801070 => { error!("Interrupt status write16"); self.interrupt_status = val as u32 },
            0x1F801074 => { error!("Interrupt mask write16"); self.interrupt_mask = val as u32 },

            0x1F801100 ..= 0x1F801130 => info!("Ignored write16 to the timer registers: {:08x} @ {:08x}", val, addr),
            //0x1F801C00 ..= 0x1F802240 => info!("Ignored write16 to the SPU register: {:08x} @ {:08x}", val, addr),
            //0x1F801D80 ..= 0x1F801DBC => error!("SPU control registers write16"),
            //0x1F801C00 ..= 0x1F801E80 => error!("SPU control registers write16 {:04X} @ {:08X}", val, addr),
            0x1F801C00 ..= 0x1F801E80 => self.spu.write16(addr - 0x1F801C00, val),

            0x80000000 ..= 0x9F000000 =>  self.write_ram16(addr - 0x80000000, val), // TODO exclusive range

            _                         => panic!("Unsupported write16 address: {:08x}", addr)
        }
    }

    // TODO rename write32
    pub fn write(&mut self, addr: u32, val: u32)
    {
        //println!("MEM write {:08x} @ {:08x}", val, addr);
        // TODO check misaligned access

        match addr
        {
            0x00000000 ..= 0x1F000000 =>  self.write_ram(addr, val), // TODO exclusive range

            0x1F801000 ..= 0x1F801024 => info!("Ignoring memory control 1 write"),
            0x1F801040 ..= 0x1F80105F => info!("Ignoring IO write"),
            0x1F801060 => info!("Ignoring memory control 2 write"),

            0x1F801070 => self.interrupt_status = val,
            0x1F801074 => self.interrupt_mask = val,

            0x1F801080 ..= 0x1F8010FF => info!("Ignoring DMA registers write"),

            0x1f801810 ..= 0x1F801810 => info!("unsupported GP0 write32"),
            0x1f801814 ..= 0x1F801814 => info!("unsupported GP1 write32"),

            0x1F801100 ..= 0x1F80112F => info!("Ignored write32 to the timer registers: {:08x} @ {:08x}", val, addr),
            //0x1F801D80 ..= 0x1F801DBC => error!("SPU control registers write32 {:08X} @ {:08X}", val, addr),
            0x1F801C00 ..= 0x1F801E80 => self.spu.write32(addr - 0x1F801C00, val),

            0x80000000 ..= 0x9F000000 =>  self.write_ram(addr - 0x80000000, val), // TODO exclusive range

            //0x1F801000 ..= 0x1F801024 => {},
            //0x1F801060 ..= 0x1F801060 => {},
            0xA0000000 ..= 0xA0200000 =>  self.write_ram(addr - 0xA0000000, val), // TODO exclusive range
            0xFFFE0130 ..= 0xFFFE0130 => {},
            _                         => panic!("Unsupported write32 {:08x} @ {:08x}", val, addr)
        }
    }

    // RAM

    fn read_ram(&self, addr: u32) -> u32
    {
        let offset = addr as usize;

        let b0 = self.ram[offset] as u32;
        let b1 = self.ram[offset + 1] as u32;
        let b2 = self.ram[offset + 2] as u32;
        let b3 = self.ram[offset + 3] as u32;

        (b3 << 24) | (b2 << 16) | (b1 << 8) | b0
    }

    fn read_ram16(&self, addr: u32) -> u16
    {
        let offset = addr as usize;

        let b0 = self.ram[offset] as u16;
        let b1 = self.ram[offset + 1] as u16;

        (b1 << 8) | b0
    }

    fn read_ram8(&self, addr: u32) -> u8
    {
        self.ram[addr as usize]
    }

    fn write_ram(&mut self, addr: u32, val: u32)
    {
        let offset = addr as usize;
        self.ram[offset] = val as u8;
        self.ram[offset + 1] = ((val & 0xFF00) >> 8) as u8;
        self.ram[offset + 2] = ((val & 0xFF0000) >> 16) as u8;
        self.ram[offset + 3] = ((val & 0xFF000000) >> 24) as u8;
    }

    fn write_ram16(&mut self, addr: u32, val: u16)
    {
        let offset = addr as usize;
        self.ram[offset] = val as u8;
        self.ram[offset + 1] = (val >> 8) as u8;
    }

    fn write_ram8(&mut self, addr: u32, val: u8)
    {
        self.ram[addr as usize] = val
    }
}
