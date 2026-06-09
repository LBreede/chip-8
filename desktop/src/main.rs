use chip8_core::{Emu, SCREEN_HEIGHT, SCREEN_WIDTH};
use sdl2::EventPump;
use sdl2::Sdl;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::env;
use std::fs::File;
use std::io::Read;
use std::time::{Duration, Instant};

const SCALE: u32 = 30;
const WINDOW_WIDTH: u32 = (SCREEN_WIDTH as u32) * SCALE;
const WINDOW_HEIGHT: u32 = (SCREEN_HEIGHT as u32) * SCALE;

const CPU_HZ: f64 = 700.0;
const TIMER_HZ: f64 = 60.0;

enum LoopControl {
    Continue,
    Quit,
}

fn cpu_step() -> Duration {
    Duration::from_nanos((1_000_000_000.0 / CPU_HZ) as u64)
}

fn timer_step() -> Duration {
    Duration::from_nanos((1_000_000_000.0 / TIMER_HZ) as u64)
}

fn draw_screen(emu: &Emu, canvas: &mut Canvas<Window>) -> Result<(), String> {
    canvas.set_draw_color(Color::BLACK);
    canvas.clear();
    let screen_buf = emu.get_display();
    canvas.set_draw_color(Color::GREEN);
    for (i, pixel) in screen_buf.iter().enumerate() {
        if *pixel {
            let x = (i % SCREEN_WIDTH) as u32;
            let y = (i / SCREEN_WIDTH) as u32;
            let rect = Rect::new((x * SCALE) as i32, (y * SCALE) as i32, SCALE, SCALE);
            canvas.fill_rect(rect)?;
        }
    }
    canvas.present();
    Ok(())
}

fn key2btn(key: Keycode) -> Option<usize> {
    match key {
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Num4 => Some(0xC),
        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::R => Some(0xD),
        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::F => Some(0xE),
        Keycode::Z => Some(0xA),
        Keycode::X => Some(0x0),
        Keycode::C => Some(0xB),
        Keycode::V => Some(0xF),
        _ => None,
    }
}

fn read_rom(path: &str) -> Result<Vec<u8>, String> {
    let mut rom = File::open(path).map_err(|err| format!("Unable to open file: {err}"))?;
    let mut buffer = Vec::new();
    rom.read_to_end(&mut buffer)
        .map_err(|err| format!("Unable to read file: {err}"))?;
    Ok(buffer)
}

fn create_canvas(sdl_context: &Sdl) -> Result<Canvas<Window>, String> {
    let video = sdl_context.video()?;

    let window = video
        .window("Chip-8 Emulator", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .map_err(|err| err.to_string())?;

    window
        .into_canvas()
        .present_vsync()
        .build()
        .map_err(|err| err.to_string())
}

fn handle_event(event: Event, chip8: &mut Emu) -> Result<LoopControl, String> {
    match event {
        Event::Quit { .. } => Ok(LoopControl::Quit),
        Event::KeyDown {
            keycode: Some(key),
            repeat,
            ..
        } => {
            if !repeat && let Some(k) = key2btn(key) {
                chip8.keypress(k, true).map_err(|err| err.to_string())?;
            }
            Ok(LoopControl::Continue)
        }
        Event::KeyUp {
            keycode: Some(key), ..
        } => {
            if let Some(k) = key2btn(key) {
                chip8.keypress(k, false).map_err(|err| err.to_string())?;
            }
            Ok(LoopControl::Continue)
        }
        _ => Ok(LoopControl::Continue),
    }
}

fn run(
    chip8: &mut Emu,
    event_pump: &mut EventPump,
    canvas: &mut Canvas<Window>,
) -> Result<(), String> {
    let mut last = Instant::now();
    let mut cpu_acc = Duration::ZERO;
    let mut timer_acc = Duration::ZERO;

    let cpu_step = cpu_step();
    let timer_step = timer_step();

    loop {
        let now = Instant::now();
        let dt = now - last;
        last = now;

        cpu_acc += dt;
        timer_acc += dt;

        for event in event_pump.poll_iter() {
            match handle_event(event, chip8)? {
                LoopControl::Continue => (),
                LoopControl::Quit => return Ok(()),
            }
        }

        while cpu_acc >= cpu_step {
            chip8.tick().map_err(|err| err.to_string())?;
            cpu_acc -= cpu_step;
        }
        while timer_acc >= timer_step {
            chip8.tick_timers();
            timer_acc -= timer_step;
        }
        draw_screen(chip8, canvas)?;
    }
}

fn main() -> Result<(), String> {
    let path = match env::args().nth(1) {
        Some(path) => path,
        None => {
            println!("Usage: cargo run path/to/game");
            return Ok(());
        }
    };

    let sdl_context = sdl2::init()?;
    let mut canvas = create_canvas(&sdl_context)?;
    let mut event_pump = sdl_context.event_pump()?;

    let mut chip8 = Emu::new();
    let rom = read_rom(&path)?;
    chip8.load(&rom).map_err(|err| err.to_string())?;

    run(&mut chip8, &mut event_pump, &mut canvas)
}
