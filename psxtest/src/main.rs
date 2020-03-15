extern crate psx;

use psx::psx::PSX; // TODO rename to System or something

use imgui::*;
use std::env;
use std::path::Path;

mod support;

fn main()
{
    let system = support::init(1600, 800, file!());

    // Initialize the emulation

    let args: Vec<String> = env::args().collect();
    if args.len() < 2
    {
        panic!("Usage: psxtest <bios>");
    }

    let mut p = PSX::new(&Path::new(&args[1]), &system.display);

    p.cpu.debugger.load("debugger.json").expect("cannot load debugger");

    let mut is_running = false;
    let mut new_breakpoint: i32 = 0;
    let mut memory_current_address: i32 = 0;

    // Build the UI

    system.main_loop(p, move |_run, ui, p|
    {
        //ui.show_demo_window(run);

        /*for (key, state) in ui.io().keys_down.iter().enumerate()
        {
            if ui.is_key_released(key as u32)
            {
                println!("RELEASED {}", key);
            }
        }*/

        // F1: Step
        if ui.is_key_released(37)
        {
            p.step();
            //p.gpu().render(&system.display);
        }
        // F2: Resume/Stop
        else if ui.is_key_released(38)
        {
            is_running = !is_running;
        }

        if is_running
        {
            is_running = p.run(1_000_000);
            //p.gpu().render(&system.display);
        }

        Window::new(im_str!("Breakpoints"))
            .position([0.0, 0.0], Condition::FirstUseEver)
            .size([300.0, 0.0], Condition::FirstUseEver)
            .collapsed(true, Condition::FirstUseEver)
            .build(ui, ||
            {
                ui.text(format!("Counter: {}", p.cpu.counter));

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
            .collapsed(true, Condition::FirstUseEver)
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
            .collapsed(true, Condition::FirstUseEver)
            .build(ui, ||
            {
                for i in 0..10
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
            .collapsed(true, Condition::FirstUseEver)
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

        Window::new(im_str!("SPU"))
            .position([400.0, 300.0], Condition::FirstUseEver)
            .size([200.0, 0.0], Condition::FirstUseEver)
            .collapsed(true, Condition::FirstUseEver)
            .build(ui, ||
            {
                // SPU status

                ui.text(im_str!("Status"));
                ui.text(format!("{:016X}", p.mem.spu.status.0));

                // Voices

                ui.text(im_str!("Voices"));

                ui.columns(2, im_str!(""), false);

                for i in 0 .. 24
                {
                    ui.text(format!("{}", i));
                    ui.next_column();

                    let voice_on = if p.mem.spu.channel_status & (1 << i) != 0 {im_str!("ON")} else {im_str!("OFF")};
                    ui.text(voice_on);
                    ui.next_column();
                }
            });

        Window::new(im_str!("GPU"))
            .position([500.0, 300.0], Condition::FirstUseEver)
            .size([400.0, 400.0], Condition::FirstUseEver)
            .collapsed(true, Condition::FirstUseEver)
            .build(ui, ||
            {
                // GPU status

                ui.text(format!("Status: {:08X}", p.gpu().status()));

                // Commands

                ui.text(im_str!("Commands"));

                ui.columns(3, im_str!(""), false);

                for command in p.gpu().previous_commands.iter()
                {
                    ui.text(format!("GP{}", command.0 as u8));
                    ui.next_column();

                    ui.text(format!("{:08X}", command.1));
                    ui.next_column();

                    ui.text(format!("{}", p.gpu().disassemble(command)));
                    ui.next_column();
                }
            });

        /*if !is_running
        {
            Window::new(im_str!("Pause"))
                .position([500.0, 0.0], Condition::FirstUseEver)
                .size([0.0, 0.0], Condition::FirstUseEver)
                .collapsed(true, Condition::FirstUseEver)
                .build(ui, ||
                {
                    ui.text(im_str!("paused"));
                });
        }*/
    });
}
