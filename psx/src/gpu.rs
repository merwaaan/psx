use crate::renderer::{ Color, Position, Renderer };
use std::collections::VecDeque;

#[derive(Debug, Copy, Clone)]
enum DMADirection
{
    Off = 0,
    FIFO = 1,
    CPUToGP0 = 2,
    GPUToCPU = 3
}

#[derive(Debug, Copy, Clone)]
enum TextureDepth
{
    Bits4 = 0,
    Bits8 = 1,
    Bits15 = 2
}

#[derive(Debug, Copy, Clone)]
struct HorizontalResolution(u8);

impl HorizontalResolution
{
    fn from_bytes(b18_17: u8, b16: u8) -> HorizontalResolution
    {
        let byte = ((b18_17 & 3) << 1) | (b16 & 1);
        HorizontalResolution(byte)
    }

    fn into_status(&self) -> u32
    {
        let HorizontalResolution(byte) = *self;
        (byte as u32) << 16
    }
}

#[derive(Debug, Copy, Clone)]
enum VerticalResolution
{
    V240 = 0,
    V480 = 1
}

#[derive(Debug, Copy, Clone)]
enum VideoMode
{
    NTSC = 0,
    PAL = 1
}

#[derive(Debug, Copy, Clone)]
enum DisplayDepth
{
    Bits15 = 0,
    Bits24 = 1
}

#[derive(Debug, Copy, Clone)]
enum Field
{
    Bottom = 0,
    Top = 1
}

#[derive(Debug, Copy, Clone)]
pub enum Port
{
    GP0 = 0,
    GP1 = 1
}

#[derive(Debug)]
pub struct Command(pub Port, pub u32);

struct CommandBuffer
{
    data: [u32; 12], // 12 is the longest command size
    current_length: usize
}

impl CommandBuffer
{
    fn new() -> CommandBuffer
    {
        CommandBuffer
        {
            data: [0; 12],
            current_length: 0
        }
    }

    fn clear(&mut self)
    {
        self.current_length = 0;
    }

    fn push(&mut self, word: u32)
    {
        self.data[self.current_length] = word;
        self.current_length += 1;
    }
}

impl ::std::ops::Index<usize> for CommandBuffer
{
    type Output = u32;

    fn index<'a>(&'a self, i: usize) -> &'a u32
    {
        if i > self.current_length
        {
            panic!("Command buffer index out of range: {} ({})", i, self.current_length);
        }

        &self.data[i]
    }
}

enum GP0Mode
{
    Command,
    LoadImage
}

pub struct GPU
{
    // Status

    dma_direction: DMADirection,
    irq: bool,
    display_disable: bool, // TODO unclear
    interlace: bool,
    display_depth: DisplayDepth,
    video_mode: VideoMode,
    resolution_vertical: VerticalResolution,
    resolution_horizontal: HorizontalResolution,
    texture_disable: bool,
    field: Field,
    ignore_masked_pixels: bool,
    force_mask_bit: bool,
    draw_to_display: bool,
    dither: bool,
    texture_depth: TextureDepth,
    semitransparency: u8,
    texture_page_base_y: u8,
    texture_page_base_x: u8,

    // Internal state

    texture_flip_x: bool,
    texture_flip_y: bool,
    texture_window_offset_y: u8,
    texture_window_offset_x: u8,
    texture_window_mask_y: u8,
    texture_window_mask_x: u8,

    drawing_area_top: u16,
    drawing_area_left: u16,
    drawing_area_right: u16,
    drawing_area_bottom: u16,
    drawing_offset_x: i16,
    drawing_offset_y: i16,
    display_vram_start_x: u16,
    display_vram_start_y: u16,
    display_horizontal_end: u16,
    display_horizontal_start: u16,
    display_vertical_end: u16,
    display_vertical_start: u16,

    // Command buffering

    gp0_command_buffer: CommandBuffer,
    gp0_command_method: fn(&mut GPU),
    gp0_words_remaining: u32,
    gp0_mode: GP0Mode,

    // Debugging

    pub previous_commands: VecDeque<Command>,

    renderer: Renderer
}

impl GPU
{
    pub fn new(display: &glium::Display) -> GPU
    {
        GPU
        {
            dma_direction: DMADirection::Off,
            irq: false,
            display_disable: false,
            interlace: false,
            display_depth: DisplayDepth::Bits15,
            video_mode: VideoMode::NTSC,
            resolution_vertical: VerticalResolution::V240,
            resolution_horizontal: HorizontalResolution::from_bytes(0, 0),
            texture_disable: false,
            field: Field::Top,
            ignore_masked_pixels: false,
            force_mask_bit: false,
            draw_to_display: false,
            dither: false,
            texture_depth: TextureDepth::Bits4,
            semitransparency: 0,
            texture_page_base_y: 0,
            texture_page_base_x: 0,

            texture_flip_x: false,
            texture_flip_y: false,
            texture_window_offset_y: 0,
            texture_window_offset_x: 0,
            texture_window_mask_y: 0,
            texture_window_mask_x: 0,

            drawing_area_top: 0,
            drawing_area_left: 0,
            drawing_area_right: 0,
            drawing_area_bottom: 0,
            drawing_offset_x: 0,
            drawing_offset_y: 0,
            display_vram_start_x: 0,
            display_vram_start_y: 0,
            display_horizontal_end: 0,
            display_horizontal_start: 0,
            display_vertical_end: 0,
            display_vertical_start: 0,

            previous_commands: VecDeque::with_capacity(100),

            gp0_command_buffer: CommandBuffer::new(),
            gp0_command_method: GPU::gp0_nop,
            gp0_words_remaining: 0,
            gp0_mode: GP0Mode::Command,

            renderer: Renderer::new(display)
        }
    }

    pub fn render(&mut self, target: &mut glium::Frame)
    {
        self.renderer.render(target);
    }

    fn enqueue_command(&mut self, port: Port, command: u32)
    {
        if self.previous_commands.len() == 100
        {
            self.previous_commands.pop_front();
        }

        self.previous_commands.push_back(Command(port, command))
    }

    pub fn disassemble(&self, command: &Command) -> String
    {
        let opcode = command.1 >> 24;

        let description = match command.0
        {
            Port::GP0 =>
            {
                match opcode
                {
                    0x00 => "NOP",
                    0x01 => "Clear cache",
                    0x28 => "Draw quad monochrome opaque",
                    0x2C => "Draw quad textured opaque",
                    0x30 => "Draw triangle shaded opaque",
                    0x38 => "Draw quad shaded opaque",
                    0xA0 => "Load image",
                    0xC0 => "Store image",
                    0xE1 => "Draw mode",
                    0xE2 => "Texture window",
                    0xE3 => "Drawing area top left",
                    0xE4 => "Drawing area bottom right",
                    0xE5 => "Drawing offset",
                    0xE6 => "Mask bit setting",
                    _ => "[UNSUPPORTED COMMAND]"
                }
            },
            Port::GP1 =>
            {
                match opcode
                {
                    0x00 => "Reset",
                    0x01 => "Reset command buffer",
                    0x02 => "Acknowledge IRQ",
                    0x03 => "Enable display",
                    0x04 => "Setup DMA",
                    0x05 => "Start of display area",
                    0x06 => "Horizontal display range",
                    0x07 => "Vertical display range",
                    0x08 => "Display mode",
                    _ => "[UNSUPPORTED COMMAND]"
                }
            },
        };

        String::from(description)
    }

    pub fn status(&self) -> u32
    {
        let dma_request = match self.dma_direction
        {
            DMADirection::Off      => 0,
            DMADirection::FIFO     => 1,
            DMADirection::CPUToGP0 => 1, // same as bit 28
            DMADirection::GPUToCPU => 1  // same as bit 27
        };

        1 << 31 | // TODO
        (self.dma_direction as u32) << 29 |
        (1 << 28) |
        (1 << 27) |
        (1 << 26) |
        dma_request << 25 |
        (self.irq as u32) << 24 |
        (self.display_disable as u32) << 23 |
        (self.interlace as u32) << 22 |
        (self.display_depth as u32) << 21 |
        (self.video_mode as u32) << 20 |
        // DISABLED FOR NOW SO WORK AROUND THE BIT31 ENDLESS LOOP (self.resolution_vertical as u32) << 19 |
        self.resolution_horizontal.into_status() |
        (self.texture_disable as u32) << 15 |
        (self.field as u32) << 13 |
        (self.ignore_masked_pixels as u32) << 12 |
        (self.force_mask_bit as u32) << 11 |

        (self.draw_to_display as u32) << 10 |
        (self.dither as u32) << 9 |
        (self.texture_depth as u32) << 7 |
        (self.semitransparency as u32) << 5 |
        (self.texture_page_base_y as u32) << 4 |
        self.texture_page_base_x as u32
    }

    pub fn read(&self) -> u32
    {
        0
    }

    pub fn gp0(&mut self, command: u32)
    {
        // No command being buffered, we start a new one

        if self.gp0_words_remaining == 0
        {
            let opcode = command >> 24;

            let (method, word_count) = match opcode
            {
                0x00 => (GPU::gp0_nop as fn(&mut GPU), 1),
                0x01 => (GPU::gp0_clear_cache as fn(&mut GPU), 1),
                0x28 => (GPU::gp0_draw_quad_mono_opaque as fn(&mut GPU), 5),
                0x2C => (GPU::gp0_draw_quad_textured_opaque as fn(&mut GPU), 9),
                0x30 => (GPU::gp0_draw_triangle_shaded_opaque as fn(&mut GPU), 6),
                0x38 => (GPU::gp0_draw_quad_shaded_opaque as fn(&mut GPU), 8),
                0xA0 => (GPU::gp0_load_image as fn(&mut GPU), 3),
                0xC0 => (GPU::gp0_store_image as fn(&mut GPU), 3),
                0xE1 => (GPU::gp0_draw_mode as fn(&mut GPU), 1),
                0xE2 => (GPU::gp0_texture_window as fn(&mut GPU), 1),
                0xE3 => (GPU::gp0_drawing_area_top_left as fn(&mut GPU), 1),
                0xE4 => (GPU::gp0_drawing_area_bottom_right as fn(&mut GPU), 1),
                0xE5 => (GPU::gp0_drawing_offset as fn(&mut GPU), 1),
                0xE6 => (GPU::gp0_mask_bit_setting as fn(&mut GPU), 1),

                _ => panic!("unsupported GP0 opcode {:0X}", opcode)
            };

            self.gp0_command_method = method;
            self.gp0_command_buffer.clear();
            self.gp0_words_remaining = word_count;

            self.enqueue_command(Port::GP0, command);
        }

        self.gp0_words_remaining -= 1;

        match self.gp0_mode
        {
            GP0Mode::Command =>
            {
                // Push the current word to the command buffer
                self.gp0_command_buffer.push(command);

                // Execute the command if we accumulated all the parameters
                if self.gp0_words_remaining == 0
                {
                    (self.gp0_command_method)(self);
                }
            },

            GP0Mode::LoadImage =>
            {
                // TODO LOAD

                if self.gp0_words_remaining == 0
                {
                    self.gp0_mode = GP0Mode::Command;
                }
            }
        }
    }

    // GP0

    fn gp0_nop(&mut self)
    {
    }

    fn gp0_clear_cache(&mut self)
    {
    }

    fn gp0_draw_quad_mono_opaque(&mut self)
    {
        let positions =
        [
            Position::from_command(self.gp0_command_buffer[1]),
            Position::from_command(self.gp0_command_buffer[2]),
            Position::from_command(self.gp0_command_buffer[3]),
            Position::from_command(self.gp0_command_buffer[4])
        ];

        let colors = [Color::from_command(self.gp0_command_buffer[0]); 4];

        self.renderer.push_quad(positions, colors);
    }

    fn gp0_draw_quad_textured_opaque(&mut self)
    {
        let positions =
        [
            Position::from_command(self.gp0_command_buffer[1]),
            Position::from_command(self.gp0_command_buffer[2]),
            Position::from_command(self.gp0_command_buffer[3]),
            Position::from_command(self.gp0_command_buffer[4])
        ];

        let colors = [Color(255, 20, 147); 4]; // TEMP FAKE COLOR

        self.renderer.push_quad(positions, colors);
    }

    fn gp0_draw_triangle_shaded_opaque(&mut self)
    {
        let positions =
        [
            Position::from_command(self.gp0_command_buffer[1]),
            Position::from_command(self.gp0_command_buffer[3]),
            Position::from_command(self.gp0_command_buffer[5])
        ];

        let colors =
        [
            Color::from_command(self.gp0_command_buffer[0]),
            Color::from_command(self.gp0_command_buffer[2]),
            Color::from_command(self.gp0_command_buffer[4])
        ];

        self.renderer.push_triangle(positions, colors);
    }

    fn gp0_draw_quad_shaded_opaque(&mut self)
    {
        let positions =
        [
            Position::from_command(self.gp0_command_buffer[1]),
            Position::from_command(self.gp0_command_buffer[3]),
            Position::from_command(self.gp0_command_buffer[5]),
            Position::from_command(self.gp0_command_buffer[7])
        ];

        let colors =
        [
            Color::from_command(self.gp0_command_buffer[0]),
            Color::from_command(self.gp0_command_buffer[2]),
            Color::from_command(self.gp0_command_buffer[4]),
            Color::from_command(self.gp0_command_buffer[6])
        ];

        self.renderer.push_quad(positions, colors);
    }

    fn gp0_load_image(&mut self)
    {
        let resolution = self.gp0_command_buffer[2];
        let width = resolution & 0xFFFF;
        let height = resolution >> 16;

        let image_size = width * height;

        self.gp0_words_remaining = image_size / 2;
        self.gp0_mode = GP0Mode::LoadImage;

        // Handle even pixel counts.
        // Since we read two pixels for each byte, we might need another word for the last pixel.
        if image_size % 2 == 1
        {
            self.gp0_words_remaining += 1;
        }
    }

    fn gp0_store_image(&mut self)
    {
        error!("unsupported GP0 store image");
    }

    fn gp0_draw_mode(&mut self)
    {
        let value = self.gp0_command_buffer[0];

        self.texture_flip_y = ((value >> 13) & 1) != 0;
        self.texture_flip_x = ((value >> 12) & 1) != 0;
        self.texture_disable = ((value >> 11) & 1) != 0;
        self.draw_to_display = ((value >> 10) & 1) != 0;
        self.dither = ((value >> 9) & 1) != 0;

        self.texture_depth = match (value >> 7) & 3
        {
            0 => TextureDepth::Bits4,
            1 => TextureDepth::Bits8,
            2 => TextureDepth::Bits15,
            x => panic!("unsupported texture depth {}", x)
        };

        self.semitransparency = ((value >> 5) & 3) as u8;
        self.texture_page_base_y = ((value >> 4) & 1) as u8;
        self.texture_page_base_x = (value & 0xF) as u8;
    }

    fn gp0_texture_window(&mut self)
    {
        let value = self.gp0_command_buffer[0];

        self.texture_window_offset_y = ((value >> 15) & 0x1F) as u8;
        self.texture_window_offset_x = ((value >> 10) & 0x1F) as u8;
        self.texture_window_mask_y = ((value >> 5) & 0x1F) as u8;
        self.texture_window_mask_x = (value & 0x1F) as u8;
    }

    fn gp0_drawing_area_top_left(&mut self)
    {
        let value = self.gp0_command_buffer[0];

        // Y: 19-10, X: 9-0
        self.drawing_area_top = ((value >> 10) & 0x3FF) as u16;
        self.drawing_area_left = (value & 0x3FF) as u16;
    }

    fn gp0_drawing_area_bottom_right(&mut self)
    {
        let value = self.gp0_command_buffer[0];

        // Y: 19-10, X: 9-0
        self.drawing_area_bottom = ((value >> 10) & 0x3FF) as u16;
        self.drawing_area_right = (value & 0x3FF) as u16;
    }

    fn gp0_drawing_offset(&mut self)
    {
        let value = self.gp0_command_buffer[0];

        // Y: 21-11, X: 10-0
        let x = (value & 0x7FF) as u16;
        let y = ((value >> 11) & 0x7FF) as u16;

        // The offset values are 11 bit two-complement signed values
        // so we shift the values  to the far left to force sign extension
        self.drawing_offset_x = ((x << 5) as i16) >> 5;
        self.drawing_offset_y = ((y << 5) as i16) >> 5;
    }

    fn gp0_mask_bit_setting(&mut self)
    {
        let value = self.gp0_command_buffer[0];

        self.ignore_masked_pixels = (value & 2) != 0; // Bit 2
        self.force_mask_bit = (value & 1) != 0; // Bit 1
    }

    // GP1

    pub fn gp1(&mut self, command: u32)
    {
        let opcode = command >> 24;

        match opcode
        {
            0x00 => self.gp1_reset(command),
            0x01 => self.gp1_reset_command_buffer(command),
            0x02 => self.gp1_acknowledge_irq(command),
            0x03 => self.gp1_enable_display(command),
            0x04 => self.gp1_dma_setup(command),
            0x05 => self.gp1_display_vram_start(command),
            0x06 => self.gp1_display_horizontal_range(command),
            0x07 => self.gp1_display_vertical_range(command),
            0x08 => self.gp1_display_mode(command),

            _ => panic!("unsupported GP1 opcode {:0X}", opcode)
        }

        self.enqueue_command(Port::GP1, command);
    }

    fn gp1_reset(&mut self, value: u32)
    {
        self.dma_direction = DMADirection::Off;
        self.irq = false;
        self.display_disable = false;
        self.interlace = false;
        self.display_depth = DisplayDepth::Bits15;
        self.video_mode = VideoMode::NTSC;
        self.resolution_vertical = VerticalResolution::V240;
        self.resolution_horizontal = HorizontalResolution::from_bytes(0, 0);
        self.texture_disable = false;
        self.field = Field::Top;
        self.ignore_masked_pixels = false;
        self.force_mask_bit = false;
        self.draw_to_display = false;
        self.dither = false;
        self.texture_depth = TextureDepth::Bits4;
        self.semitransparency = 0;
        self.texture_page_base_y = 0;
        self.texture_page_base_x = 0;

        self.texture_flip_x = false;
        self.texture_flip_y = false;
        self.texture_window_offset_y = 0;
        self.texture_window_offset_x = 0;
        self.texture_window_mask_y = 0;
        self.texture_window_mask_x = 0;

        self.drawing_area_top = 0;
        self.drawing_area_left = 0;
        self.drawing_area_right = 0;
        self.drawing_area_bottom = 0;
        self.drawing_offset_x = 0;
        self.drawing_offset_y = 0;
        self.display_vram_start_x = 0;
        self.display_vram_start_y = 0;
        self.display_horizontal_end = 0;
        self.display_horizontal_start = 0;
        self.display_vertical_end = 0;
        self.display_vertical_start = 0;

        self.gp1_reset_command_buffer(value);
    }

    fn gp1_reset_command_buffer(&mut self, _value: u32)
    {
        self.gp0_command_buffer.clear();
        self.gp0_words_remaining = 0;
        self.gp0_mode = GP0Mode::Command;
    }

    fn gp1_acknowledge_irq(&mut self, _value: u32)
    {
        // TODO
    }

    fn gp1_enable_display(&mut self, value: u32)
    {
        self.display_disable = (value & 1) != 0;
    }

    fn gp1_dma_setup(&mut self, value: u32)
    {
        self.dma_direction = match value & 3
        {
            0 => DMADirection::Off,
            1 => DMADirection::FIFO,
            2 => DMADirection::CPUToGP0,
            3 => DMADirection::GPUToCPU,
            _ => unreachable!()
        }
    }

    fn gp1_display_vram_start(&mut self, value: u32)
    {
        self.display_vram_start_y = ((value >> 10) & 0x1FF) as u16;
        self.display_vram_start_x = (value & 0x3FE) as u16; // LSB ignored, always aligned
    }

    fn gp1_display_horizontal_range(&mut self, value: u32)
    {
        self.display_horizontal_end = ((value >> 12) & 0xFFF) as u16;
        self.display_horizontal_start = (value & 0xFFF) as u16;
    }

    fn gp1_display_vertical_range(&mut self, value: u32)
    {
        self.display_vertical_end = ((value >> 10) & 0x3FF) as u16;
        self.display_vertical_start = (value & 0x3FF) as u16;
    }

    fn gp1_display_mode(&mut self, value: u32)
    {
        self.interlace = (value & 0x20) != 0;

        self.display_depth = match (value & 0x10) != 0
        {
            true => DisplayDepth::Bits15,
            false => DisplayDepth::Bits24
        };

        self.video_mode = match (value & 8) != 0
        {
            true => VideoMode::PAL,
            false => VideoMode::NTSC
        };

        self.resolution_vertical = match (value & 4) != 0
        {
            true => VerticalResolution::V480,
            false => VerticalResolution::V240
        };

        self.resolution_horizontal = HorizontalResolution::from_bytes(value as u8 & 3, (value >> 6) as u8 & 1);

        // Don't know how to handle this flag
        if (value & 0x80) != 0
        {
            panic!("weird reverse flag!");
        }
    }
}
