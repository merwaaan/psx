extern crate psx;

use psx::psx::PSX; // TODO rename to System or something

use std::env;
use std::path::Path;

fn main()
{
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
