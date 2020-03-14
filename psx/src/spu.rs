use bitfield::bitfield;

const SPU_OFFSET: u32 = 0x1F801C00;

bitfield!
{
    struct Control(u16);

    enabled, _: 15;
    muted, _: 14;
    noise_clock_frequency, _: 13, 8;
    reverb_enabled, _: 7;
    irq_enabled, _: 6;
    dma, _: 5, 4;
    reveb_external, _: 3;
    reverb_cd, _: 2;
    external_audio, _: 1;
    cd_audio, _: 0;
}

bitfield!
{
    pub struct Status(u16);
    impl Debug;
    capture_buffer_half, _: 11;
    transfer_busy, _: 10;
    transfer_dma_r_req, _: 9;
    transfer_dma_w_req, _: 8;
    transfer_dma_rw_req, _: 7;
    irq9, _: 6;
    mode, set_mode: 5, 0; // Same as the low bits of the control register
}

pub struct SPU
{
    //data: [u8; 640], // TODO remove

    // Voice registers
    voice_data: [u16; 0xC0], // TODO structure this

    // Control registers

    volume_main_left: u16,
    volume_main_right: u16,
    volume_reverb_left: u16,
    volume_reverb_right: u16,

    voice_on: u32, // writing affects voice_status
    voice_off: u32, // writing affects voice_status
    channel_pitch: u32, // TODO use Voice struct
    channel_noise: u32,
    channel_reverb: u32,
    pub channel_status: u32, // set when writing to voice ON/OFF

    address_irq: u16,
    address_transfer: u16,

    control: Control,
    control_transfer: u16,
    pub status: Status,

    volume_cd_left: u16,
    volume_cd_right: u16,
    volume_extern_left: u16,
    volume_extern_right: u16,

    // reverb registers
    reverb_data: [u16; 0x1F] // TODO structure this
}

impl SPU
{
    pub fn new() -> SPU
    {
        SPU
        {
            //data: [0; 640],

            voice_data: [0; 0xC0],

            volume_main_left: 0,
            volume_main_right: 0,
            volume_reverb_left: 0,
            volume_reverb_right: 0,
            voice_on: 0,
            voice_off: 0,
            channel_pitch: 0,
            channel_noise: 0,
            channel_reverb: 0,
            channel_status: 0,
            address_irq: 0,
            address_transfer: 0,
            control: Control(0),
            control_transfer: 0,
            status: Status(0),
            volume_cd_left: 0,
            volume_cd_right: 0,
            volume_extern_left: 0,
            volume_extern_right: 0,

            reverb_data: [0; 0x1F]
        }
    }

    pub fn read8(&self, addr: u32) -> u8
    {
        //if addr >= 0x188 && addr <= 0x18F
        {
        //error!("SPU control registers read8 @ {:08X}", addr + SPU_OFFSET);
        }

        /*match addr
        {
            // Voice keys
            0x1F801D88 => self.voice_key,
            0x1F801D88 => self.voice_key,
            0x1F801D88 => self.voice_key,
            0x1F801D88 => self.voice_key,

            _ => self.data[addr as usize]
        }*/

        match addr
        {
            //0x1AA => self.control = Control(val),
            //0x1AE => (), // Status is read-only
            _ => panic!()
        }

        //self.data[addr as usize]
    }

    pub fn read16(&self, addr: u32) -> u16
    {
        //if addr >= 0x188 && addr <= 0x18F
        {
        //error!("SPU control registers read16 @ {:08X}", addr + SPU_OFFSET);
        }

        match addr
        {
            0 ..= 0x17F => self.voice_data[(addr >> 2) as usize],

            0x188 => self.voice_on as u16,
            0x18A => (self.voice_on >> 16) as u16,

            0x18C => self.voice_off as u16,
            0x18E => (self.voice_off >> 16) as u16,

            /*
            0x190 => self.channel_pitch = self.channel_pitch & 0x0000FFFF | ((val as u32) << 16),
            0x192 => self.channel_pitch = self.channel_pitch & 0xFFFF0000 | (val as u32),

            0x194 => self.channel_noise = self.channel_noise & 0x0000FFFF | ((val as u32) << 16),
            0x196 => self.channel_noise = self.channel_noise & 0xFFFF0000 | (val as u32),

            0x198 => self.channel_reverb = self.channel_reverb & 0x0000FFFF | ((val as u32) << 16),
            0x19A => self.channel_reverb = self.channel_reverb & 0xFFFF0000 | (val as u32),

            0x19C => self.channel_on = self.channel_on & 0x0000FFFF | ((val as u32) << 16),
            0x19E => self.channel_on = self.channel_on & 0xFFFF0000 | (val as u32),
            */
            //0x1A2 => 0, // TODO what's this?

            0x1AA => self.control.0,
            0x1AC => self.control_transfer,
            0x1AE => self.status.0,

            _ => panic!()
        }

        /*let offset = addr as usize;

        let b0 = self.data[offset] as u16;
        let b1 = self.data[offset + 1] as u16;

        (b1 << 8) | b0*/
    }

    pub fn read32(&self, addr: u32) -> u32
    {
        //if addr >= 0x188 && addr <= 0x18F
        {
       // error!("SPU control registers read32 @ {:08X}", addr + SPU_OFFSET);
        }

        match addr
        {
            //0x1AA => self.control = Control(val),
            //0x1AE => (), // Status is read-only
            _ => panic!()
        }

        /*let offset = addr as usize;

        let b0 = self.data[offset] as u32;
        let b1 = self.data[offset + 1] as u32;
        let b2 = self.data[offset + 2] as u32;
        let b3 = self.data[offset + 3] as u32;

        (b3 << 24) | (b2 << 16) | (b1 << 8) | b0*/
    }

    pub fn write8(&mut self, addr: u32, val: u8)
    {
        //if addr >= 0x188 && addr <= 0x18F
        {
        //error!("SPU control registers write8 {:02X} @ {:08X}", val, addr + SPU_OFFSET);
        }

        match addr
        {
            //0x1AA => self.control = Control(val),
            //0x1AE => (), // Status is read-only
            _ => panic!()
        }

        //self.data[addr as usize] = val
    }

    pub fn write16(&mut self, addr: u32, val: u16)
    {
        //if addr >= 0x188 && addr <= 0x18F
        {
            info!("SPU write16 {:04X} @ {:08X}", val, addr + SPU_OFFSET);
        }

        match addr
        {
            0 ..= 0x17F => self.voice_data[(addr >> 2) as usize] = val,

            0x180 => self.volume_main_left = val,
            0x182 => self.volume_main_right = val,
            0x184 => self.volume_reverb_left = val,
            0x186 => self.volume_reverb_right = val,

            0x188 =>
            {
                self.voice_on = (self.voice_on & 0xFFFF0000) | (val as u32);
                self.channel_status = (self.channel_status & 0xFFFF0000) | (val as u32);
            },
            0x18A =>
            {
                self.voice_on = (self.voice_on & 0x0000FFFF) | ((val as u32) << 16);
                self.channel_status = (self.channel_status & 0x0000FFFF) | ((val as u32) << 16);
            }

            0x18C =>
            {
                self.voice_off = (self.voice_off & 0xFFFF0000) | (val as u32);
                self.channel_status = (self.channel_status & 0xFFFF0000) & !(val as u32);
            }
            0x18E =>
            {
                self.voice_off = (self.voice_off & 0x0000FFFF) | ((val as u32) << 16);
                self.channel_status = (self.channel_status & 0x0000FFFF) & !((val as u32) << 16);
            }

            // TODO fix endianness
            0x190 => self.channel_pitch = self.channel_pitch & 0x0000FFFF | ((val as u32) << 16),
            0x192 => self.channel_pitch = self.channel_pitch & 0xFFFF0000 | (val as u32),

            0x194 => self.channel_noise = self.channel_noise & 0x0000FFFF | ((val as u32) << 16),
            0x196 => self.channel_noise = self.channel_noise & 0xFFFF0000 | (val as u32),

            0x198 => self.channel_reverb = self.channel_reverb & 0x0000FFFF | ((val as u32) << 16),
            0x19A => self.channel_reverb = self.channel_reverb & 0xFFFF0000 | (val as u32),

            0x1A2 => {},//error!("unimplemented SPU register"),
            0x1A4 => self.address_irq = val,
            0x1A6 => self.address_transfer = val,
            0x1A8 => {},//error!("unimplemented SPU transfer"),

            0x1AA =>
            {
                self.control = Control(val);
                self.status.set_mode(val & 0x3F); // Update status
            },
            0x1AC => self.control_transfer = val,
            0x1AE => (), // Status is read-only

            0x1B0 => self.volume_cd_left = val,
            0x1B2 => self.volume_cd_right = val,

            0x1B4 => self.volume_extern_left = val,
            0x1B6 => self.volume_extern_right = val,

            0x1C0 ..= 0x1FF => self.reverb_data[((addr - 0x1C0) >> 2) as usize] = val,

            _ => panic!()
        }

        /*let offset = addr as usize;
        self.data[offset] = val as u8;
        self.data[offset + 1] = (val >> 8) as u8;*/
    }

    pub fn write32(&mut self, addr: u32, val: u32)
    {
        //if addr >= 0x188 && addr <= 0x18F
        {
        //error!("SPU control registers write32 {:08X} @ {:08X}", val, addr + SPU_OFFSET);
        }

        match addr
        {
            //0x1AA => self.control = Control(val),
            //0x1AE => (), // Status is read-only
            _ => panic!()
        }

        /*
        let offset = addr as usize;
        self.data[offset] = val as u8;
        self.data[offset + 1] = ((val & 0xFF00) >> 8) as u8;
        self.data[offset + 2] = ((val & 0xFF0000) >> 16) as u8;
        self.data[offset + 3] = ((val & 0xFF000000) >> 24) as u8;
        */
    }
}
