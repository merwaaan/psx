extern crate psx;

use psx::psx::PSX; // TODO rename to System or something

use imgui::*;
use std::env;
use std::path::Path;

mod support;

fn main()
{
    let system = support::init(file!());
    system.main_loop(move |_, ui| {
        Window::new(im_str!("Hello world"))
            .size([300.0, 100.0], Condition::FirstUseEver)
            .build(ui, || {
                ui.text(im_str!("Hello world!"));
                ui.text(im_str!("こんにちは世界！"));
                ui.text(im_str!("This...is...imgui-rs!"));
                ui.separator();
                let mouse_pos = ui.io().mouse_pos;
                ui.text(format!(
                    "Mouse Position: ({:.1},{:.1})",
                    mouse_pos[0], mouse_pos[1]
                ));
            });
    });

    // Load the BIOS

    let args: Vec<String> = env::args().collect();
    if args.len() < 2
    {
        panic!("Usage: psxtest <bios>");
    }

    //

    let mut p = PSX::new(&Path::new(&args[1]));

    loop
    {
        p.step();
    }
}
