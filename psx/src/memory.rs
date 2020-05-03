// TODO clean up ranges (in/ex)

use crate::bios::BIOS;
use crate::cdrom::CDROM;
use crate::dma::DMA;
use crate::gpu::GPU;
use crate::interrupt_controller::InterruptController;
use crate::ram::RAM;
use crate::scratchpad::Scratchpad;
use crate::spu::SPU;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub struct Memory
{
    bios: BIOS,
    cd: CDROM,
    dma: DMA,
    pub gpu: GPU,
    ram: RAM,
    scratchpad: Scratchpad,
    pub spu: SPU,

    interrupt_controller: Rc<RefCell<InterruptController>>
}

impl Memory
{
    pub fn new(bios_path: PathBuf, display: &glium::Display, interrupt_controller: &Rc<RefCell<InterruptController>>) -> Self
    {
        Memory
        {
            bios: BIOS::new(bios_path),
            cd: CDROM::new(interrupt_controller),
            dma: DMA::new(),
            gpu: GPU::new(display),
            ram: RAM::new(),
            scratchpad: Scratchpad::new(),
            spu: SPU::new(),
            interrupt_controller: interrupt_controller.clone()
        }
    }

    // TODO mut self because of cd, clean this up?
    pub fn read8(&mut self, addr: u32) -> u8
    {
        //println!("MEM read8 @ {:08x}", addr);
        // TODO check misaligned access

        match addr
        {
            0x00000000 ..= 0x1EFF_FFFF =>  self.ram.read8(addr),
            0x1F000000 ..= 0x1F7F_FFFF => 0xFF, // fake license check
            0x1F80_0000 ..= 0x1F80_03FF => self.scratchpad.read8(addr - 0x1F80_0000),
            0x1F801800 ..= 0x1F80_1803 => self.cd.read8(addr - 0x1F801800),
            0x1F801C00 ..= 0x1F80_1E80 => self.spu.read8(addr - 0x1F801C00),

            0x80000000 ..= 0x9F00_0000 =>  self.ram.read8(addr - 0x80000000), // TODO exclusive range
            0xBFC00000 ..= 0xBFC8_0000 => self.bios.read8(addr - 0xBFC00000), // TODO exclusive range

            0xA0000000 ..= 0xA020_0000 => self.bios.read8(addr - 0xA0000000), // TODO exclusive range

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
            0x1F80_0000 ..= 0x1F80_03FF => self.scratchpad.read16(addr - 0x1F80_0000),

            0x1F80_1040 ..= 0x1F80_105F => { /*warn!("IO read16 not implemented");*/ 0xFFFF },

            //0x1F801070 ..= 0x1F801078 => { info!("IRQ read16 @ {:08x}", addr); 0 },

            0x1F801070 => self.interrupt_controller.borrow().read_status(),
            0x1F801074 => self.interrupt_controller.borrow().read_mask(),

            //0x1F801C00 ..= 0x1F802240 => { info!("Unhandled read from the SPU register @ {:08x}", addr); 0 },
            0x1F801C00 ..= 0x1F801E80 => self.spu.read16(addr - 0x1F801C00),

            0x80000000 ..= 0x9F000000 =>  self.ram.read16(addr - 0x80000000), // TODO exclusive range

            _ =>
            {
                warn!("Unsupported read16 @ {:08x}", addr);
                0
            }
        }
    }

    // TODO rename read32
    pub fn read(&mut self, addr: u32) -> u32 // TODO mut because of gpu, how to deal with this?
    {
        //println!("MEM read @ {:08x}", addr);
        // TODO check misaligned access

        match addr
        {
            0x00000000 ..= 0x1F000000 =>  self.ram.read32(addr), // TODO exclusive range
            0x1F80_0000 ..= 0x1F80_03FF => self.scratchpad.read32(addr - 0x1F80_0000),
            0x1F801000 ..= 0x1F801078 => 0,

            0x1F801080 ..= 0x1F8010FF => self.dma.read(addr - 0x1F801080),

            0x1F801810 ..= 0x1F801810 => self.gpu.read(),
            0x1F801814 ..= 0x1F801814 => self.gpu.status(),

            0x1F801C00 ..= 0x1F801E80 => self.spu.read32(addr - 0x1F801C00),

            0x80000000 ..= 0x9F000000 =>  self.ram.read32(addr - 0x80000000), // TODO exclusive range

            0xA0000000 ..= 0xBF000000 => self.ram.read32(addr - 0xA0000000), // TODO exclusive range
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
            0x00000000 ..= 0x1F000000 =>  self.ram.write8(addr, val), // TODO exclusive range
            0x1F80_0000 ..= 0x1F80_03FF => self.scratchpad.write8(addr - 0x1F80_0000, val),

            0x1F80_1040 ..= 0x1F80_105F => warn!("IO write8 not implemented"),
            0x1F802000 ..= 0x1F802042 => info!("Ignored write to Expansion 2"),
            //0x1F801D80 ..= 0x1F801DBC => error!("SPU control registers write8 {:02X} @ {:08X}", val, addr),

            0x1F801800 ..= 0x1F801803 => self.cd.write8(addr - 0x1F801800, val), // CD

            0x1F801C00 ..= 0x1F801E80 => self.spu.write8(addr - 0x1F801C00, val),

            0x80000000 ..= 0x9F000000 =>  self.ram.write8(addr - 0x80000000, val), // TODO exclusive range

            0xA0000000 ..= 0xBF000000 =>  self.ram.write8(addr - 0xA0000000, val), // TODO exclusive range

            _                         => panic!("Unsupported write8 {:08X} @ {:08x}", val, addr)
        }
    }

    pub fn write16(&mut self, addr: u32, val: u16)
    {
        // TODO check misaligned access

        match addr
        {
            0x1F80_0000 ..= 0x1F80_03FF => self.scratchpad.write16(addr - 0x1F80_0000, val),

            0x1F80_1040 ..= 0x1F80_105F => warn!("IO write16 not implemented"),

            0x1F801070 => self.interrupt_controller.borrow_mut().write_status(val),
            0x1F801074 => self.interrupt_controller.borrow_mut().write_mask(val),

            0x1F801100 ..= 0x1F801130 => info!("Ignored write16 to the timer registers: {:08x} @ {:08x}", val, addr),
            //0x1F801C00 ..= 0x1F802240 => info!("Ignored write16 to the SPU register: {:08x} @ {:08x}", val, addr),
            //0x1F801D80 ..= 0x1F801DBC => error!("SPU control registers write16"),
            //0x1F801C00 ..= 0x1F801E80 => error!("SPU control registers write16 {:04X} @ {:08X}", val, addr),
            0x1F801C00 ..= 0x1F801E80 => self.spu.write16(addr - 0x1F801C00, val),

            0x80000000 ..= 0x9F000000 =>  self.ram.write16(addr - 0x80000000, val), // TODO exclusive range

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
            0x00000000 ..= 0x1F000000 =>  self.ram.write32(addr, val), // TODO exclusive range
            0x1F80_0000 ..= 0x1F80_03FF => self.scratchpad.write32(addr - 0x1F80_0000, val),

            0x1F801000 ..= 0x1F801024 => info!("Ignoring memory control 1 write"),
            0x1F801040 ..= 0x1F80105F => info!("Ignoring IO write"),
            0x1F801060 => info!("Ignoring memory control 2 write"),

            0x1F801070 => self.interrupt_controller.borrow_mut().write_status(val as u16),
            0x1F801074 => self.interrupt_controller.borrow_mut().write_mask(val as u16),

            0x1F801080 ..= 0x1F8010FF => self.dma.write(addr - 0x1F801080, val, &mut self.ram, &mut self.gpu),

            0x1f801810 ..= 0x1F801810 => self.gpu.gp0(val),
            0x1f801814 ..= 0x1F801814 => self.gpu.gp1(val),

            0x1F801100 ..= 0x1F80112F => info!("Ignored write32 to the timer registers: {:08x} @ {:08x}", val, addr),
            //0x1F801D80 ..= 0x1F801DBC => error!("SPU control registers write32 {:08X} @ {:08X}", val, addr),
            0x1F801C00 ..= 0x1F801E80 => self.spu.write32(addr - 0x1F801C00, val),

            0x80000000 ..= 0x9F000000 =>  self.ram.write32(addr - 0x80000000, val), // TODO exclusive range

            //0x1F801000 ..= 0x1F801024 => {},
            //0x1F801060 ..= 0x1F801060 => {},
            0xA0000000 ..= 0xA0200000 =>  self.ram.write32(addr - 0xA0000000, val), // TODO exclusive range
            0xFFFE0130 ..= 0xFFFE0130 => {},
            _                         => panic!("Unsupported write32 {:08x} @ {:08x}", val, addr)
        }
    }
}
