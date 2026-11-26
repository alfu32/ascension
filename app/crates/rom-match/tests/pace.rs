//! Task 159: the economy pace travels in the match settings to the world, to a follower and to
//! the report.

use rom_match::{ContentRequest, MapKind, MatchReport, MatchSettings, PLAYER_ONE};
use rom_save::MatchInfo;
use rom_sim::EconomyPace;

fn settings(pace: EconomyPace) -> MatchSettings {
    MatchSettings {
        content: ContentRequest {
            seed: 5,
            players: 3,
            ..ContentRequest::default()
        },
        map: MapKind::Flat,
        ai: Some(Vec::new()),
        humans: vec![PLAYER_ONE],
        pace,
        ..MatchSettings::default()
    }
}

/// Leads `ticks` with the computers playing and a follower fed the journal; the leader's and
/// the follower's final hashes and the report.
fn run(pace: EconomyPace, ticks: u64) -> (u64, u64, String) {
    let mut leader = settings(pace).build().expect("leader").leader();
    let mut follower = settings(pace).build().expect("follower").follower();
    assert_eq!(leader.world().pace(), pace);
    assert_eq!(follower.world().pace(), pace);
    for _ in 0..ticks {
        let outcome = leader.lead(Vec::new());
        follower.follow(&outcome.journal).expect("in step");
    }
    let report = MatchReport::of(leader.world(), &MatchInfo::default()).to_text();
    (leader.world().hash(), follower.world().hash(), report)
}

#[test]
fn the_pace_reaches_the_world_a_follower_and_the_report_and_repeats() {
    assert_eq!(MatchSettings::default().pace, EconomyPace::default());
    let slow = EconomyPace {
        gather_rate: 10,
        research_time: 400,
    };
    let (leader, follower, report) = run(slow, 1_200);
    assert_eq!(leader, follower);
    assert!(
        report.contains("gathering 10 % · research time 400 %"),
        "{report}"
    );
    let (again, _, _) = run(slow, 1_200);
    assert_eq!(again, leader, "run twice, the same hash");
    let (usual, usual_follower, report) = run(EconomyPace::default(), 1_200);
    assert_eq!(usual, usual_follower);
    assert!(
        report.contains("gathering 100 % · research time 100 %"),
        "{report}"
    );
    assert_ne!(usual, leader);
    // A pace outside the lists is refused when the match is built.
    assert!(
        settings(EconomyPace {
            gather_rate: 3,
            research_time: 100
        })
        .build()
        .is_err()
    );
}
