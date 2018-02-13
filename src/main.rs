// main.rs --- 
// 
// Filename: main.rs
// Author: Louise <louise>
// Created: Mon Feb  5 11:53:25 2018 (+0100)
// Last-Updated: Tue Feb 13 11:20:59 2018 (+0100)
//           By: Louise <louise>
//
#![feature(slice_patterns)]
#[macro_use] extern crate log;
extern crate simplelog;

extern crate libc;

mod machine;
mod jit;

use std::env;
use std::fs::File;

use machine::Machine;

fn main() {
    let _ = simplelog::TermLogger::init(
        simplelog::LogLevelFilter::Info,
        simplelog::Config::default()
    ).unwrap();
    
    if let Some(filename) = env::args().nth(1) {
        if let Ok(mut file) = File::open(filename) {
            let mut machine = Machine::new_with_file(&mut file);

            machine.run();
        } else {
            error!("We couldn't open the file");
        }
    } else {
        error!("You didn't provide a file");
    }
}
