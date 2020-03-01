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

    p.cpu.debugger.load("debugger.json").expect("cannot load debugger");

    //

    let mut is_paused = true;
    let mut new_breakpoint: i32 = 0;
    let mut memory_current_address: i32 = 0;

    let system = support::init(file!());
    system.main_loop(move |_run, ui|
    {
        //ui.show_demo_window(run);

        /*for (key, state) in ui.io().keys_down.iter().enumerate()
        {
            if ui.is_key_released(key as u32)
            {
                println!("RELEASED {}", key);
            }
        }*/

        if ui.is_key_released(37)
        {
            p.step();
        }
        else if ui.is_key_released(38)
        {
            is_paused = false;
            p.run();
            is_paused = true;
        }

        Window::new(im_str!("Breakpoints"))
            .position([0.0, 0.0], Condition::FirstUseEver)
            .size([300.0, 0.0], Condition::FirstUseEver)
            .build(ui, ||
            {
                for b in p.cpu.debugger.get_breakpoints().to_vec()
                {
                    ui.text(format!("0x{:08X}", b));
                    ui.same_line(0.0);

                    if ui.small_button(&ImString::new(format!("Remove##{}", b)))
                    {
                        println!("removing {:08x}", b);
                        p.cpu.debugger.remove_breakpoint(b);
                        p.cpu.debugger.save("debugger.json").expect("cannot save debugger");
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
                    p.cpu.debugger.save("debugger.json").expect("cannot save debugger");
                }
            });

        Window::new(im_str!("Registers"))
            .position([0.0, 100.0], Condition::FirstUseEver)
            .size([400.0, 0.0], Condition::FirstUseEver)
            .build(ui, ||
            {
                ui.text(format!("PC {:08X}", p.cpu.pc));
                ui.separator();

                ui.columns(4, im_str!(""), false);

                for i in 0..32
                {
                    if i > 0 && i % 8 == 0
                    {
                        ui.next_column();
                    }

                    ui.text(format!("R{: <2} {:08X}", i, p.cpu.r[i]));
                }

                ui.columns(1, im_str!(""), false);
                ui.separator();

                ui.columns(2, im_str!(""), false);
                ui.text(format!("HI {:08X}", p.cpu.hi));
                ui.next_column();
                ui.text(format!("LO {:08X}", p.cpu.lo));
            });

        const COLOR_DIMMED: [f32; 4]   = [0.5, 0.5, 0.5, 1.0];
        const _COLOR_DISABLED: [f32; 4] = [0.25, 0.25, 0.25, 1.0];
        const COLOR_ACCENT: [f32; 4]   = [1.0, 0.07, 0.57, 1.0];
        const COLOR_DEFAULT: [f32; 4]  = [1.0, 1.0, 1.0, 1.0];

        Window::new(im_str!("Instructions"))
            .position([0.0, 400.0], Condition::FirstUseEver)
            .size([0.0, 0.0], Condition::FirstUseEver)
            .build(ui, ||
            {
                for i in -10..10
                {
                    let pc = p.cpu.pc.wrapping_add(i as u32 * 4);
                    let disasm = p.cpu.debugger.disassemble(pc, &p.cpu, &p.mem);

                    ui.text_colored(
                        if i == 0 { COLOR_ACCENT } else { COLOR_DEFAULT },
                        format!("{:08X}    {:08X}    {}", pc, disasm.bits, disasm.mnemonics));
                }
            });

        Window::new(im_str!("Memory"))
            .position([500.0, 0.0], Condition::FirstUseEver)
            .size([0.0, 0.0], Condition::FirstUseEver)
            .build(ui, ||
            {
                for offset in (memory_current_address .. memory_current_address + 16 * 20).step_by(16)
                {
                    ui.text(format!("{:08X}:", offset));
                    ui.same_line_with_spacing(0.0, 15.0);

                    for i in 0 .. 16
                    {
                        let value = p.mem.read8(offset as u32 + i);

                        ui.text_colored(
                            if value == 0 { COLOR_DIMMED } else { COLOR_DEFAULT },
                            format!("{:02X}", value));

                        ui.same_line_with_spacing(0.0, if i != 15 { 5.0 } else { 15.0 });
                    }

                    for i in 0 .. 16
                    {
                        let value = p.mem.read8(offset as u32 + i);

                        ui.text_colored(
                            if value == 0 { COLOR_DIMMED } else { COLOR_DEFAULT },
                            format!("{}", value as char));

                        if i != 15
                        {
                            ui.same_line_with_spacing(0.0, 0.0);
                        }
                    }
                }

                ui.separator();

                ui.input_int(im_str!(""), &mut memory_current_address)
                    .step(16)
                    .chars_hexadecimal(true)
                    .chars_uppercase(true)
                    .build();

                // Make sure the starting address is always aligned on 16
                memory_current_address -= memory_current_address % 16;

                ui.same_line(0.0);
            });

        if is_paused
        {
            Window::new(im_str!("Pause"))
                .position([500.0, 500.0], Condition::FirstUseEver)
                .size([0.0, 0.0], Condition::FirstUseEver)
                .build(ui, ||
                {
                    ui.text(im_str!("paused"));
                });
        }
    });
}
