struct Chip8 {
    registers: [u8; 16],
    memory: [u8; 4096],
    index: u16,
    pc: u16,
    stack: [u16; 16],
    sp: u8,
    delay_timer: u8,
    sound_timer: u8,
    keypad: [u8; 16],
    video: [u32; 64 * 32],
    opcode: u16,
    rand_byte: rand::distributions::Uniform<u8>,
    rng_core: rand::rngs::ThreadRng,
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

impl Chip8 {
    fn load_rom(&mut self, filename: &str) -> Result<(), std::io::Error> {
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
    fn new(&mut self) -> Chip8 {
        let mut memory = [0; 4096];
        
        // Load fonts into memory
        for i in 0..FONTSET_SIZE {
            memory[FONTSET_START_ADDRESS + i] = FONTSET[i];
        }

        // Initialize RNG
        let rand_byte = rand::distributions::Uniform::new(0, 255);
        let rng_core = rand::thread_rng();

        Chip8 {
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
        }
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
        let x_pos: u8 = self.registers[vx as usize] % VIDEO_WIDTH;
        let y_pos: u8 = self.registers[vy as usize] % VIDEO_HEIGHT;

        self.registers[0xF] = 0;

        for row in 0..height {
            let sprite_byte: u8 = 
                self.memory[(self.index + row as u16) as usize];
            
            for col in 0..8 {
                let sprite_pixel: u8 = sprite_byte & (0x80 >> col);
                let screen_pixel_index: usize
                    = ((y_pos + row) * VIDEO_WIDTH + (x_pos + col)) as usize;
                let screen_pixel 
                    = &mut self.video[screen_pixel_index];

                // Sprite pixel is on
                if sprite_pixel != 0 {
                    // #Collision: screen pixel also on
                    if *screen_pixel == 0xFFFFFFFF {
                        self.registers[0xF] = 1;
                    }

                    // Effectively XOR with the sprite pixel
                    *screen_pixel ^= 0xFFFFFFFF;
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

    fn op_fx1E(&mut self) {
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

}