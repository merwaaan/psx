use crate::gpu::GPU;
use crate::ram::RAM;

#[derive(Debug, Copy, Clone)]
enum TransferDirection
{
    ToRAM = 0,
    FromRAM = 1
}

#[derive(Debug, Copy, Clone)]
enum SyncMode
{
    Manual = 0,
    Request = 1,
    LinkedList = 2
}

//#[derive(Copy, Clone)]
pub struct Channel
{
    // Base address

    base_address: u32,

    // Block control

    // Number of blocks, only used in Request mode
    block_count: u16,

    // Manual mode: number of words to transfer
    // Request mode: size of a block
    transfer_size: u16,

    // Control

    unknown: u8,
    trigger: bool,
    enable: bool,
    chopping_cpu_window: u8,
    chopping_dma_window: u8,
    sync_mode: SyncMode,
    chopping_enable: bool,
    increment: bool, // decrement if false
    direction: TransferDirection
}

impl Channel
{
    pub fn new() -> Channel
    {
        Channel
        {
            base_address: 0,

            block_count: 0,
            transfer_size: 0,

            unknown: 0,
            trigger : false,
            enable: false,
            chopping_cpu_window: 0,
            chopping_dma_window: 0,
            sync_mode: SyncMode::Manual,
            chopping_enable: false,
            increment: true,
            direction: TransferDirection::ToRAM
        }
    }

    pub fn block_control_register(&self) -> u32
    {
        (self.block_count as u32) << 16 |
        self.transfer_size as u32
    }

    pub fn set_block_control_register(&mut self, value: u32)
    {
        self.block_count = (value >> 16) as u16;
        self.transfer_size = value as u16;
    }

    pub fn control_register(&self) -> u32
    {
        (self.unknown as u32) << 29 |
        (self.trigger as u32) << 28 |
        (self.enable as u32) << 24 |
        (self.chopping_cpu_window as u32) << 20 |
        (self.chopping_dma_window as u32) << 16 |
        (self.sync_mode as u32) << 9 |
        (self.chopping_enable as u32) << 8 |
        (self.increment as u32) << 1 |
        self.direction as u32
    }

    pub fn set_control_register(&mut self, value: u32)
    {
        self.unknown = ((value >> 29) & 3) as u8;
        self.trigger = ((value >> 28) & 1) != 0;
        self.enable = ((value >> 24) & 1) != 0;
        self.chopping_cpu_window = ((value >> 20) & 7) as u8;
        self.chopping_dma_window = ((value >> 16) & 7) as u8;
        self.sync_mode = match (value >> 9) & 3
        {
            0 => SyncMode::Manual,
            1 => SyncMode::Request,
            2 => SyncMode::LinkedList,
            x => panic!("unknown DMA sync mode {}", x)
        };
        self.chopping_enable = ((value >> 8) & 1) != 0;
        self.increment = ((value >> 1) & 1) == 0;
        self.direction = if (value & 1) == 0 {TransferDirection::ToRAM} else {TransferDirection::FromRAM};
    }

    pub fn is_active(&self) -> bool
    {
        // In Manual sync mode, the trigger must be set to start the transfer
        let trigger = match self.sync_mode
        {
            SyncMode::Manual => self.trigger,
            _                => true
        };

        self.enable && trigger
    }
}

#[derive(Debug, Copy, Clone)]
enum Port
{
    MDECin = 0,
    MDECout = 1,
    GPU = 2,
    CDROM = 3,
    SPU = 4,
    PIO = 5,
    OTC = 6
}

impl Port
{
    pub fn from_index(index: u32) -> Port
    {
        match index
        {
            0 => Port::MDECin,
            1 => Port::MDECout,
            2 => Port::GPU,
            3 => Port::CDROM,
            4 => Port::SPU,
            5 => Port::PIO,
            6 => Port::OTC,
            n => panic!("unsupported port {}", n)
        }
    }
}

pub struct DMA
{
    channels: [Channel; 7],

    control: u32, // Bit 3 of byte n set: DMA channel n enabled

    // Interrupt register
    irq_enable: bool,
    irq_channel_enable: u8,
    irq_channel_status: u8,
    irq_force: bool,
    irq_unknown: u8
}

impl DMA
{
    pub fn new() -> DMA
    {
        DMA
        {
            channels: [Channel::new(), Channel::new(), Channel::new(), Channel::new(), Channel::new(), Channel::new(), Channel::new()],

            control: 0x7654321, // Initial value according to Nocash

            irq_enable: false,
            irq_channel_enable: 0,
            irq_channel_status: 0,
            irq_force: false,
            irq_unknown: 0
        }
    }

    pub fn read(&self, offset: u32) -> u32
    {
        match offset
        {
            0 ..= 0x6F =>
            {
                let port = Port::from_index((offset >> 4) & 7);
                let channel = &self.channels[port as usize];

                match offset & 0xF
                {
                    0 => channel.base_address,
                    4 => channel.block_control_register(),
                    8 => channel.control_register(),
                    _ => panic!("unsuppported DMA channel read @ {:08X}", offset)
                }
            },

            0x70 => self.control,
            0x74 => self.interrupt_register(),

            _    => { error!("unsupported DMA read"); 0 }
        }
    }

    pub fn write(&mut self, offset: u32, value: u32, ram: &mut RAM, gpu: &mut GPU)
    {
        match offset
        {
            0 ..= 0x6F =>
            {
                let port = Port::from_index((offset >> 4) & 7);
                let channel = &mut self.channels[port as usize];

                match offset & 0xF
                {
                    0 => channel.base_address = value & 0xFFFFFF,
                    4 => channel.set_block_control_register(value),
                    8 => channel.set_control_register(value),
                    _ => panic!("unsuppported DMA channel write {:08X} @ {:08X}", value, offset)
                }

                if channel.is_active()
                {
                    self.transfer(port, ram, gpu);
                }
            },

            0x70 => self.control = value,
            0x74 => self.set_interrupt_register(value),

            _    => panic!("unsupported DMA write @ {:08X}", offset)
        }
    }

    fn channel(&self, port: Port) -> &Channel
    {
        &self.channels[port as usize]
    }

    fn channel_mut(&mut self, port: Port) -> &mut Channel
    {
        &mut self.channels[port as usize]
    }

    fn interrupt_register(&self) -> u32
    {
        let master_flag = self.irq_force || (self.irq_enable && (self.irq_channel_enable & self.irq_channel_status) != 0);

        (master_flag as u32) << 31 |
        (self.irq_channel_status as u32) << 24 |
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
        self.irq_channel_status &= !reset;
    }

    fn transfer(&mut self, port: Port, ram: &mut RAM, gpu: &mut GPU)
    {
        match self.channel(port).sync_mode
        {
            SyncMode::LinkedList => self.transfer_linked_list(port, ram, gpu),
            _                    => self.transfer_block(port, ram, gpu)
        }
    }

    fn transfer_block(&mut self, port: Port, ram: &mut RAM, gpu: &mut GPU)
    {
        let channel = self.channel_mut(port);

        // For now, copy everything in one shot

        let mut blocks = match channel.sync_mode
        {
            SyncMode::Manual  => channel.transfer_size,
            SyncMode::Request => channel.transfer_size * channel.block_count,
            _ => panic!("LinkedList not supported in block transfer")
        };

        info!("DMA transfer {:?} {:?} {} {:X} {}", channel.sync_mode, channel.direction, channel.increment, channel.base_address, blocks);

        let mut address = channel.base_address;

        match port
        {
            Port::GPU =>
            {
                match channel.direction
                {
                    TransferDirection::FromRAM =>
                    {
                        while blocks > 0
                        {
                            let actual_address = address & 0x1FFFFC; // The address must stay in RAM & aligned

                            // TODO other channels than OTC
                            let value = ram.read32(actual_address);

                            info!("GPU command {:08X}", value);
                            gpu.gp0(value);

                            //address = address.wrapping_add(step);
                            address = if channel.increment { address.wrapping_add(4) } else { address.wrapping_sub(4) };
                            blocks -= 1;
                        }
                    },

                    x => panic!("unsupported DMA transfer direction {:?}", x)
                }
            },

            Port::OTC =>
            {
                match channel.direction
                {
                    TransferDirection::ToRAM =>
                    {
                        while blocks > 0
                        {
                            let actual_address = address & 0x1FFFFC; // The address must stay in RAM & aligned

                            // TODO other channels than OTC
                            let value = match blocks
                            {
                                1 => 0xFFFFFF, // Last entry: end of the table
                                _ => actual_address.wrapping_sub(4) & 0x1FFFFF // Pointer to the previous entry
                            };

                            ram.write32(actual_address, value);

                            info!("{:0X} {:0X}", actual_address, value);

                            //address = address.wrapping_add(step);
                            address = if channel.increment { address.wrapping_add(4) } else { address.wrapping_sub(4) };
                            blocks -= 1;
                        }
                    },

                    x => panic!("unsupported DMA transfer direction {:?}", x)
                }
            },

            x => panic!("unsupported port {:?}", x)
        }

        // Reset the state

        channel.enable = false;
        channel.trigger = false;
    }

    fn transfer_linked_list(&mut self, port: Port, ram: &mut RAM, gpu: &mut GPU)
    {
        let channel = self.channel_mut(port);

        // For now, copy everything in one shot

        info!("DMA transfer {:?} {:?} {:X}", channel.sync_mode, channel.direction, channel.base_address,);

        let mut address = channel.base_address & 0x1FFFFC;

        match port
        {
            Port::GPU =>
            {
                match channel.direction
                {
                    TransferDirection::FromRAM =>
                    {
                        loop
                        {
                            let header = ram.read32(address);
                            info!("header {:08X}", header);

                            // Send the commands to the GPU

                            let mut word_count = header >> 24;
                            let next_address = header & 0x1FFFFF; // TODO align?
                            info!("word count {}, next {:08X}", word_count, next_address);

                            while word_count > 0
                            {
                                address = address.wrapping_add(4) & 0x1FFFFC;
                                let value = ram.read32(address);

                                info!("GPU command {:08X}", value);
                                gpu.gp0(value);

                                word_count -= 1;
                            }

                            // Check if we hit the end of the linked list

                            if (header & 0xFFFFFF) == 0xFFFFFF // the psx guide & mednafen check only the msb though
                            {
                                info!("STOP");
                                break;
                            }

                            // Go to the next entry in the linked list

                            address = next_address;
                        }
                    },

                    x => panic!("unsupported DMA transfer direction {:?}", x)
                }
            },

            x => panic!("unsupported port {:?}", x)
        }

        // Reset the state

        channel.enable = false;
        channel.trigger = false;
    }
}
