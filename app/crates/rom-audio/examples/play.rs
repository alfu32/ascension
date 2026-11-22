//! Hear one sound: `cargo run -p rom-audio --example play -- <sound> [gain]`.
//! Without arguments it lists the sound names. Loops (`ambient.`, `.moving`, `.active`) play for
//! four seconds; one-shots play to their end.

use std::time::{Duration, Instant};

use rom_audio::{AudioEngine, EngineStatus, Placement, SoundId, decode};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(name) = args.next() else {
        for id in SoundId::all() {
            println!("{}", id.name());
        }
        return;
    };
    let gain: f32 = args.next().and_then(|g| g.parse().ok()).unwrap_or(0.3);
    let Some(sound) = SoundId::from_name(&name) else {
        eprintln!("no sound named {name}; run without arguments for the list");
        std::process::exit(2);
    };
    let pcm = decode(sound.name(), sound.bytes()).expect("decode");
    println!(
        "{name}: {:.2} s, {} channel(s), {} Hz, {} bytes embedded",
        pcm.seconds(),
        pcm.channels,
        pcm.rate,
        sound.bytes().len()
    );

    let engine = AudioEngine::new();
    let start = Instant::now();
    while engine.status() == EngineStatus::Starting && start.elapsed() < Duration::from_secs(30) {
        std::thread::sleep(Duration::from_millis(20));
    }
    println!(
        "engine: {:?} after {:.2} s",
        engine.status(),
        start.elapsed().as_secs_f32()
    );
    let looped =
        name.starts_with("ambient.") || name.ends_with(".moving") || name.contains(".active");
    let seconds = if looped {
        engine.set_loop(0, Some(sound), gain);
        4.0
    } else {
        engine.play(sound, Placement::centre(gain), 100);
        pcm.seconds() + 0.3
    };
    std::thread::sleep(Duration::from_secs_f32(seconds));
    if looped {
        engine.set_loop(0, None, 0.0);
        std::thread::sleep(Duration::from_millis(1100));
    }
    println!("frames sent to the device: {}", engine.frames_played());
}
