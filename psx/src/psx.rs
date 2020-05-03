use crate::cpu::CPU;
use crate::gpu::GPU;
use crate::interrupt_controller::InterruptController;
use crate::memory::Memory;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub struct PSX
{
    pub mem: Memory,
    pub cpu: CPU,
    interrupt_controller: Rc<RefCell<InterruptController>>
}

impl PSX
{
    pub fn new(bios_path: PathBuf, program_path: Option<PathBuf>, display: &glium::Display) -> Self
    {
        env_logger::init();

        let _interrupt_controller = Rc::new(RefCell::new(InterruptController::new()));

        // If the program is stored in an EXE file, we'll need
        // to hot-load it after the BIOS has been initialized

        let exe_path = program_path.and_then(|path|
        {
            path.extension()
                .and_then(|ext| ext.to_str()) // OsStr to &str
                .and_then(|ext|
                    match ext
                    {
                        "exe" => Some(path.clone()),
                        _ => None
                    }
                )
        });

        PSX
        {
            mem: Memory::new(bios_path, display, &_interrupt_controller),
            cpu: CPU::new(&_interrupt_controller, exe_path),
            interrupt_controller: _interrupt_controller
        }
    }

    pub fn load_bios()
    {

    }

    pub fn step(&mut self)
    {
        self.cpu.step(&mut self.mem);
    }

    pub fn run(&mut self, instructions: u32) -> bool
    {
        self.cpu.run(instructions, &mut self.mem)
    }

    // TEMP
    pub fn gpu(&self) -> &GPU
    {
        &self.mem.gpu
    }
    pub fn gpu_mut(&mut self) -> &mut GPU
    {
        &mut self.mem.gpu
    }
}
