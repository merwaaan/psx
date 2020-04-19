use psx::psx::PSX;
use glium::glutin;
use glium::glutin::event::{ Event, WindowEvent };
use glium::glutin::event_loop::{ ControlFlow, EventLoop };
use glium::{ Display, Surface };
use imgui::{ Context, FontConfig, FontId, FontSource, Ui };
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{ HiDpiMode, WinitPlatform };
use std::time::Instant;

pub struct System
{
    pub event_loop: EventLoop<()>,
    pub display: glium::Display,
    pub imgui: Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
    pub font_size: f32,

    pub font_default: FontId,
    pub font_symbols: FontId
}

pub fn init(width: u16, height: u16, title: &str) -> System
{
    // Create the display

    let event_loop = EventLoop::new();

    let context_builder = glutin::ContextBuilder::new()
        .with_vsync(true);

    let window_builder = glutin::window::WindowBuilder::new()
        .with_title(title.to_owned())
        .with_inner_size(glutin::dpi::LogicalSize::new(width, height));

    let display = Display::new(window_builder, context_builder, &event_loop).expect("Failed to initialize display");

    // Initialize Dear imgui

    let mut imgui = Context::create();

    let mut platform = WinitPlatform::init(&mut imgui);
    {
        let gl_window = display.gl_window();
        let window = gl_window.window();
        platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Rounded);
    }

    let hidpi_factor = platform.hidpi_factor();
    let font_size = (13.0 * hidpi_factor) as f32;

    let font_default = imgui.fonts().add_font(&[
        FontSource::DefaultFontData
        {
            config: Some(FontConfig
            {
                size_pixels: font_size,
                ..FontConfig::default()
            })
        }
    ]);

    let font_symbols = imgui.fonts().add_font(&[
        FontSource::TtfData
        {
            data: include_bytes!("Inconsolata-Bold.ttf"),
            size_pixels: font_size,
            config: Some(FontConfig
            {
                size_pixels: font_size,
                ..FontConfig::default()
            })
        }
    ]);

    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

    let renderer = Renderer::init(&mut imgui, &display).expect("Failed to initialize renderer");

    System
    {
        event_loop,
        display,
        imgui,
        platform,
        renderer,
        font_size,

        font_default,
        font_symbols
    }
}

impl System
{
    pub fn main_loop<F: FnMut(&mut bool, &mut Ui, &mut PSX) + 'static>(self, mut p: PSX, mut run_ui: F)
    {
        let System
        {
            event_loop,
            display,
            mut imgui,
            mut platform,
            mut renderer,
            ..
        } = self;

        let mut last_frame = Instant::now();

        event_loop.run(move |event, _, control_flow| match event
        {
            Event::NewEvents(_) =>
            {
                last_frame = imgui.io_mut().update_delta_time(last_frame);
            },

            Event::MainEventsCleared =>
            {
                let gl_window = display.gl_window();
                platform
                    .prepare_frame(imgui.io_mut(), &gl_window.window())
                    .expect("Failed to prepare frame");
                gl_window.window().request_redraw();
            }

            Event::RedrawRequested(_) =>
            {
                let mut ui = imgui.frame();

                let mut run = true;
                run_ui(&mut run, &mut ui, &mut p);
                if !run
                {
                    *control_flow = ControlFlow::Exit;
                }

                let gl_window = display.gl_window();

                let mut target = display.draw();

                // Clear
                target.clear_color_srgb(0.0, 0.0, 0.0, 1.0);

                // Draw the GPU output
                p.gpu_mut().render(&mut target);

                // Draw the UI
                platform.prepare_render(&ui, gl_window.window());
                let draw_data = ui.render();
                renderer
                    .render(&mut target, draw_data)
                    .expect("Rendering failed");

                target.finish().expect("Failed to swap buffers");
            }

            Event::WindowEvent
            {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,

            event =>
            {
                let gl_window = display.gl_window();
                platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
            }
        })
    }
}
