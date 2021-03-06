extern crate psx;

use psx::psx::PSX; // TODO rename to System or something

use imgui::*;
use std::env;
use std::path::PathBuf;

mod support;

fn main()
{
    // Check the arguments

    let args: Vec<String> = env::args().collect();
    if args.len() < 2
    {
        panic!("Usage: psxtest <bios> [game]");
    }

    let mut bios_path = PathBuf::new();
    bios_path.push(&args[1]);

    let program_path = match args.len()
    {
        2 => None,
        _ =>
        {
            let mut path = PathBuf::new();
            path.push(&args[2]);
            Some(path)
        }
    };

    // Initialize the emulation

    let system = support::init(1600, 800, file!());

    let mut p = PSX::new(bios_path, program_path, &system.display);

    match p.cpu.debugger.load("debugger.json")
    {
        Ok(_) => (),
        Err(error) => println!("cannot load debugger from file {:?}", error)
    }

    let mut is_running = true;
    let mut new_breakpoint: i32 = 0;
    let mut new_breakpoint_cond_reg: i32 = 0;
    let mut new_breakpoint_cond_val: i32 = 0;
    let mut new_data_breakpoint: i32 = 0;
    let mut memory_current_address: i32 = 0;

    // Build the UI

    system.main_loop(p, move |_run, ui, p|
    {
        //let mut op = true; ui.show_demo_window(&mut op);

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
                let mut need_saving = false;

                ui.text(format!("Counter: {}", p.cpu.counter));

                // Code breakpoints

                ui.separator();

                for b in p.cpu.debugger.get_breakpoints().to_vec()
                {
                    // Breakpoint on/off

                    let mut breakpoint_enabled = b.enabled;
                    let breakpoint_enabled_changed = ui.checkbox(&ImString::new(format!("##{}", b.address)), &mut breakpoint_enabled);

                    if breakpoint_enabled_changed
                    {
                        p.cpu.debugger.toggle_breakpoint(b.address, breakpoint_enabled);
                        need_saving = true;
                    }

                    ui.same_line(0.0);

                    // Address

                    ui.text_colored(
                        if b.address == p.cpu.pc { COLOR_ACCENT } else { COLOR_DEFAULT },
                        format!("0x{:08X}", b.address));

                    ui.same_line(0.0);

                    // Remove button

                    if ui.small_button(&ImString::new(format!("Remove##{}", b.address)))
                    {
                        p.cpu.debugger.remove_breakpoint(b.address);
                        need_saving = true;
                        continue;
                    }

                    ui.same_line(0.0);

                    if ui.small_button(&ImString::new(format!("Add register condition##{}", b.address)))
                    {
                        p.cpu.debugger.add_breakpoint_condition(b.address);
                        need_saving = true;
                    }

                    for bc in p.cpu.debugger.get_breakpoint_conditions_mut(b.address)
                    {
                        // TODO combo
                        new_breakpoint_cond_reg = bc.register as i32;
                        let reg_changed = ui.input_int(im_str!("R"), &mut new_breakpoint_cond_reg)
                            .build();

                        ui.same_line(0.0);

                        new_breakpoint_cond_val = bc.value as i32;
                        let val_changed = ui.input_int(im_str!("="), &mut new_breakpoint_cond_val)
                            .chars_hexadecimal(true)
                            .chars_uppercase(true)
                            .build();

                        if reg_changed || val_changed
                        {
                            bc.register = new_breakpoint_cond_reg as u8;
                            bc.value = new_breakpoint_cond_val as u32;
                            need_saving = true;
                        }
                    }
                }

                ui.input_int(im_str!(""), &mut new_breakpoint)
                    .chars_hexadecimal(true)
                    .chars_uppercase(true)
                    .build();

                ui.same_line(0.0);

                if ui.small_button(im_str!("Add"))
                {
                    p.cpu.debugger.add_breakpoint(new_breakpoint as u32);
                    need_saving = true;
                }

                ui.separator();

                // Data breakpoints

                let mut data_breakpoint_to_remove = None;

                let breakpoints_hit = p.cpu.debugger.get_data_breakpoints_hit().to_vec();

                for b in p.cpu.debugger.get_data_breakpoints_mut()
                {
                    // On read / On write

                    let mut on_read = b.on_read;
                    if ui.checkbox(&ImString::new(format!("R##r{}", b.address)), &mut on_read)
                    {
                        b.on_read = on_read;
                        need_saving = true;
                    }

                    ui.same_line(0.0);

                    let mut on_write = b.on_write;
                    if ui.checkbox(&ImString::new(format!("W##w{}", b.address)), &mut on_write)
                    {
                        b.on_write = on_write;
                        need_saving = true;
                    }

                    ui.same_line(0.0);

                    // Address

                    // TODO
                    ui.text_colored(
                        if breakpoints_hit.iter().any(|a| *a == b.address) { COLOR_ACCENT } else { COLOR_DEFAULT },
                        format!("0x{:08X}", b.address));

                    ui.same_line(0.0);

                    // Remove button

                    if ui.small_button(&ImString::new(format!("Remove##data{}", b.address)))
                    {
                        data_breakpoint_to_remove = Some(b.address);
                    }
                }

                if data_breakpoint_to_remove.is_some()
                {
                    p.cpu.debugger.remove_data_breakpoint(data_breakpoint_to_remove.unwrap());
                    need_saving = true;
                }

                ui.input_int(im_str!("##data"), &mut new_data_breakpoint)
                    .chars_hexadecimal(true)
                    .chars_uppercase(true)
                    .build();

                ui.same_line(0.0);

                if ui.small_button(im_str!("Add##data"))
                {
                    p.cpu.debugger.add_data_breakpoint(new_data_breakpoint as u32);
                    need_saving = true;
                }

                if need_saving
                {
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
            .position([0.0, 700.0], Condition::FirstUseEver)
            .size([0.0, 0.0], Condition::FirstUseEver)
            .collapsed(true, Condition::FirstUseEver)
            .build(ui, ||
            {
                for i in -10 .. 10
                {
                    let pc = p.cpu.pc.wrapping_add(i as u32 * 4);
                    let disasm = p.cpu.debugger.disassemble(pc, &p.cpu, &mut p.mem);

                    ui.text_colored(
                        if i == 0 { COLOR_ACCENT } else { COLOR_DEFAULT },
                        format!("{:08X}    {:08X}    {}", pc, disasm.bits, disasm.mnemonics));

                    ui.same_line(0.0);

                    //let font_stack = ui.push_font(system.font_symbols);
                    ui.text_colored(COLOR_DIMMED, &disasm.hint);
                    //font_stack.pop(&ui);

                    if ui.is_item_clicked(MouseButton::Left)
                    {
                        match u32::from_str_radix(&disasm.hint, 16)
                        {
                            Ok(target) => memory_current_address = target as i32,
                            _ => println!("cannot convert hex \"{}\" to u32", &disasm.hint)
                        }
                    }
                }
            });

        Window::new(im_str!("Memory"))
            .position([500.0, 0.0], Condition::FirstUseEver)
            .size([0.0, 0.0], Condition::FirstUseEver)
            .collapsed(true, Condition::FirstUseEver)
            .build(ui, ||
            {
                // Header row

                ui.text("         ");
                ui.same_line_with_spacing(0.0, 15.0);

                for i in 0 .. 16
                {
                    ui.text_colored(COLOR_DIMMED, format!("{:02X}", i));

                    let spacing = match i
                    {
                        7  => 10.0, // middle of row
                        15 => 15.0, // end of row,
                        _  => 5.0
                    };

                    if i != 15
                    {
                        ui.same_line_with_spacing(0.0, spacing);
                    }
                }

                // Memory

                // Make sure the starting address is always aligned on 16
                memory_current_address -= (memory_current_address as u32 % 16) as i32;

                for offset in (memory_current_address .. memory_current_address + 16 * 20).step_by(16)
                {
                    ui.text(format!("{:08X}:", offset));
                    ui.same_line_with_spacing(0.0, 15.0);

                    for i in 0 .. 16
                    {
                        let value = p.mem.read::<u8>(offset as u32 + i);

                        ui.text_colored(
                            if value == 0 { COLOR_DIMMED } else { COLOR_DEFAULT },
                            format!("{:02X}", value));

                        if ui.is_item_hovered()
                        {
                            ui.tooltip_text(format!("{:08X}", offset + i as i32));
                        }

                        let spacing = match i
                        {
                            7  => 10.0, // middle of row
                            15 => 15.0, // end of row,
                            _  => 5.0
                        };

                        ui.same_line_with_spacing(0.0, spacing);
                    }

                    for i in 0 .. 16
                    {
                        let value = p.mem.read::<u8>(offset as u32 + i);

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
                ui.set_column_width(0, 40.0);
                ui.set_column_width(1, 100.0);

                let mut i = 0;
                for command in p.gpu().previous_commands.iter()
                {
                    ui.text(format!("{} GP{}", i, command.0 as u8));
                    ui.next_column();

                    ui.text(format!("{:08X}", command.1[0]));
                    ui.next_column();

                    ui.text(format!("{}", p.gpu().disassemble(command)));
                    ui.next_column();

                    i += 1;
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
