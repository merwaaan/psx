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
    _Bottom = 0,
    Top = 1
}

#[derive(Debug, Copy, Clone)]
pub enum Port
{
    GP0 = 0,
    GP1 = 1
}

#[derive(Debug, Clone)]
pub struct CommandBuffer
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

#[derive(Debug)]
pub struct CommandRecord(pub Port, pub CommandBuffer);

enum GP0Mode
{
    Command,
    LoadImage
}

const VRAM_WIDTH: usize = 1024;
const VRAM_HEIGHT: usize = 512;

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

    // TODO put into struct
    load_image_start_x: u16,
    load_image_start_y: u16,
    load_image_end_x: u16,
    load_image_end_y: u16,
    load_image_current_x: u16,
    load_image_current_y: u16,

    reading_vram: bool,

    vram: Vec<u16>,

    // GPUREAD value
    read_response: u32,

    // Debugging

    pub previous_commands: VecDeque<CommandRecord>,

    renderer: Renderer
}

const MAX_COMMAND_RECORD_SIZE: usize = 1000;

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

            previous_commands: VecDeque::with_capacity(MAX_COMMAND_RECORD_SIZE),

            gp0_command_buffer: CommandBuffer::new(),
            gp0_command_method: GPU::gp0_nop,
            gp0_words_remaining: 0,
            gp0_mode: GP0Mode::Command,

            load_image_start_x: 0,
            load_image_start_y: 0,
            load_image_end_x: 0,
            load_image_end_y: 0,
            load_image_current_x: 0,
            load_image_current_y: 0,

            vram: vec![0; VRAM_WIDTH * VRAM_HEIGHT],
            reading_vram: false,

            read_response: 0,

            renderer: Renderer::new(display)
        }
    }

    pub fn render(&mut self, target: &mut glium::Frame)
    {
        self.renderer.render(target);
    }

    fn save_command(&mut self, port: Port, command: CommandBuffer)
    {
        if self.previous_commands.len() == MAX_COMMAND_RECORD_SIZE
        {
            self.previous_commands.pop_front();
        }

        // TODO only when debugging
        self.previous_commands.push_back(CommandRecord(port, command))
    }

    pub fn disassemble(&self, command: &CommandRecord) -> String
    {
        let opcode = command.1[0] >> 24;

        let description = match command.0
        {
            Port::GP0 =>
            {
                match opcode
                {
                    0x00 => "NOP".to_string(),
                    0x01 => "Clear cache".to_string(),
                    0x28 => "Draw quad monochrome opaque".to_string(),
                    0x2C => "Draw quad textured opaque".to_string(),
                    0x30 => "Draw triangle shaded opaque".to_string(),
                    0x38 => "Draw quad shaded opaque".to_string(),
                    0x68 =>
                    {
                        let pos = Position::from_command(command.1[1]);
                        let color = Color::from_command(command.1[0]);
                        format!("Draw dot monochrome opaque [pos={:?}, col={:?}]", pos, color)
                    },
                    0xA0 => "Load image".to_string(),
                    0xC0 => "Store image".to_string(),
                    0xE1 => "Draw mode".to_string(),
                    0xE2 => "Texture window".to_string(),
                    0xE3 => "Drawing area top left".to_string(),
                    0xE4 => "Drawing area bottom right".to_string(),
                    0xE5 => "Drawing offset".to_string(),
                    0xE6 => "Mask bit setting".to_string(),
                    _ => "[UNSUPPORTED COMMAND]".to_string()
                }
            },
            Port::GP1 =>
            {
                match opcode
                {
                    0x00 => "Reset".to_string(),
                    0x01 => "Reset command buffer".to_string(),
                    0x02 => "Acknowledge IRQ".to_string(),
                    0x03 => "Enable display".to_string(),
                    0x04 => "Setup DMA".to_string(),
                    0x05 => "Start of display area".to_string(),
                    0x06 => "Horizontal display range".to_string(),
                    0x07 => "Vertical display range".to_string(),
                    0x08 => "Display mode".to_string(),
                    0x10 => "Get GPU info".to_string(),
                    _ => "[UNSUPPORTED COMMAND]".to_string()
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

    pub fn read(&mut self) -> u32 // TODO how to avoid mut? interior mutab?
    {
        if self.reading_vram
        {
            let a = self.read_vram_pixel();
            let b = self.read_vram_pixel();

            self.read_response =
                (b as u32) << 16 |
                a as u32;

            if self.load_image_current_y == self.load_image_end_y
            {
                self.reading_vram = false;
            }
        }

        self.read_response
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
                0x68 => (GPU::gp0_draw_dot_mono_opaque as fn(&mut GPU), 2),
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

                    self.save_command(Port::GP0, self.gp0_command_buffer.clone());
                }
            },

            GP0Mode::LoadImage =>
            {
                error!("load image {}", self.gp0_words_remaining);

                self.write_vram_pixel((command & 0xFFFF) as u16);
                self.write_vram_pixel((command >> 16) as u16);
                // TODO take masking into account

                if self.gp0_words_remaining == 0
                {
                    self.gp0_mode = GP0Mode::Command;
                }
            }
        }
    }

    fn write_vram_pixel(&mut self, value: u16)
    {
        error!("write to VRAM {:08X} @ {:08X} {:08X}", value, self.load_image_current_x, self.load_image_current_y);

        self.vram[self.load_image_current_y as usize * VRAM_WIDTH + self.load_image_current_x as usize] = value;

        // TODO wrap?
        self.load_image_current_x += 1;
        if self.load_image_current_x == self.load_image_end_x
        {
            self.load_image_current_x = self.load_image_start_x;
            self.load_image_current_y += 1;
        }
    }

    fn read_vram_pixel(&mut self) -> u16
    {
        let value = self.vram[self.load_image_current_y as usize * VRAM_WIDTH + self.load_image_current_x as usize];

        error!("read from VRAM {:08X} @ {:08X} {:08X}", value, self.load_image_current_x, self.load_image_current_y);

        // TODO wrap?
        self.load_image_current_x += 1;
        if self.load_image_current_x == self.load_image_end_x
        {
            self.load_image_current_x = self.load_image_start_x;
            self.load_image_current_y += 1;
        }

        value
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
            Position::from_command(self.gp0_command_buffer[3]),
            Position::from_command(self.gp0_command_buffer[5]),
            Position::from_command(self.gp0_command_buffer[7])
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

    fn gp0_draw_dot_mono_opaque(&mut self)
    {
        let pos = Position::from_command(self.gp0_command_buffer[1]);

        let positions =
        [
            pos,
            Position(pos.0 + 1, pos.1),
            Position(pos.0,     pos.1 + 1),
            Position(pos.0 + 1, pos.1 + 1)
        ];

        let color = Color::from_command(self.gp0_command_buffer[0]);

        let colors =
        [
            color,
            color,
            color,
            color
        ];

        self.renderer.push_quad(positions, colors);
    }

    fn gp0_load_image(&mut self)
    {
        let resolution = self.gp0_command_buffer[2];
        let width = (resolution & 0xFFFF) as u16;
        let height = (resolution >> 16) as u16;

        let image_size = width as u32 * height as u32;
        let image_size = (image_size + 1) & !1; // Handle odd pixel counts

        self.gp0_words_remaining = image_size / 2;

        self.load_image_start_x = (self.gp0_command_buffer[1] & 0xFFFF) as u16;
        self.load_image_start_y = (self.gp0_command_buffer[1] >> 16) as u16;
        self.load_image_end_x = self.load_image_start_x + width; // TODO wrap/cap?
        self.load_image_end_y = self.load_image_start_y + height;

        self.load_image_current_x = self.load_image_start_x;
        self.load_image_current_y = self.load_image_start_y;

        error!("load image command {} {} {} {:08X} {:08X}", width, height, self.gp0_words_remaining, self.load_image_start_x, self.load_image_start_y);
        self.gp0_mode = GP0Mode::LoadImage;
    }

    fn gp0_store_image(&mut self)
    {
        let resolution = self.gp0_command_buffer[2];
        let width = (resolution & 0xFFFF) as u16;
        let height = (resolution >> 16) as u16;

        self.load_image_start_x = (self.gp0_command_buffer[1] & 0xFFFF) as u16;
        self.load_image_start_y = (self.gp0_command_buffer[1] >> 16) as u16;
        self.load_image_end_x = self.load_image_start_x + width; // TODO wrap/cap?
        self.load_image_end_y = self.load_image_start_y + height;

        self.load_image_current_x = self.load_image_start_x;
        self.load_image_current_y = self.load_image_start_y;

        error!("store image command {} {} {:08X} {:08X}", width, height, self.load_image_start_x, self.load_image_start_y);
        self.reading_vram = true;
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
            0x10 ..= 0x1F => self.gp1_get_gpu_info(command),

            _ => panic!("unsupported GP1 opcode {:0X}", opcode)
        }

        self.save_command(Port::GP1, CommandBuffer { data: [command; 12], current_length: 1});
    }

    fn gp1_reset(&mut self, value: u32)
    {
        self.dma_direction = DMADirection::Off;
        self.irq = false;
        self.display_disable = true;
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

        self.read_response = 0;

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
            false => DisplayDepth::Bits15,
            true => DisplayDepth::Bits24
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

    fn gp1_get_gpu_info(&mut self, value: u32)
    {
        error!("GetGPUInfo {:08X}", value);

        // Only the first 3 bits are meaningful
        self.read_response = match value & 7
        {
            // Texture window
            2 =>
            {
                (self.texture_window_mask_x as u32) |
                (self.texture_window_mask_y as u32) << 5 |
                (self.texture_window_offset_x as u32) << 10 |
                (self.texture_window_offset_y as u32) << 15
            }

            // Draw area top left
            3 =>
            {
                (self.drawing_area_left as u32) |
                (self.drawing_area_top as u32) << 10
            }

            // Draw area bottom right
            4 =>
            {
                (self.drawing_area_right as u32) |
                (self.drawing_area_bottom as u32) << 10
            }

            // Draw offset
            5 =>
            {
                // Mask the value as we performed sign extension when writing the value
                ((self.drawing_offset_x & 0x7FF) as u32) |
                ((self.drawing_offset_y & 0x7FF) as u32 )<< 11

            }

            _ => self.read_response
        }
    }
}
