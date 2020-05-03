use crate::memory::{Addressable};

pub struct MemorySegment
{
    data: Vec<u8>
}

impl MemorySegment
{
    pub fn new(size: usize) -> Self
    {
        MemorySegment
        {
            data: vec![0; size]
        }
    }

    pub fn from_buffer(buffer: Vec<u8>) -> Self
    {
        MemorySegment
        {
            data: buffer
        }
    }

    pub fn read<T: Addressable>(&self, address: u32) -> T
    {
        let offset = address as usize;

        let mut value = 0u32;

        for i in 0 .. T::width() as usize
        {
            let byte = self.data[offset + i];
            value |= (byte as u32) << (i * 8);
        }

        T::from_u32(value)
    }

    pub fn write<T: Addressable>(&mut self, address: u32, value: T)
    {
        let offset = address as usize;
        let value = value.as_u32();

        for i in 0 .. T::width() as usize
        {
            let byte = value >> (i * 8);
            self.data[offset + i] = byte as u8;
        }
    }
}
