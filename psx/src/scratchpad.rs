pub struct Scratchpad
{
    data: Vec<u8>
}

impl Scratchpad
{
    pub fn new() -> Scratchpad
    {
        Scratchpad
        {
            data: vec![0; 0x400]
        }
    }

    pub fn read32(&self, address: u32) -> u32
    {
        let offset = address as usize;

        let b0 = self.data[offset] as u32;
        let b1 = self.data[offset + 1] as u32;
        let b2 = self.data[offset + 2] as u32;
        let b3 = self.data[offset + 3] as u32;

        (b3 << 24) | (b2 << 16) | (b1 << 8) | b0
    }

    pub fn read16(&self, address: u32) -> u16
    {
        let offset = address as usize;

        let b0 = self.data[offset] as u16;
        let b1 = self.data[offset + 1] as u16;

        (b1 << 8) | b0
    }

    pub fn read8(&self, address: u32) -> u8
    {
        self.data[address as usize]
    }

    pub fn write32(&mut self, address: u32, calue: u32)
    {
        let offset = address as usize;
        self.data[offset] = calue as u8;
        self.data[offset + 1] = ((calue & 0xFF00) >> 8) as u8;
        self.data[offset + 2] = ((calue & 0xFF0000) >> 16) as u8;
        self.data[offset + 3] = ((calue & 0xFF000000) >> 24) as u8;
    }

    pub fn write16(&mut self, address: u32, calue: u16)
    {
        let offset = address as usize;
        self.data[offset] = calue as u8;
        self.data[offset + 1] = (calue >> 8) as u8;
    }

    pub fn write8(&mut self, address: u32, calue: u8)
    {
        self.data[address as usize] = calue
    }
}
