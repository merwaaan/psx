pub struct DMA
{
    control: u32, // Bit 3 of byte n set: DMA channel n enabled

    // Interrupt register
    irq_enable: bool,
    irq_channel_enable: u8,
    irq_channel_flags: u8,
    irq_force: bool,
    irq_unknown: u8
}

impl DMA
{
    pub fn new() -> DMA
    {
        DMA
        {
            control: 0x7654321, // Initial value according to Nocash

            irq_enable: false,
            irq_channel_enable: 0,
            irq_channel_flags: 0,
            irq_force: false,
            irq_unknown: 0
        }
    }

    pub fn read(&self, offset: u32) -> u32
    {
        match offset
        {
            0x70 => self.control,
            0x74 => self.interrupt_register(),
            _    => { error!("unsupported DMA read"); 0 }
        }
    }

    pub fn write(&mut self, offset: u32, value: u32)
    {
        match offset
        {
            0x70 => self.control = value,
            0x74 => self.set_interrupt_register(value),
            _    => panic!("unsupported DMA write")
        }
    }

    fn interrupt_register(&self) -> u32
    {
        let master_flag = self.irq_force || (self.irq_enable && (self.irq_channel_enable & self.irq_channel_flags) != 0);

        (master_flag as u32) << 31 |
        (self.irq_channel_flags as u32) << 24 |
        (self.irq_enable as u32) << 23 |
        (self.irq_channel_enable as u32) << 16 |
        (self.irq_force as u32) << 15 |
        (self.irq_unknown as u32)
    }

    fn set_interrupt_register(&mut self, value: u32)
    {
        self.irq_enable = ((value >> 23) & 1) != 0;
        self.irq_channel_enable = ((value >> 16) & 0x7F) as u8;
        self.irq_force = ((value >> 15) & 1) != 0;
        self.irq_unknown = (value & 0x3F) as u8;

        // Write 1 to flag -> reset it
        let reset = ((value >> 24) & 0x7F) as u8;
        self.irq_channel_flags &= !reset;
    }
}
