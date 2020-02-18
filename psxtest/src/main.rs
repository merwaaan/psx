extern crate psx;

use psx::psx::PSX; // TODO rename to System or something

use imgui::*;
use std::env;
use std::path::Path;

mod support;

fn main()
{
    // Load the BIOS

    let args: Vec<String> = env::args().collect();
    if args.len() < 2
    {
        panic!("Usage: psxtest <bios>");
    }

    let mut p = PSX::new(&Path::new(&args[1]));

    //

    let mut new_breakpoint = 0;

    let system = support::init(file!());
    system.main_loop(move |run, ui|
    {
        ui.show_demo_window(run);

        Window::new(im_str!("Breakpoints"))
            .position([0.0, 0.0], Condition::FirstUseEver)
            .size([300.0, 100.0], Condition::FirstUseEver)
            .build(ui, ||
            {
                for b in p.cpu.debugger.get_breakpoints().to_vec()
                {
                    ui.text(format!("0x{:08X}", b));
                    ui.same_line(0.0);
                    if ui.small_button(im_str!("Remove"))
                    {
                        p.cpu.debugger.remove_breakpoint(b);
                    }
                }

                ui.separator();

                ui.input_int(im_str!(""), &mut new_breakpoint)
                    .chars_hexadecimal(true)
                    .chars_uppercase(true)
                    .build();

                ui.same_line(0.0);

                if ui.small_button(im_str!("Add"))
                {
                    p.cpu.debugger.add_breakpoint(new_breakpoint as u32);
                }
            });

        Window::new(im_str!("Registers"))
            .position([0.0, 200.0], Condition::FirstUseEver)
            .size([300.0, 100.0], Condition::FirstUseEver)
            .build(ui, ||
            {
                ui.text(format!("PC {:08X}", p.cpu.pc));
                ui.separator();

                for i in 0..32
                {
                    ui.columns(2, im_str!(""), false);
                    ui.text(format!("R{} {:08X}", i, p.cpu.r[i]));

                    if i == 15
                    {
                        ui.next_column();
                    }
                }

                ui.columns(1, im_str!(""), false);
                ui.separator();

                ui.columns(2, im_str!(""), false);
                ui.text(format!("HI {:08X}", p.cpu.hi));
                ui.next_column();
                ui.text(format!("LO {:08X}", p.cpu.lo));
            });

        Window::new(im_str!("Instructions"))
            .position([0.0, 400.0], Condition::FirstUseEver)
            .size([300.0, 100.0], Condition::FirstUseEver)
            .build(ui, ||
            {
                for i in 0..10
                {
                    let pc = p.cpu.pc + i * 4;
                    let opcode = p.mem.read(pc);
                    let disasm = p.cpu.debugger.disassemble(opcode, &p.cpu, &p.mem);

                    ui.text(format!("{:08X} {:08X} {}", pc, opcode, disasm));
                }
            });

        //p.step();
    });
}
