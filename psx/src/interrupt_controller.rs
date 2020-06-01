#[derive(Debug, Copy, Clone)]
pub enum InterruptRequest
{
    VBlank = 0,
    GPU = 1,
    CDROM = 2,
    DMA = 3,
    Timer0 = 4,
    Timer1 = 5,
    Timer2 = 6,
    Controller = 7, // memcard?
    SIO = 8, // ???
    SPU = 9
    // IRQ10???
}

pub struct InterruptController
{
    interrupt_status: u16,
    interrupt_mask: u16,
}

// Only bits 10-0 are meaningful
const INTERRUPT_REGISTER_MASK: u16 = 0x7FF;

impl InterruptController
{
    pub fn new() -> Self
    {
        InterruptController
        {
            interrupt_status: 0,
            interrupt_mask: 0
        }
    }

    pub fn read_status(&self) -> u16
    {
        self.interrupt_status
    }

    pub fn write_status(&mut self, value: u16)
    {
        self.interrupt_status &= value & INTERRUPT_REGISTER_MASK;
        //error!("interrupt status set {:b}", self.interrupt_status );
    }

    pub fn read_mask(&self) -> u16
    {
        self.interrupt_mask
    }

    pub fn write_mask(&mut self, value: u16)
    {
        self.interrupt_mask = value & INTERRUPT_REGISTER_MASK;
        //error!("interrupt mask set {:b}", self.interrupt_mask );
    }

    pub fn request(&mut self, request: InterruptRequest)
    {
        error!("REQUEST");
        error!("interrupt req {:?}", request);
        error!("interrupt req val {:b}", request as u16);
        error!("interrupt req val {:b}", self.interrupt_status | (1 << (request as u16)));

        self.interrupt_status |= 1 << (request as u16);
    }

    pub fn pending(&self, p: bool) -> bool
    {
        /*if p
        {
            error!("stat {:b}", self.interrupt_status);
            error!("mask {:b}", self.interrupt_mask);
        }*/
        (self.interrupt_status & self.interrupt_mask) != 0
    }
}