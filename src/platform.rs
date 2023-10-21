use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Canvas, Texture, TextureCreator, WindowCanvas};
use sdl2::sys::{KeyCode, SDL_KeyCode};
use sdl2::video::{Window, WindowContext};
use sdl2::EventPump;
use sdl2::VideoSubsystem;

pub struct Platform<'a> {
    canvas: WindowCanvas,
    texture: Texture<'a>,
    event_pump: EventPump,
}

impl<'tex> Platform<'tex> {
    pub fn new(
        video_subsystem: &VideoSubsystem,
        title: &str,
        window_size: (u32, u32),
        texture_creator: &'tex mut TextureCreator<WindowContext>,
        texture_size: (u32, u32),
        event_pump: EventPump,
    ) -> Self {
        let window: Window = video_subsystem
            .window(title, window_size.0, window_size.1)
            .position_centered()
            .build()
            .unwrap();

        let canvas = window.into_canvas().accelerated().build().unwrap();

        let texture: Texture<'_> = texture_creator
            .create_texture_streaming(PixelFormatEnum::RGBA8888, texture_size.0, texture_size.1)
            .unwrap();

        Platform {
            canvas,
            texture,
            event_pump,
        }
    }

    pub fn update(&mut self, pixel_data: &[u8], pitch: usize) {
        self.texture.update(None, pixel_data, pitch).unwrap();
        self.canvas.clear();
        self.canvas.copy(&self.texture, None, None).unwrap();
        self.canvas.present();
    }

    fn key_to_chip8_key(key: Keycode) -> Option<usize> {
        match key {
            Keycode::X => Some(0),
            Keycode::Num1 => Some(1),
            Keycode::Num2 => Some(2),
            Keycode::Num3 => Some(3),
            Keycode::Q => Some(4),
            Keycode::W => Some(5),
            Keycode::E => Some(6),
            Keycode::A => Some(7),
            Keycode::S => Some(8),
            Keycode::D => Some(9),
            Keycode::Z => Some(0xA),
            Keycode::C => Some(0xB),
            Keycode::Num4 => Some(0xC),
            Keycode::R => Some(0xD),
            Keycode::F => Some(0xE),
            Keycode::V => Some(0xF),
            _ => None,
        }
    }

    pub fn process_input(&mut self, keys: &mut [u8; 16]) -> bool {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    return true;
                }

                Event::KeyDown {
                    keycode,
                    ..
                } => {
                    if let Some(key) = keycode {
                        if let Some(index) = Self::key_to_chip8_key(key) {
                            keys[index] = 1;
                        }
                        if key == Keycode::Escape {
                            return true;
                        }
                    }
                }

                Event::KeyUp {
                    keycode,
                    ..
                } => {
                    if let Some(key) = keycode {
                        if let Some(index) = Self::key_to_chip8_key(key) {
                            keys[index] = 0;
                        }
                        if key == Keycode::Escape {
                            return true;
                        }
                    }
                }

                _ => {}
            }
        }
        false
    }


}
