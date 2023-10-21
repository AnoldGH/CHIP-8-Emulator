mod chip8;
mod platform;

use std::env;
use std::process;
use std::time::{Duration, Instant};

use sdl2::VideoSubsystem;
use sdl2::render::Canvas;
use sdl2::video;

// Static variables
static TITLE: &str = "CHIP-8 Emulator";
static WINDOW_WIDTH: u32 = 640;
static WINDOW_HEIGHT: u32 = 640;
static TEXTURE_WIDTH: u32 = 640;
static TEXTURE_HEIGHT: u32 = 640;


fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: {} <Scale> <Delay> <ROM>", args[0]);
        process::exit(1);
    }

    let video_scale: usize
        = args[1].parse().expect("Failed to parse Scale");
    let cycle_delay: u64
        = args[2].parse().expect("Failed to parse Delay");
    let rom_filename = &args[3];

    /* Build sdl context */
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let event_pump = sdl_context.event_pump().unwrap();
    
    let window = video_subsystem.window
        ("Chip-8 Emulator", WINDOW_WIDTH, WINDOW_HEIGHT)
            .position_centered()
            .build()
            .unwrap();
    let canvas 
        = window.into_canvas().accelerated().build().unwrap();
    let mut texture_creator 
        = canvas.texture_creator();

    let platform = platform::Platform::new(&video_subsystem, TITLE, (WINDOW_WIDTH, WINDOW_HEIGHT), &mut texture_creator, (TEXTURE_WIDTH, TEXTURE_HEIGHT), event_pump);

    let mut chip8 = chip8::Chip8::new();
    chip8.load_rom(rom_filename);

    let video_pitch 
        = std::mem::size_of_val(&chip8.video[0]) * WINDOW_WIDTH as usize;

    let mut last_cycle_time = Instant::now();
    let mut quit = false;

    while !quit {
        quit = platform.process_input(&mut chip8.keypad);

        let current_time = Instant::now();
        let dt = current_time.duration_since(last_cycle_time);

        if dt > Duration::from_millis(cycle_delay) {
            last_cycle_time = current_time;
            chip8.cycle();
            platform.update(&chip8.video, video_pitch)
        }
    }
}