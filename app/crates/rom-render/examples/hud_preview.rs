//! Writes a CPU-only preview of the complete match HUD.
use rom_render::hud::HudPainter;
use rom_render::hud_layout::HudLayout;
use rom_render::{
    ButtonState, Camera, HudButton, HudPalette, HudResearch, HudResource, HudSnapshot, PaletteTab,
    ResearchCell, ResearchOverview, ResearchTrack, TerrainScene, TileKind,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: hud_preview <out.png> <width> <height>".into());
    }
    let width: u32 = args[1].parse()?;
    let height: u32 = args[2].parse()?;
    if width == 0 || height == 0 || width > 16384 || height > 16384 {
        return Err("dimensions must be 1..16384".into());
    }
    let layout = HudLayout::new(width, height, None);
    let mut painter = HudPainter::bake(layout.scale)?;
    let snapshot = HudSnapshot {
        game_ticks: 754 * 20,
        commerce: (320, 500),
        population: (1240, 1500),
        cities: (5, 8),
        resources: [
            ("food", 4280, 20),
            ("timber", 2160, 30),
            ("metal", 1340, 42),
            ("wealth", 3050, 88),
            ("knowledge", 1720, 76),
            ("oil", 640, 36),
            ("energy", 960, 48),
        ]
        .into_iter()
        .map(|(label, amount, rate)| HudResource {
            icon: format!("icons/res_{label}"),
            label: label.into(),
            amount,
            rate,
            capped: false,
            visible: true,
        })
        .collect(),
        research: [("Agriculture", 72), ("Metallurgy", 43), ("Engineering", 18)]
            .into_iter()
            .map(|(label, progress_pct)| HudResearch {
                label: label.into(),
                progress_pct,
            })
            .collect(),
        events: vec!["CPU sample: READY / UNAFFORDABLE / LOCKED / DONE / IN PROGRESS".into()],
        help: "Q Ready   W Unaffordable   E Locked   R Done   T In progress".into(),
        data_tab: 4,
        palette: Some(HudPalette {
            tab: PaletteTab::Research,
            tabs: [true; 4],
            pages: 1,
            buttons: [
                ("Ready", ButtonState::Ready),
                (
                    "Unaffordable",
                    ButtonState::Unaffordable {
                        missing: vec![("icons/res_timber".into(), 160)],
                    },
                ),
                (
                    "Locked",
                    ButtonState::Locked {
                        reason: "Needs Classical Age".into(),
                    },
                ),
                ("Done", ButtonState::Done),
                ("In progress", ButtonState::InProgress { pct: 43 }),
            ]
            .into_iter()
            .enumerate()
            .map(|(slot, (name, state))| HudButton {
                slot,
                icon: "icons/pic_scroll".into(),
                name: name.into(),
                price: "200 Food, 200 Timber".into(),
                state,
                armed: false,
            })
            .collect(),
            ..HudPalette::default()
        }),
        overview: ResearchOverview {
            tracks: ["CIVIC", "COMMERCE", "MILITARY", "SCIENCE"]
                .into_iter()
                .enumerate()
                .map(|(row, label)| ResearchTrack {
                    label: label.into(),
                    levels: (0..13)
                        .map(|col| {
                            if col < row + 1 {
                                ResearchCell::Done
                            } else if col == row + 1 && row == 2 {
                                ResearchCell::InProgress
                            } else if col == row + 1 {
                                ResearchCell::Next
                            } else {
                                ResearchCell::Locked
                            }
                        })
                        .collect(),
                })
                .collect(),
            ages: (0..14)
                .map(|i| match i {
                    0 => ResearchCell::Done,
                    1 => ResearchCell::InProgress,
                    2 => ResearchCell::Next,
                    _ => ResearchCell::Locked,
                })
                .collect(),
        },
        ..HudSnapshot::default()
    };
    let terrain = TerrainScene {
        width: 96,
        height: 96,
        heights: (0..9216)
            .map(|i| ((i % 96 + i / 96) * 255 / 190) as u8)
            .collect(),
        kinds: (0..9216)
            .map(|i| {
                let (x, y) = (i % 96, i / 96);
                if x < 14 || y > 80 {
                    TileKind::DeepSea
                } else if x < 20 || y > 74 {
                    TileKind::ShallowSea
                } else {
                    TileKind::Land
                }
            })
            .collect(),
        feature_colors: Default::default(),
        starts: vec![],
        nodes: vec![],
    };
    let entities = (0..16)
        .map(|i| rom_render::EntityView {
            id: i + 1,
            kind: 1,
            owner: if i < 8 { 1 } else { 2 },
            face: 0,
            x: 32.0 + (i % 4) as f32 * 3.0,
            y: 30.0 + (i / 4) as f32 * 9.0,
            facing_deg: 0.0,
            hp_frac: 1.0,
            building: i % 8 == 0,
            footprint: (1, 1),
            radius: 0.3,
            carrying: false,
            kept: 0,
            capacity: 0,
            carry_node: false,
            under_construction: None,
            model: None,
            activity: Default::default(),
        })
        .collect::<Vec<_>>();
    let camera = Camera {
        target: [40.0, 0.0, 45.0],
        distance: 45.0,
        ..Camera::default()
    };
    rom_render::hud_preview::render(
        &mut painter,
        &layout,
        &snapshot,
        &terrain,
        &entities,
        &camera,
    )
    .save(&args[0])?;
    println!("hud-preview {} {width}x{height}", args[0]);
    Ok(())
}
