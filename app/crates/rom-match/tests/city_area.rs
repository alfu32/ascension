//! Task 158: the city area option travels in the match settings to the world, to a follower and
//! to the report.

use rom_match::{ContentRequest, MapKind, MatchReport, MatchSettings, PLAYER_ONE};
use rom_save::MatchInfo;
use rom_sim::CityArea;

fn settings(city_area: CityArea) -> MatchSettings {
    MatchSettings {
        content: ContentRequest {
            seed: 5,
            players: 3,
            ..ContentRequest::default()
        },
        map: MapKind::Flat,
        ai: Some(Vec::new()),
        humans: vec![PLAYER_ONE],
        city_area,
        ..MatchSettings::default()
    }
}

/// Leads `ticks` with the computers playing and a follower fed the journal; the leader's and
/// the follower's final hashes.
fn run(city_area: CityArea, ticks: u64) -> (u64, u64, String) {
    let mut leader = settings(city_area).build().expect("leader").leader();
    let mut follower = settings(city_area).build().expect("follower").follower();
    assert_eq!(leader.world().match_options().city_area, city_area);
    assert_eq!(follower.world().match_options().city_area, city_area);
    for _ in 0..ticks {
        let outcome = leader.lead(Vec::new());
        follower.follow(&outcome.journal).expect("in step");
    }
    let report = MatchReport::of(leader.world(), &MatchInfo::default()).to_text();
    (leader.world().hash(), follower.world().hash(), report)
}

#[test]
fn the_option_reaches_the_world_a_follower_and_the_report_and_repeats() {
    assert_eq!(MatchSettings::default().city_area, CityArea::LimitsBuilding);
    let (leader, follower, report) = run(CityArea::Free, 1_200);
    assert_eq!(leader, follower);
    assert!(
        report.contains("city area: building anywhere in the borders"),
        "{report}"
    );
    let (again, _, _) = run(CityArea::Free, 1_200);
    assert_eq!(again, leader, "run twice, the same hash");
    let (on, on_follower, report) = run(CityArea::LimitsBuilding, 1_200);
    assert_eq!(on, on_follower);
    assert!(report.contains("city area: limits building"), "{report}");
    assert_ne!(on, leader);
}
