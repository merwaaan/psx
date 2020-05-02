use glium::*; // TODO clean up
use glium::backend::Facade;

use std::rc::Rc;

#[derive(Debug, Copy, Clone)]
pub struct Position(pub i16, pub i16);

impl Position
{
    pub fn from_command(value: u32) -> Position
    {
        Position(value as i16, (value >> 16) as i16)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Color(pub u8, pub u8, pub u8);

impl Color
{
    pub fn from_command(value: u32) -> Color
    {
        Color(value as u8, (value >> 8) as u8, (value >> 16) as u8)
    }
}

pub struct Renderer
{
    context: Rc<glium::backend::Context>,
    render_buffer: glium::framebuffer::RenderBuffer,
    program: glium::Program,

    vertex_buffer: glium::VertexBuffer<Vertex>,
    vertex_index: usize
}

#[derive(Debug, Copy, Clone)]
struct Vertex
{
    position: [i16; 2],
    color: [u8; 3]
}

implement_vertex!(Vertex, position, color);

const MAX_BUFFER_SIZE: usize = 5_000_000;

impl Renderer
{
    pub fn new(display: &glium::Display) -> Renderer
    {
        let render_buffer = glium::framebuffer::RenderBuffer::new(
            display,
            glium::texture::UncompressedFloatFormat::F32F32F32,
            1024,
            512).unwrap();

        let program = glium::Program::from_source(display, VERTEX_SHADER_SOURCE, FRAGMENT_SHADER_SOURCE, None).unwrap();

        let vertex_buffer = glium::VertexBuffer::empty_dynamic(display, MAX_BUFFER_SIZE).unwrap();

        Renderer
        {
            context: display.get_context().clone(),

            render_buffer,
            program,

            vertex_buffer,
            vertex_index: 0
        }
    }

    pub fn render(&mut self, target: &mut glium::Frame)
    {
        // Draw to the render buffer

        let mut framebuffer = glium::framebuffer::SimpleFrameBuffer::new(&self.context, &self.render_buffer).unwrap();

        let index_buffer = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

        framebuffer.draw(&self.vertex_buffer, &index_buffer, &self.program, &glium::uniforms::EmptyUniforms, &Default::default()).unwrap();

        // Blit the render texture to the target texture

        let source_rect = glium::Rect { left: 0, bottom: 0, width: 1024, height: 512 };
        let target_rect = glium::BlitTarget { left: 0, bottom: 0, width: 1600, height: 800 }; // TODO for now = same size as window, clean this up

        //framebuffer.blit_whole_color_to(target, &target_rect, uniforms::MagnifySamplerFilter::Linear);
        framebuffer.blit_color(&source_rect, target, &target_rect, uniforms::MagnifySamplerFilter::Linear);

        // Reset the vertex buffer's content

        let dummy_vertex = Vertex { position: [0, 0], color: [0, 0, 0] };

        let mut w = self.vertex_buffer.map_write();
        for i in 0 .. self.vertex_index
        {
            w.set(i, dummy_vertex);
        }

        self.vertex_index = 0;
    }

    pub fn push_triangle(&mut self, positions: [Position; 3], colors: [Color; 3])
    {
        if self.vertex_index + 3 > MAX_BUFFER_SIZE
        {
            warn!("Vertex buffer full, ignoring triangle");
            return;
        }

        let vertex1 = Vertex { position: [positions[0].0, positions[0].1], color: [colors[0].0, colors[0].1, colors[0].2] };
        let vertex2 = Vertex { position: [positions[1].0, positions[1].1], color: [colors[1].0, colors[1].1, colors[1].2] };
        let vertex3 = Vertex { position: [positions[2].0, positions[2].1], color: [colors[2].0, colors[2].1, colors[2].2] };

        let mut w = self.vertex_buffer.map_write();
        w.set(self.vertex_index, vertex1);
        w.set(self.vertex_index + 1, vertex2);
        w.set(self.vertex_index + 2, vertex3);

        self.vertex_index += 3;
    }

    pub fn push_quad(&mut self, positions: [Position; 4], colors: [Color; 4])
    {
        if self.vertex_index + 6 > MAX_BUFFER_SIZE
        {
            warn!("Vertex buffer full, ignoring triangle");
            return;
        }

        let vertex1 = Vertex { position: [positions[0].0, positions[0].1], color: [colors[0].0, colors[0].1, colors[0].2] };
        let vertex2 = Vertex { position: [positions[1].0, positions[1].1], color: [colors[1].0, colors[1].1, colors[1].2] };
        let vertex3 = Vertex { position: [positions[2].0, positions[2].1], color: [colors[2].0, colors[2].1, colors[2].2] };

        let vertex4 = Vertex { position: [positions[3].0, positions[3].1], color: [colors[3].0, colors[3].1, colors[3].2] };
        let vertex5 = Vertex { position: [positions[2].0, positions[2].1], color: [colors[2].0, colors[2].1, colors[2].2] };
        let vertex6 = Vertex { position: [positions[1].0, positions[1].1], color: [colors[1].0, colors[1].1, colors[1].2] };

        let mut w = self.vertex_buffer.map_write();
        w.set(self.vertex_index, vertex1);
        w.set(self.vertex_index + 1, vertex2);
        w.set(self.vertex_index + 2, vertex3);
        w.set(self.vertex_index + 3, vertex4);
        w.set(self.vertex_index + 4, vertex5);
        w.set(self.vertex_index + 5, vertex6);

        self.vertex_index += 6;
    }
}

const VERTEX_SHADER_SOURCE: &str = "
#version 140

in vec2 position;
in vec3 color;

out vec3 color2;

void main()
{
    // VRAM coordinates ([0;1023], [0;512]) into GL coordinates ([-1;1], [-1;1])
    float x = float(position.x) / 512 - 1.0f;
    float y = -1.0f * (float(position.y) / 256 - 1.0f); // mirror vertically

    gl_Position = vec4(x, y, 0.0, 1.0);

    color2 = vec3(color.x / 255.0f, color.y / 255.0f, color.z / 255.0f);
}";


const FRAGMENT_SHADER_SOURCE: &str = "
#version 140

in vec3 color2;

out vec4 out_color;

void main()
{
    out_color = vec4(color2, 1.0);
}";
