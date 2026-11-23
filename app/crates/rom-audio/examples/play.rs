//! Hear one sound: `cargo run -p rom-audio --example play -- <sound> [gain]`.
//! Without arguments it lists the sound names. Loops (`ambient.`, `.moving`, `.active`) play for
//! four seconds; one-shots play to their end. A piece of music (`title`, `ancient.theme`, task
//! 133) plays from `[start]` seconds for up to 20 seconds, streamed as in the game.

use std::time::{Duration, Instant};

use rom_audio::{AudioEngine, EngineStatus, MusicId, MusicPlay, Placement, SoundId, decode};

fn music(track: MusicId, start: f32) {
    let engine = AudioEngine::new();
    let began = Instant::now();
    while engine.status() == EngineStatus::Starting && began.elapsed() < Duration::from_secs(30) {
        std::thread::sleep(Duration::from_millis(20));
    }
    println!("engine: {:?}", engine.status());
    let seconds = (track.seconds() - start).clamp(0.0, 20.0);
    engine.play_music(MusicPlay {
        start,
        end: Some(start + seconds),
        fade_in: 0.5,
        fade_out: 1.0,
        ..MusicPlay::whole(track, 1.0)
    });
    for _ in 0..(seconds * 2.0) as usize {
        std::thread::sleep(Duration::from_millis(500));
        if let Some(now) = engine.music_now() {
            println!("{} at {:.1} s", now.track.name(), now.seconds);
        }
    }
    std::thread::sleep(Duration::from_millis(600));
    println!("frames sent to the device: {}", engine.frames_played());
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(name) = args.next() else {
        for id in SoundId::all() {
            println!("{}", id.name());
        }
        for id in MusicId::all() {
            println!("{} (music, {:.0} s)", id.name(), id.seconds());
        }
        return;
    };
    let second: Option<f32> = args.next().and_then(|g| g.parse().ok());
    if let Some(track) = MusicId::from_name(&name) {
        // For music the second argument is where to start, in seconds.
        music(track, second.unwrap_or(0.0));
        return;
    }
    let gain = second.unwrap_or(0.3);
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
