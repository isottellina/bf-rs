// machine.rs --- 
// 
// Filename: machine.rs
// Author: Louise <louise>
// Created: Mon Feb  5 12:07:43 2018 (+0100)
// Last-Updated: Tue Feb 13 11:23:42 2018 (+0100)
//           By: Louise <louise>
//
use std::fs::File;
use std::io::Read;
use std::iter::Peekable;
use std::slice::Iter;

use std::mem::transmute;

#[derive(Clone, Debug)]
enum Symbol {
    // Normal BF instructions
    Add(u8),
    Move(usize),
    Print,
    Read,
    Loop(Vec<Symbol>),

    // Loop optimisations
    Zero,
}

#[derive(Clone, Debug, Default)]
#[cfg(all(target_arch = "x86_64", target_family = "unix"))]
pub struct Machine {
    program: Vec<Symbol>
}

#[derive(Clone, Debug, Default)]
#[cfg(not(all(target_arch = "x86_64", target_family = "unix")))]
pub struct Machine {
    program: Vec<Symbol>,
    input: input::Input,

    tape: Vec<u8>,
    tape_idx: usize
}

#[cfg(not(all(target_arch = "x86_64", target_family = "unix")))]
impl Machine {
    pub fn new_with_file(file: &mut File) -> Machine {
        let mut program_s = String::new();
        let _ = file.read_to_string(&mut program_s);
        
        Machine {
            program: Symbol::parse_str(&program_s, true),
            input: Input::new(),

            tape: vec![0; 30000],
            tape_idx: 0
        }
    }

    fn run_instr(&mut self, instr: &Symbol) {
        match *instr {
            Symbol::Add(n) => self.tape[self.tape_idx] = self.tape[self.tape_idx].wrapping_add(n),
            Symbol::Move(n) => self.tape_idx = self.tape_idx.wrapping_add(n),
            Symbol::Print => print!("{}", self.tape[self.tape_idx] as char),
            Symbol::Read => self.tape[self.tape_idx] = self.input.read(),
            Symbol::Loop(ref block) => while self.tape[self.tape_idx] != 0 {
                self.run_block(&block);
            },

            // Loop optimisations
            Symbol::Zero => self.tape[self.tape_idx] = 0,
        }
    }

    fn run_block(&mut self, block: &Vec<Symbol>) {
        for instr in block {
            self.run_instr(instr);
        }
    }
    
    pub fn run(&mut self) {
        let program = self.program.clone();
        
        self.run_block(&program);
    }
}


#[cfg(all(target_arch = "x86_64", target_family = "unix"))]
impl Machine {
    pub fn new_with_file(file: &mut File) -> Machine {
        let mut program_s = String::new();
        let _ = file.read_to_string(&mut program_s);
        
        Machine {
            program: Symbol::parse_str(&program_s, true)
        }
    }
    
    fn compile_instr(&self, instr: &Symbol) -> Vec<u8> {
        match *instr {
            Symbol::Add(n) => vec![0x41, 0x80, 0x45, 0x00, n],              // add byte[r13], n
            Symbol::Move(n) => {
                let mut code: Vec<u8> = vec![0x49, 0x81, 0xC5];             // add r13, <incomplete>
                code.extend_from_slice(unsafe { &transmute::<u32, [u8; 4]>(n as u32) });
                
                code
            },
            Symbol::Print => vec![0x48, 0xc7, 0xc0, 0x01, 0x00, 0x00, 0x00, // mov rax, 1
                                  0x48, 0xc7, 0xc7, 0x00, 0x00, 0x00, 0x00, // mov rdi, 0
                                  0x48, 0xc7, 0xc2, 0x01, 0x00, 0x00, 0x00, // mov rdx, 1
                                  0x4c, 0x89, 0xee,                         // mov rsi, r13
                                  0x0f, 0x05,                               // syscall
            ],
            Symbol::Read =>  vec![0x48, 0xc7, 0xc0, 0x00, 0x00, 0x00, 0x00, // mov rax, 0
                                  0x48, 0xc7, 0xc7, 0x01, 0x00, 0x00, 0x00, // mov rdi, 1
                                  0x48, 0xc7, 0xc2, 0x01, 0x00, 0x00, 0x00, // mov rdx, 1
                                  0x4c, 0x89, 0xee,                         // mov rsi, r13
                                  0x0f, 0x05,                               // syscall
            ],
            Symbol::Loop(ref block) => {
                let mut code: Vec<u8> = vec![];
                let loop_code = self.compile_block(block);

                let forward = (loop_code.len() + 11) as i32;
                let backward = -forward;

                code.extend(vec![0x41, 0x80, 0x7D, 0x00, 0x00]);            // cmp byte[r13], 0
                
                if forward > 0x84 {
                    // Long branch
                    code.extend(vec![0x0F, 0x84]);                          // jz <incomplete>
                    code.extend_from_slice(unsafe { &transmute::<u32, [u8; 4]>(forward as u32) });
                } else {
                    // Short branch
                    let forward = (loop_code.len() + 7) as u8;
                    code.extend(vec![0x74, forward]);                       // jz <offset>
                }
                
                // Adding actual code
                code.extend(&loop_code);

                // Epilogue
                code.extend(vec![0x41, 0x80, 0x7D, 0x00, 0x00]);            // cmp byte[r13], 0

                if forward > 0x84 {
                    // Long branch
                    code.extend(vec![0x0F, 0x85]);                          // jnz <incomplete>
                    code.extend_from_slice(unsafe { &transmute::<u32, [u8; 4]>(backward as u32) });
                } else {
                    // Short branch
                    let backward = !((loop_code.len() + 7) as u8) + 1;
                    code.extend(vec![0x75, backward]);                      // jnz <offset>
                }
                
                code
            },

            // Loop Optimisations
            Symbol::Zero => vec![0x41, 0xc6, 0x45, 0x00, 0x00]              // mov byte[r13], 0
        }
    }
    
    fn compile_block(&self, block: &Vec<Symbol>) -> Vec<u8> {
        let mut compiled_block: Vec<u8> = vec![];
        
        for instr in block {
            let instr_code = self.compile_instr(instr);
            
            compiled_block.extend(instr_code);
        }

        compiled_block
    }
    
    pub fn run(&self) {
        use jit::JIT;
        
        let program = self.program.clone();
        let mut jit = JIT::new();
        let jit_code = self.compile_block(&program);

        // Prelogue
        let tape: [u8; 30000] = [0; 30000];
        let tape_addr = unsafe { transmute::<&[u8; 30000], u64>(&tape) };
        
        jit.emit(&[0x49, 0xbd]); // mov r13, <incomplete>
        jit.emit(unsafe { &transmute::<u64, [u8; 8]>(tape_addr) });

        // Actual generated code
        jit.emit(&jit_code);

        // Epilogue
        jit.emit(&[0xc3]); // ret

        // Running the code
        jit.run();
    }
}

impl Symbol {
    fn parse_block(program: &mut Peekable<Iter<char>>) -> Vec<Symbol> {
        let mut symbols: Vec<Symbol> = vec!();

        loop {
            match program.next() {
                Some(&'+') => {
                    let mut n: u8 = 1;

                    while program.peek() == Some(&&'+') {
                        program.next();
                        n = n.wrapping_add(1);
                    }
                    
                    symbols.push(Symbol::Add(n))
                },
                Some(&'-') => {
                    let mut n: u8 = 0xFF;

                    while program.peek() == Some(&&'-') {
                        program.next();
                        n = n.wrapping_sub(1);
                    }
                    
                    symbols.push(Symbol::Add(n))
                },
                Some(&'>') => {
                    let mut n: usize = 1;

                    while program.peek() == Some(&&'>') {
                        program.next();
                        n = n.wrapping_add(1);
                    }
                    
                    symbols.push(Symbol::Move(n))
                },
                Some(&'<') => {
                    let mut n: usize = usize::max_value();

                    while program.peek() == Some(&&'<') {
                        program.next();
                        n = n.wrapping_sub(1);
                    }
                    
                    symbols.push(Symbol::Move(n))
                },
                Some(&'.') => symbols.push(Symbol::Print),
                Some(&',') => symbols.push(Symbol::Read),
                Some(&'[') => symbols.push(
                    Symbol::Loop(
                        Symbol::parse_block(program)
                    )
                ),
                Some(&']') | None => break,

                Some(_) => { }
            }
        }

        symbols
    }

    pub fn parse_str(program: &str, opt: bool) -> Vec<Symbol> {
        let vec: Vec<char> = program.chars().collect();
        let mut it = vec.iter().peekable();
        let res = Symbol::parse_block(&mut it);
        
        if opt {
            res.into_iter().map(
                |elem| elem.optimize()
            ).collect()
        } else {
            res
        }
    }

    // Optimize a symbol and consumes it
    pub fn optimize(self) -> Symbol {
        match self {
            Symbol::Loop(content) => {
                // Let's see if the loop matches one of the one we can optimize

                // Opt 1. Zeroing current cell
                if let &[Symbol::Add(_)] = content.as_slice() {
                    Symbol::Zero
                } else {
                    Symbol::Loop(content)
                }
            },
            _ => self
        }
    }
}

#[cfg(not(all(target_arch = "x86_64", target_family = "unix")))]
mod input {
    use std::collections::VecDeque;
    
    #[derive(Debug, Clone, Default)]
    struct Input {
        buffer: VecDeque<u8>,   
    }
    
    impl Input {
        pub fn new() -> Input {
            Input {
                buffer: VecDeque::new(),
            }
        }
        
        pub fn read(&mut self) -> u8 {
            use std::io;
            
            if let Some(c) = self.buffer.pop_front() {
                c
            } else {
                let stdin = io::stdin();
                let mut s = String::new();
                
                let _ = stdin.read_line(&mut s);
                self.buffer = s.bytes().collect();
                
                self.read()
            }
        }
    }
}
