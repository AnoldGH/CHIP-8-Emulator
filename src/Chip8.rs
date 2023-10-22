pub struct Chip8 {
    pub registers: [u8; 16],
    pub memory: [u8; 4096],
    pub index: u16,
    pub pc: u16,
    pub stack: [u16; 16],
    pub sp: u8,
    pub delay_timer: u8,
    pub sound_timer: u8,
    pub keypad: [u8; 16],
    pub video: [u8; 64 * 32],
    pub opcode: u16,
    pub rand_byte: rand::distributions::Uniform<u8>,
    pub rng_core: rand::rngs::ThreadRng,
    pub table: [fn(&mut Chip8); 0x10],
    pub table_0: [fn(&mut Chip8); 0xF],
    pub table_8: [fn(&mut Chip8); 0xF],
    pub table_e: [fn(&mut Chip8); 0xF],
    pub table_f: [fn(&mut Chip8); 0x66],
}

use std::arch::x86_64::_MM_FROUND_NINT;
use std::{fs::File, arch::x86_64::_MM_FROUND_NEARBYINT};
use std::io::Read;
use rand::{Rng, thread_rng, RngCore};

const START_ADDRESS: u16 = 0x200;

// Sprites
const FONTSET_SIZE: usize = 80;
const FONTSET_START_ADDRESS: usize = 0x50;
const FONTSET: [u8; FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub const VIDEO_HEIGHT: u8 = 32;
pub const VIDEO_WIDTH: u8 = 64;

impl Chip8 {
    pub fn load_rom(&mut self, filename: &str) -> Result<(), std::io::Error> {
        // Open the file as a binary read-only stream
        //  and move the file pointer to the end
        let mut file = File::open(filename)?;

        // Get the file size
        let metadata = file.metadata()?;
        let size = metadata.len() as usize;

        // Allocate a buffer to hold the contents
        let mut buffer = Vec::with_capacity(size);

        // Read the file contents into the buffer
        file.read_to_end(&mut buffer)?;

        // Load the ROM contents into Chip8's memory, starting at 0x200
        for (i, byte) in buffer.iter().enumerate() {
            self.memory[START_ADDRESS as usize + i] = *byte;
        }

        Ok(())
    }

    // Initialize Program Counter
    pub fn new() -> Chip8 {
        let mut memory = [0; 4096];
        
        // Load fonts into memory
        for i in 0..FONTSET_SIZE {
            memory[FONTSET_START_ADDRESS + i] = FONTSET[i];
        }

        // Initialize RNG
        let rand_byte = rand::distributions::Uniform::new(0, 255);
        let rng_core = rand::thread_rng();

        let mut chip8 = Chip8 {
            registers: [0; 16],
            memory,
            index: 0,
            pc: START_ADDRESS,
            stack: [0; 16],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            keypad: [0; 16],
            video: [0; 64 * 32],
            opcode: 0,
            rand_byte,
            rng_core,
            table: [Chip8::op_null; 0x10],
            table_0: [Chip8::op_null; 0xF],
            table_8: [Chip8::op_null; 0xF],
            table_e: [Chip8::op_null; 0xF],
            table_f: [Chip8::op_null; 0x66],
        };

        chip8.table[0x0] = Chip8::table_0;
        chip8.table[0x1] = Chip8::op_1nnn;
        chip8.table[0x2] = Chip8::op_2nnn;
        chip8.table[0x3] = Chip8::op_3xkk;
        chip8.table[0x4] = Chip8::op_4xkk;
        chip8.table[0x5] = Chip8::op_5xy0;
        chip8.table[0x6] = Chip8::op_6xkk;
        chip8.table[0x7] = Chip8::op_7xkk;
        chip8.table[0x8] = Chip8::table_8;
        chip8.table[0x9] = Chip8::op_9xy0;
        chip8.table[0xA] = Chip8::op_annn;
        chip8.table[0xB] = Chip8::op_bnnn;
        chip8.table[0xC] = Chip8::op_cxkk;
        chip8.table[0xD] = Chip8::op_dxyn;
        chip8.table[0xE] = Chip8::table_e;
        chip8.table[0xF] = Chip8::table_f;

        // Sub-tables
        chip8.table_0[0x0] = Chip8::op_00e0;
        chip8.table_0[0xE] = Chip8::op_00ee;

        chip8.table_8[0x0] = Chip8::op_8xy0;
        chip8.table_8[0x1] = Chip8::op_8xy1;
        chip8.table_8[0x2] = Chip8::op_8xy2;
        chip8.table_8[0x3] = Chip8::op_8xy3;
        chip8.table_8[0x4] = Chip8::op_8xy4;
        chip8.table_8[0x5] = Chip8::op_8xy5;
        chip8.table_8[0x6] = Chip8::op_8xy6;
        chip8.table_8[0x7] = Chip8::op_8xy7;
        chip8.table_8[0xE] = Chip8::op_8xye;

        chip8.table_e[0x1] = Chip8::op_exa1;
        chip8.table_e[0xE] = Chip8::op_ex9e;

        chip8.table_f[0x07] = Chip8::op_fx07;
        chip8.table_f[0x0A] = Chip8::op_fx0a;
        chip8.table_f[0x15] = Chip8::op_fx15;
        chip8.table_f[0x18] = Chip8::op_fx18;
        chip8.table_f[0x1E] = Chip8::op_fx1e;
        chip8.table_f[0x29] = Chip8::op_fx29;
        chip8.table_f[0x33] = Chip8::op_fx33;
        chip8.table_f[0x55] = Chip8::op_fx55;
        chip8.table_f[0x65] = Chip8::op_fx65;

        chip8
    }

    // Function pointer helper functions
    fn table_0(&mut self) {
        self.table_0[(self.opcode & 0x000F) as usize](self);
    }

    fn table_8(&mut self) {
        self.table_8[(self.opcode & 0x000F) as usize](self);
    }
    
    fn table_e(&mut self) {
        self.table_e[(self.opcode & 0x000F) as usize](self);
    }
    
    fn table_f(&mut self) {
        self.table_f[(self.opcode & 0x000F) as usize](self);
    }

    fn op_null(&mut self) {
        return;
    }

    fn op_00e0(&mut self) {
        // Clear the video array by setting all elements to zeroi
        self.video = [0; 64 * 32];
    }

    fn op_00ee(&mut self) {
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize];
    }

    fn op_1nnn(&mut self) {
        let address: u16 = self.opcode & 0x0FFF;
        self.pc = address;
    }

    fn op_2nnn(&mut self) {
        let address: u16 = self.opcode & 0x0FFF;
        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;
        self.pc = address;
    }

    fn op_3xkk(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        if self.registers[vx as usize] == byte {
            self.pc += 2;
        }
    }

    fn op_4xkk(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        if self.registers[vx as usize] == byte {
            self.pc += 2;
        }
    }

    fn op_5xy0(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        if self.registers[vx as usize] == byte {
            self.pc += 2;
        }
    }

    fn op_6xkk(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        self.registers[vx as usize] = byte;
    } 

    fn op_7xkk(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        self.registers[vx as usize] += byte;
    } 

    fn op_8xy0(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let vy: u16 = (self.opcode & 0x00F0) >> 4;

        self.registers[vx as usize] = self.registers[vy as usize];
    }

    fn op_8xy1(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let vy: u16 = (self.opcode & 0x00F0) >> 4;

        self.registers[vx as usize] |= self.registers[vy as usize];
    }

    fn op_8xy2(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let vy: u16 = (self.opcode & 0x00F0) >> 4;

        self.registers[vx as usize] &= self.registers[vy as usize];
    }

    fn op_8xy3(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let vy: u16 = (self.opcode & 0x00F0) >> 4;

        self.registers[vx as usize] ^= self.registers[vy as usize];
    }

    fn op_8xy4(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let vy: u16 = (self.opcode & 0x00F0) >> 4;

        let sum: u16 = (self.registers[vx as usize] 
                + self.registers[vy as usize]) as u16;

        if sum > 255 {
            self.registers[0xF] = 1;
        }
        else {
            self.registers[0xF] = 0;
        }

        self.registers[vx as usize] = (sum & 0xFF) as u8;
    }

    fn op_8xy5(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let vy: u16 = (self.opcode & 0x00F0) >> 4;

        if self.registers[vx as usize] > self.registers[vy as usize] {
            self.registers[0xF] = 1;
        }
        else {
            self.registers[0xF] = 0;
        }

        self.registers[vx as usize] -= self.registers[vy as usize];
    }

    fn op_8xy6(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;

        // Save LSB in VF
        self.registers[0xF] = self.registers[vx as usize] & 0x1;

        self.registers[vx as usize] >>= 1;
    }

    fn op_8xy7(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let vy: u16 = (self.opcode & 0x00F0) >> 4;

        if self.registers[vy as usize] > self.registers[vx as usize] {
            self.registers[0xF] = 1;
        }
        else {
            self.registers[0xF] = 0;
        }

        self.registers[vx as usize] = 
            self.registers[vy as usize] - self.registers[vx as usize];
    }

    fn op_8xye(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;

        // Save MSB in VF
        self.registers[0xF] = (self.registers[vx as usize] & 0x80) >> 7;

        // Shift the register value to the left by 1
        self.registers[vx as usize] <<= 1;
    }

    fn op_9xy0(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let vy: u16 = (self.opcode & 0x00F0) >> 4;

        if self.registers[vx as usize] != self.registers[vy as usize] {
            self.pc += 2;
        }
    }

    fn op_annn(&mut self) {
        let address: u16 = self.opcode & 0x0FFF;
        self.pc = self.registers[0] as u16 + address;
    }

    fn op_bnnn(&mut self) {
        let address: u16 = self.opcode & 0x0FFF;
        self.pc = self.registers[0] as u16 + address;
    }

    fn op_cxkk(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let byte: u8 = (self.opcode & 0x00FF) as u8;

        self.registers[vx as usize] = self.rng_core.gen::<u8>() & byte;
    }

    fn op_dxyn(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let vy: u16 = (self.opcode & 0x00F0) >> 4;
        let height: u8 = (self.opcode & 0x000F) as u8;

        // Wrap if going beyond screen boundaries
        let x_pos: u8 = self.registers[vx as usize] % VIDEO_WIDTH as u8;
        let y_pos: u8 = self.registers[vy as usize] % VIDEO_HEIGHT as u8;

        self.registers[0xF] = 0;

        for row in 0..height {
            let sprite_byte: u8 = 
                self.memory[(self.index + row as u16) as usize];
            
            for col in 0..8 {
                let sprite_pixel: u8 = sprite_byte & (0x80 >> col);
                let screen_pixel_index: usize
                    = ((y_pos + row) * VIDEO_WIDTH as u8 + (x_pos + col)) as usize;
                let screen_pixel 
                    = &mut self.video[screen_pixel_index];

                // Sprite pixel is on
                if sprite_pixel != 0 {
                    // #Collision: screen pixel also on
                    if *screen_pixel == 0xFF {
                        self.registers[0xF] = 1;
                    }

                    // Effectively XOR with the sprite pixel
                    *screen_pixel ^= 0xFF;
                }
            }
        }
    }

    fn op_ex9e(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let key: u8 = self.registers[vx as usize];

        if self.keypad[key as usize] > 0 {
            self.pc += 2;
        }
    }

    fn op_exa1(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let key: u8 = self.registers[vx as usize];

        if self.keypad[key as usize] == 0 {
            self.pc += 2;
        }
    }

    fn op_fx07(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        self.registers[vx as usize] = self.delay_timer;
    }

    fn op_fx0a(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        
        for i in 0..16 {
            if self.keypad[i] > 0 {
                self.registers[vx as usize] = i as u8;
                return;
            }
        }

        // Else
        self.pc -= 2;
    }

    fn op_fx15(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        self.delay_timer = self.registers[vx as usize];
    }

    fn op_fx18(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        self.sound_timer = self.registers[vx as usize];
    }

    fn op_fx1e(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        self.index += (self.registers[vx as usize]) as u16;
    }

    fn op_fx29(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let digit: u8 = self.registers[vx as usize];

        self.index = (FONTSET_START_ADDRESS + (5 * digit) as usize) as u16;
    }

    fn op_fx33(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        let mut value: u8 = self.registers[vx as usize];

        // Ones-place
        self.memory[(self.index + 2) as usize] = value % 10;
        value /= 10;

        // Tens-place
        self.memory[(self.index + 1) as usize] = value % 10;
        value /= 10;

        // Hundreds-place
        self.memory[self.index as usize] = value % 10;
    }

    fn op_fx55(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;

        for i in 0..=vx {
            self.memory[(self.index + i) as usize] = self.registers[i as usize];
        }
    }

    fn op_fx65(&mut self) {
        let vx: u16 = (self.opcode & 0x0F00) >> 8;
        
        for i in 0..=vx {
            self.registers[i as usize] = self.memory[(self.index + i) as usize];
        }
    }

    // Cycle
    pub fn cycle(&mut self) {
        // Fetch next instruction
        self.opcode = ((self.memory[self.pc as usize] as u16) << 8) 
            | self.memory[(self.pc + 1) as usize] as u16;
        
        // Increment pc before execution
        self.pc += 2;

        // Get the instruction and execute
        self.table[((self.opcode & 0xF000) >> 12) as usize](self);

        // Decrement the delay timer if set
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        // Decrement the sound timer if set
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }
}