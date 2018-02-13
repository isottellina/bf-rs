// jit.rs --- 
// 
// Filename: jit.rs
// Author: Louise <louise>
// Created: Sat Feb 10 17:52:12 2018 (+0100)
// Last-Updated: Sat Feb 10 19:13:45 2018 (+0100)
//           By: Louise <louise>
//
use std;
use std::mem;
use libc::{posix_memalign, mprotect, PROT_READ, PROT_WRITE, PROT_EXEC};

pub struct JIT {
    code: Vec<u8>
}

impl JIT {
    pub fn new() -> JIT {
        JIT {
            code: vec![],
        }
    }

    pub fn emit(&mut self, code: &[u8]) {
        self.code.extend_from_slice(code);
    }

    pub fn run(&self) {
        let zone_ptr: *mut u8 = unsafe { mem::zeroed() };
        let zone_size = ((self.code.len() >> 12) + 1) << 12;

        // Allocating the zone (in R/W for now)
        unsafe {
            posix_memalign(mem::transmute(&zone_ptr), 0x1000, zone_size);
            mprotect(mem::transmute(zone_ptr), zone_size, PROT_READ | PROT_WRITE);
        }

        let mut zone: Box<[u8]> = unsafe { Box::from_raw(std::slice::from_raw_parts_mut(zone_ptr, zone_size)) };

        // Writing C3 (RET) to the zone, to avoid most issues
        for idx in 0..zone_size {
            zone[idx] = 0xc3;
        }
        
        // Writing the code to the zone
        for (idx, byte) in self.code.iter().enumerate() {
            zone[idx] = *byte;
        }

        // The zone must now be in R/X (not writable but executable)
        unsafe {
            mprotect(mem::transmute(zone_ptr), zone_size, PROT_READ | PROT_EXEC);
        }

        // Get the zone as a function pointer
        let f: fn() = unsafe { mem::transmute(zone_ptr) };

        // Executing the code emitted
        f();
    }
}
