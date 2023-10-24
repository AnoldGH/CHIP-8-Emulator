mod chip8;
mod platform;

use std::env;
use std::process;
use std::time::{Duration, Instant};

use chip8::Chip8;
use sdl2::VideoSubsystem;
use sdl2::render::Canvas;
use sdl2::video;

// Static variables
static TITLE: &str = "CHIP-8 Emulator";

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: {} <Scale> <Delay> <ROM>", args[0]);
        process::exit(1);
    }

    let video_scale: u8
        = args[1].parse().expect("Failed to parse Scale");
    let cycle_delay: u64
        = args[2].parse().expect("Failed to parse Delay");
    let rom_filename = &args[3];

    /* Build sdl context */
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let event_pump = sdl_context.event_pump().unwrap();

    let window_width: u32 = chip8::VIDEO_WIDTH as u32 * video_scale as u32;
    let window_height: u32 = chip8::VIDEO_HEIGHT as u32 * video_scale as u32;
    
    let window = video_subsystem.window
        ("Chip-8 Emulator", window_width, window_height)
            .position_centered()
            .build()
            .unwrap();
    let canvas 
        = window.into_canvas().accelerated().build().unwrap();
    let mut texture_creator 
        = canvas.texture_creator();

    let mut platform = platform::Platform::new(&video_subsystem, TITLE, (window_width, window_height), canvas, &mut texture_creator, (chip8::VIDEO_WIDTH as u32, chip8::VIDEO_HEIGHT as u32), event_pump);

    let mut chip8 = chip8::Chip8::new();
    chip8.load_rom(rom_filename);

    eprintln!("Finished reading in ROM.");

    let video_pitch 
        = std::mem::size_of_val(&chip8.video[0]) * chip8::VIDEO_WIDTH as usize;

    let mut last_cycle_time = Instant::now();
    let mut quit = false;

    eprintln!("Started drawing graphics.");

    let mut cycle_counter: usize = 1;
    while !quit {
        // TODO: debug

        quit = platform.process_input(&mut chip8.keypad);
        eprintln!("Finished processing input.");

        let current_time = Instant::now();
        let dt = current_time.duration_since(last_cycle_time);

        if dt > Duration::from_millis(cycle_delay) {
            last_cycle_time = current_time;
            chip8.cycle();
            eprintln!("Cycle {} completed.", cycle_counter);
            
            // eprintln!("---DEBUG--- Pixel data {}", chip8.video.len());
            // for i in 0..chip8.video.len() {
            //     eprint!("{} ", chip8.video[i]);
            // }
            // eprintln!("---DEBUG---");

            platform.update(&mut chip8.video, video_pitch);
            


            eprintln!("Platform updated.");

            cycle_counter += 1;
        }

        // TODO: debug
    }
}