use crate::interrupt_controller::{InterruptController, InterruptRequest};

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

enum Interrupt
{
    _Int1 = 1,
    _Int2 = 2,
    Int3 = 3,
    _Int4 = 4,
    _Int5 = 5
}

pub struct CDROM
{
    index: u8,

    interrupt_enable: u8,
    interrupt_flag: u8,

    //current_command: Option<u8>,
    parameter_fifo: VecDeque<u8>,
    response_fifo: VecDeque<u8>,

    interrupt_controller: Rc<RefCell<InterruptController>>
}

impl CDROM
{
    pub fn new(interrupt_controller: &Rc<RefCell<InterruptController>>) -> Self
    {
        CDROM
        {
            index: 0,

            interrupt_enable: 0,
            interrupt_flag: 0,

            //current_command: None,
            parameter_fifo: VecDeque::with_capacity(16),
            response_fifo: VecDeque::new(),

            interrupt_controller: interrupt_controller.clone()
        }
    }

    // TODO read is mut self which is weird, use some form of interior mutability?
    pub fn read8(&mut self, offset: u32) -> u8
    {
        error!("CDROM read8 @ {} (index {})", offset, self.index);

        match offset
        {
            0 =>
            {
                self.status()
            },

            // Response FIFO
            1 =>
            {
                match self.response_fifo.pop_front()
                {
                    Some(value) =>
                    {
                        error!("CDROM pop response FIFO: {:02X}", value);
                        value
                    },
                    None =>
                    {
                        error!("CDROM response FIFO empty");
                        0 // TODO correct behavior?
                    }
                }
            },

            2 =>
            {
                0// data FIFO
            },

            3 =>
            {
                match self.index
                {
                    0 | 2 => self.interrupt_enable,
                    1 | 3 => self.interrupt_flag,
                    n => panic!("invalid index {}", n)
                }
            },

            n => panic!("invalid CDROM offset {}", n)
        }
    }

    pub fn write8(&mut self, offset: u32, value: u8)
    {
        error!("CDROM write8 {:08X} @ {} (index {})", value, offset, self.index);

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
                    // Parameter FIFO
                    0 =>
                    {
                        if self.parameter_fifo.len() == 16
                        {
                            self.parameter_fifo.pop_front();
                            error!("CDROM parameter FIFO full");
                        }

                        error!("CDROM push to FIFO {:02X}", value);
                        self.parameter_fifo.push_back(value);
                    },

                    // Interrupt Enable Register
                    1 =>
                    {
                        self.interrupt_enable = value & 0x1F;
                        // TODO can generate interrupt if flag set?
                    },

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

                    //  Interrupt Flag Register
                    1 =>
                    {
                        self.interrupt_flag &= !(value & 0x1F);

                        // TODO clear FIFO?
                    },

                    2 => {}, // Audio Volume for Left-CD-Out to Right-SPU-Input
                    3 => {}, // Audio Volume Apply Changes
                    n => panic!("invalid index {}", n)
                }
            },

            n => panic!("invalid CDROM offset {}", n)
        }
    }

    fn status(&self) -> u8
    {
        (0 << 7) | // Command/Parameter transmission busy
        (0 << 6) | // Data FIFO empty
        (0 << 5) | // Response FIFO empty
        (((self.parameter_fifo.len() != 16) as u8) << 4) | // Parameter FIFO full (0 = full)
        (((self.parameter_fifo.len() == 0) as u8) << 3) | // Parameter FIFO empty (1 = empty)
        (0 << 2) | // ??
        (self.index & 3)
    }

    fn interrupt(&mut self, int: Interrupt)
    {
        // Mark the interrupt a requested
        self.interrupt_flag = int as u8;

        // Request the interrupt if enabled
        if (self.interrupt_flag & self.interrupt_enable) != 0 // TODO only this specific interrupt?
        {
            self.interrupt_controller.borrow_mut().request(InterruptRequest::CDROM);
        }
    }

    fn command(&mut self, value: u8)
    {
        error!("CDROM command {:08X}", value);

        match value
        {
            // Test
            0x19 =>
            {
                let subcommand = self.parameter_fifo.pop_front().unwrap();
                error!("CDROM test subcommand {:08X}", subcommand);

                match subcommand
                {
                    // BIOS version
                    0x20 =>
                    {
                        // Nocash lists a few real-world values.
                        // Here we return "Version vC0 (a), 19 Sep 1994".
                        self.response_fifo.push_back(0x94);
                        self.response_fifo.push_back(0x09);
                        self.response_fifo.push_back(0x19);
                        self.response_fifo.push_back(0xC0);
                    }

                    x => panic!("unsupported subcommand {:02X}", x)
                }
            },

            x => panic!("unsupported command {:02X}", x)
        }

        self.interrupt_flag = 0x3; // TODO |?

        // TODO sure about this?
        if (self.interrupt_flag & self.interrupt_enable) != 0
        {
            self.interrupt(Interrupt::Int3);
        }
    }
}