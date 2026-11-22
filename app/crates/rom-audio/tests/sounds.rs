//! The embedded sound set: every file in app/assets/sounds is in the table and decodes.

use rom_audio::{AudioEngine, Bus, EngineStatus, Placement, SoundBank, SoundId};

#[test]
fn table_holds_every_file() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/sounds");
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .expect("sounds folder")
        .filter_map(|e| {
            let p = e.ok()?.path();
            (p.extension()? == "ogg").then(|| p.file_stem()?.to_str().map(str::to_owned))?
        })
        .collect();
    files.sort();
    let table: Vec<&str> = SoundId::all().map(SoundId::name).collect();
    assert_eq!(table, files);
    assert!(!table.is_empty());
}

#[test]
fn every_sound_decodes() {
    let bank = SoundBank::decode_all(4).expect("every sound decodes");
    assert_eq!(bank.len(), SoundId::count());
    for id in SoundId::all() {
        let pcm = bank.pcm(id).expect("decoded");
        assert!(pcm.frames() > 100, "{} is too short", id.name());
        let expected = if id.bus() == Bus::Ambient { 2 } else { 1 };
        assert_eq!(pcm.channels, expected, "{} channels", id.name());
        let name = id.name();
        let looped = name.starts_with("ambient.")
            || name.ends_with(".moving")
            || name.ends_with(".active")
            || name.starts_with("act.footsteps_grass")
            || name == "act.saw"
            || name == "act.rotor";
        if looped {
            // Loops carry loop points inside padding of their own wrap-around.
            assert!(
                pcm.loop_start > 0 && pcm.loop_start + pcm.loop_frames < pcm.frames(),
                "{name} has no loop points"
            );
        } else {
            assert_eq!(
                (pcm.loop_start, pcm.loop_frames),
                (0, pcm.frames()),
                "{name}"
            );
        }
        assert!(
            pcm.rate == 44_100 || (id.bus() == Bus::Voice && pcm.rate == 22_050),
            "{} at {} Hz",
            id.name(),
            pcm.rate
        );
    }
}

#[test]
fn silent_engine_accepts_every_call() {
    let engine = AudioEngine::silent();
    assert!(matches!(engine.status(), EngineStatus::Silent { .. }));
    let first = SoundId::all().next().unwrap();
    engine.set_volumes(0.5, 1.0, 1.0, 1.0);
    engine.set_volumes(f32::NAN, -1.0, 2.0, 0.0);
    engine.play(first, Placement::centre(1.0), 0);
    engine.play(
        first,
        Placement {
            gain: 1.0,
            pan: 9.0,
        },
        255,
    );
    engine.set_loop(0, Some(first), 0.5);
    engine.set_loop(99, Some(first), 0.5);
    engine.set_loop(0, None, 0.0);
    engine.stop_all();
    assert_eq!(engine.frames_played(), 0);
}
