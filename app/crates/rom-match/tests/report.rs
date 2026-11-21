//! Task 117: the match report of a short match with one person and two computer players.

use rom_match::{Aspect, ContentRequest, MapKind, MatchReport, MatchSettings, PLAYER_ONE};
use rom_save::MatchInfo;
use rom_sim::PlayerId;

#[test]
fn the_report_ranks_every_player_in_every_aspect_with_its_history() {
    let settings = MatchSettings {
        content: ContentRequest {
            seed: 4,
            players: 3,
            ..ContentRequest::default()
        },
        map: MapKind::Flat,
        ai: Some(Vec::new()),
        humans: vec![PLAYER_ONE],
        ..MatchSettings::default()
    };
    let mut core = settings.build().expect("match").leader();
    for _ in 0..1_300 {
        core.lead(Vec::new());
    }
    let world = core.world();
    assert_eq!(world.humans().collect::<Vec<_>>(), [PLAYER_ONE]);
    assert_eq!(world.match_end(), None);
    let report = MatchReport::of(world, &MatchInfo::default());
    assert_eq!(report.players.len(), 3);
    assert_eq!(report.tick, 1_300);
    for p in &report.players {
        assert_eq!(p.human, p.id == PLAYER_ONE);
        assert_eq!(p.ranks.len(), Aspect::ALL.len());
        assert!(p.ranks.iter().all(|r| (1..=3).contains(r)));
        assert_eq!(p.name, format!("Player {}", p.id.0));
    }
    // Every aspect has a leader; the overall leader comes first.
    for index in 0..Aspect::ALL.len() {
        assert!(report.players.iter().any(|p| p.ranks[index] == 1));
    }
    assert_eq!(report.players[0].ranks[0], 1);
    // The computers trained, built and gathered in their first game minute.
    let computer = report.player(PlayerId(2)).unwrap();
    assert!(
        computer.gathered > 0 && computer.units_trained > 0,
        "{computer:?}"
    );
    // One sample per player at 1 200 ticks in each chart.
    assert_eq!(report.charts.len(), 4);
    for chart in &report.charts {
        assert_eq!(chart.series.len(), 3);
        for series in &chart.series {
            assert_eq!(
                series.points.iter().map(|p| p.0).collect::<Vec<_>>(),
                [1_200]
            );
        }
    }
    let text = report.to_text();
    assert!(
        text.contains("Overall") && text.contains("Player 2"),
        "{text}"
    );
    assert!(text.contains("end: not ended"), "{text}");
}
