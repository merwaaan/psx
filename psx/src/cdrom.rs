pub struct CDROM
{
    index: u8
}

impl CDROM
{
    pub fn new() -> Self
    {
        CDROM
        {
            index: 0
        }
    }

    pub fn read(&self, offset: u32) -> u32
    {
        match offset
        {
            0 =>
            {
                self.status()
            },

            1 =>
            {
                0// response FIFO
            },

            2 =>
            {
                0// data FIFO
            },

            3 =>
            {
                match self.index
                {
                    0 | 2 => 0, // Interrupt Enable Register
                    1 | 3 => 0, // Interrupt Flag Register
                    n => panic!("invalid index {}", n)
                }
            },

            n => panic!("invalid CDROM offset {}", n)
        }
    }

    pub fn write(&mut self, offset: u32, value: u32)
    {
        match offset
        {
            0 =>
            {
                self.index = (value as u8) & 3;
            },

            1 =>
            {
                match self.index
                {
                    0 => self.command(value),
                    1 => {}, // Sound Map Data Out
                    2 => {}, // Sound Map Coding Info
                    3 => {}, // Audio Volume for Right-CD-Out to Right
                    n => panic!("invalid index {}", n)
                }
            },

            2 =>
            {
                match self.index
                {
                    0 => {}, // param
                    1 => {}, // Interrupt Enable Register
                    2 => {}, // Audio Volume for Left-CD-Out to Left
                    3 => {}, // Audio Volume for Right-CD-Out to Left-
                    n => panic!("invalid index {}", n)
                }
            },

            3 =>
            {
                match self.index
                {
                    0 => {}, // request reg
                    1 | 3 => {}, //  Interrupt Flag Register
                    2 => {}, // Audio Volume for Left-CD-Out to Right-SPU-Input
                    n => panic!("invalid index {}", n)
                }
            },

            n => panic!("invalid CDROM offset {}", n)
        }
    }

    fn status(&self) -> u32
    {
        self.index as u32
    }

    fn command(&self, value: u32)
    {
        error!("CDROM command {:08X}", value);
    }
}