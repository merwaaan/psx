// TODO clean up ranges (in/ex)
// TODO refactor this to be the PSX?

use crate::bios::BIOS;
use crate::cdrom::CDROM;
use crate::dma::DMA;
use crate::gpu::GPU;
use crate::interrupt_controller::InterruptController;
use crate::memory_segment::MemorySegment;
use crate::spu::SPU;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub enum Width
{
    Byte = 1,
    Half = 2,
    Word = 4
}

pub trait Addressable
{
    fn width() -> Width;

    fn from_u32(value: u32) -> Self;
    fn from_u16(value: u16) -> Self;
    fn from_u8(value: u8) -> Self;

    fn as_u32(&self) -> u32;
    fn as_u16(&self) -> u16;
    fn as_u8(&self) -> u8;
}

impl Addressable for u8
{
    fn width() -> Width { Width::Byte }

    fn from_u32(value: u32) -> Self { value as u8 }
    fn from_u16(value: u16) -> Self { value as u8 }
    fn from_u8(value: u8) -> Self { value }

    fn as_u32(&self) -> u32 { *self as u32 }
    fn as_u16(&self) -> u16 { *self as u16 }
    fn as_u8(&self) -> u8 { *self }
}

impl Addressable for u16
{
    fn width() -> Width { Width::Half }

    fn from_u32(value: u32) -> Self { value as u16 }
    fn from_u16(value: u16) -> Self { value }
    fn from_u8(value: u8) -> Self { value as u16 }

    fn as_u32(&self) -> u32 { *self as u32 }
    fn as_u16(&self) -> u16 { *self }
    fn as_u8(&self) -> u8 { *self as u8 }
}

impl Addressable for u32
{
    fn width() -> Width { Width::Word }

    fn from_u32(value: u32) -> Self { value }
    fn from_u16(value: u16) -> Self { value as u32 }
    fn from_u8(value: u8) -> Self { value as u32 }

    fn as_u32(&self) -> u32 { *self }
    fn as_u16(&self) -> u16 { *self as u16 }
    fn as_u8(&self) -> u8 { *self as u8 }
}

pub struct Memory
{
    bios: BIOS,
    cd: CDROM,
    dma: DMA,
    pub gpu: GPU,
    ram: MemorySegment,
    scratchpad: MemorySegment,
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
            ram: MemorySegment::new(0x1F00_0000),
            scratchpad: MemorySegment::new(0x400),
            spu: SPU::new(),
            interrupt_controller: interrupt_controller.clone()
        }
    }

    pub fn read<T: Addressable>(&mut self, address: u32) -> T
    {
        match address
        {
            // KUSEG

            0x0000_0000 ..= 0x1EFF_FFFF =>  self.ram.read(address), //  TODO smaller range?

            0x1F00_0000 ..= 0x1F7F_FFFF => T::from_u32(0xFF), // fake license check

            0x1F80_0000 ..= 0x1F80_03FF => self.scratchpad.read(address - 0x1F80_0000),

            0x1F80_1040 ..= 0x1F80_105F => { warn!("Ignoring IO read {:08x}", address); T::from_u32(0xFFFF) },
            0x1F80_1070 => T::from_u16(self.interrupt_controller.borrow().read_status()),
            0x1F80_1074 => T::from_u16(self.interrupt_controller.borrow().read_mask()),
            0x1F80_1080 ..= 0x1F80_10FF => self.dma.read(address - 0x1F80_1080),
            0x1F80_1100 ..= 0x1F80_112F => { warn!("Ignoring Timer read {:08x}", address); T::from_u32(0) },
            0x1F80_1800 ..= 0x1F80_1803 => self.cd.read(address - 0x1F80_1800),
            0x1F80_1810 => T::from_u32(self.gpu.read()),
            0x1F80_1814 => T::from_u32(self.gpu.status()),
            0x1F80_1C00 ..= 0x1F80_1E80 => T::from_u16(self.spu.read(address - 0x1F80_1C00)),

            // TODO bios here

            // KSEG0

            0x8000_0000 ..= 0x9F00_0000 =>  self.ram.read(address - 0x8000_0000), // TODO exclusive range

            // KSEG1

            0xA000_0000 ..= 0xBF00_0000 => self.ram.read(address - 0xA000_0000), // TODO exclusive range

            0xBFC0_0000 ..= 0xBFC8_0000 => self.bios.read(address - 0xBFC0_0000), // TODO exclusive range

            _ => panic!("Unsupported read {:?} @ {:08x}", T::width(), address)
        }
    }

    pub fn write<T: Addressable>(&mut self, address: u32, value: T)
    {
        match address
        {
            // KUSEG

            0x0000_0000 ..= 0x1F00_0000 =>  self.ram.write(address, value), // TODO exclusive range

            0x1F80_0000 ..= 0x1F80_03FF => self.scratchpad.write(address - 0x1F80_0000, value),

            0x1F80_1000 ..= 0x1F80_1024 => warn!("Ignoring memory control 1 write"),
            0x1F80_1040 ..= 0x1F80_105F => warn!("Ignoring IO write"),
            0x1F80_1060 => warn!("Ignoring memory control 2 write"),
            0x1F80_1070 => self.interrupt_controller.borrow_mut().write_status(value.as_u16()),
            0x1F80_1074 => self.interrupt_controller.borrow_mut().write_mask(value.as_u16()),
            0x1F80_1080 ..= 0x1F80_10FF => self.dma.write(address - 0x1F80_1080, value, &mut self.ram, &mut self.gpu),
            0x1F80_1100 ..= 0x1F80_112F => warn!("Ignoring write to the timer registers: {:08x} @ {:08x}", value.as_u32(), address),
            0x1F80_1800 ..= 0x1F80_1803 => self.cd.write(address - 0x1F80_1800, value),
            0x1f80_1810  => self.gpu.gp0(value.as_u32()),
            0x1f80_1814 => self.gpu.gp1(value.as_u32()),
            0x1F80_1C00 ..= 0x1F80_1E80 => self.spu.write(address - 0x1F80_1C00, value.as_u16()),
            0x1F80_2000 ..= 0x1F80_2042 => warn!("Ignoring write to Expansion 2"),

            // KSEG1

            0x8000_0000 ..= 0x9F00_0000 =>  self.ram.write(address - 0x8000_0000, value), // TODO exclusive range

            // KSEG2

            0xA000_0000 ..= 0xA020_0000 =>  self.ram.write(address - 0xA000_0000, value), // TODO exclusive range

            0xFFFE_0130 ..= 0xFFFE_0130 => warn!("Ignoring write to memory control 3 {:?} {:08x}", T::width(), address),

            _ => panic!("Unsupported write {:?} {:08X} @ {:08x}", T::width(), value.as_u32(), address)
        }
    }
}
