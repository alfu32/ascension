//! The embedded sound set: every file in app/assets/sounds is in the table and decodes.

use rom_audio::{
    AudioEngine, Category, EngineStatus, Kind, MusicId, MusicPlay, Placement, SoundBank, SoundId,
    Volumes,
};

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
        let expected = if id.kind() == Kind::Ambient { 2 } else { 1 };
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
            pcm.rate == 44_100 || (id.kind() == Kind::Voice && pcm.rate == 22_050),
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
    engine.set_volumes(Volumes::default());
    engine.set_volumes(Volumes {
        master: f32::NAN,
        music: -1.0,
        units: 2.0,
        ..Volumes::FULL
    });
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
    engine.play_as(Category::Buildings, first, Placement::centre(1.0), 3);
    engine.set_loop_as(
        Category::Environment,
        1,
        Some(first),
        Placement::centre(0.2),
    );
    engine.stop_all();
    assert_eq!(engine.frames_played(), 0);
}

#[test]
fn silent_engine_accepts_every_music_call() {
    let engine = AudioEngine::silent();
    for id in MusicId::all() {
        engine.play_music(MusicPlay::whole(id, 0.5));
    }
    engine.play_music(MusicPlay {
        start: 1e9,
        end: Some(-4.0),
        fade_in: f32::NAN,
        fade_out: -1.0,
        gain: f32::INFINITY,
        ..MusicPlay::whole(MusicId::all().next().unwrap(), 1.0)
    });
    engine.stop_music(0.0);
    engine.stop_music(f32::NAN);
    engine.duck(0.7);
    engine.duck(f32::NAN);
    assert_eq!(engine.music_now(), None);
    assert_eq!(engine.frames_played(), 0);
}

#[test]
fn every_sound_has_a_category_by_its_name() {
    for id in SoundId::all() {
        let name = id.name();
        let expected = match name.split('.').next() {
            Some("bld") => Category::Buildings,
            Some("ambient") => Category::Environment,
            Some("ui") => Category::Announcements,
            Some("voice" | "unit" | "act") => Category::Units,
            other => panic!("{name}: no category for {other:?}"),
        };
        assert_eq!(id.category(), expected, "{name}");
    }
}

#[test]
fn table_holds_every_piece_of_music() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/music");
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .expect("music folder")
        .filter_map(|e| {
            let p = e.ok()?.path();
            (p.extension()? == "ogg").then(|| p.file_stem()?.to_str().map(str::to_owned))?
        })
        .collect();
    files.sort();
    let table: Vec<&str> = MusicId::all().map(MusicId::name).collect();
    assert_eq!(table, files);
}
