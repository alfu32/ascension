//! The economy's pace (setup options, task 159): the share of every income drawn from the world
//! a player receives, and how long every research takes.

use crate::{PlayerId, SimError, World};
use rom_rules::DefId;

/// How fast a match's economy runs, in percent. Unit speed, combat, build and training times,
/// costs and deposits are untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EconomyPace {
    /// Percent of every income drawn from the world (gathering, yields, claimed bodies, trade
    /// deliveries, completion bonuses); one of [`EconomyPace::GATHER_RATES`].
    pub gather_rate: u32,
    /// Percent of every research's time, age advances and the ascension included; one of
    /// [`EconomyPace::RESEARCH_TIMES`].
    pub research_time: u32,
}

impl Default for EconomyPace {
    fn default() -> Self {
        Self {
            gather_rate: 100,
            research_time: 100,
        }
    }
}

impl EconomyPace {
    /// The gathering rates a setup may choose, the default first.
    pub const GATHER_RATES: [u32; 6] = [100, 50, 25, 10, 5, 2];
    /// The research times a setup may choose, the default first.
    pub const RESEARCH_TIMES: [u32; 5] = [100, 200, 400, 1000, 2000];

    /// Whether both options are at their defaults.
    pub fn is_default(self) -> bool {
        self == Self::default()
    }

    /// Whether both values are in their lists.
    pub fn is_valid(self) -> bool {
        Self::GATHER_RATES.contains(&self.gather_rate)
            && Self::RESEARCH_TIMES.contains(&self.research_time)
    }

    /// Reads a gathering rate (`25` or `25%`); `None` outside [`EconomyPace::GATHER_RATES`].
    pub fn parse_gather_rate(text: &str) -> Option<u32> {
        percent(text).filter(|pct| Self::GATHER_RATES.contains(pct))
    }

    /// Reads a research time (`400` or `400%`); `None` outside [`EconomyPace::RESEARCH_TIMES`].
    pub fn parse_research_time(text: &str) -> Option<u32> {
        percent(text).filter(|pct| Self::RESEARCH_TIMES.contains(pct))
    }

    /// `amount` at the gathering rate, rounded down (what an interface shows as a rate).
    pub fn gathered(self, amount: i64) -> i64 {
        if self.gather_rate == 100 {
            return amount;
        }
        (i128::from(amount) * i128::from(self.gather_rate) / 100) as i64
    }

    /// `ticks` of research at the research time.
    pub fn research_ticks(self, ticks: i64) -> i64 {
        if self.research_time == 100 {
            return ticks;
        }
        (i128::from(ticks) * i128::from(self.research_time) / 100).min(i128::from(i64::MAX)) as i64
    }
}

fn percent(text: &str) -> Option<u32> {
    text.trim().trim_end_matches('%').trim().parse().ok()
}

impl World {
    /// Sets the economy's pace of the setup (task 159), before the first tick; values outside
    /// the lists are refused.
    pub fn set_pace(&mut self, pace: EconomyPace) -> Result<(), SimError> {
        if self.tick != 0 || !pace.is_valid() {
            return Err(SimError::InvalidOptions);
        }
        self.victory.options.pace = pace;
        Ok(())
    }

    /// The setup's economy pace.
    pub fn pace(&self) -> EconomyPace {
        self.victory.options.pace
    }

    /// Credits `amount` of `resource` drawn from the world to `player` at the gathering rate and
    /// returns what reached the stock. What falls below one unit is kept, in hundredths, in the
    /// player's remainder for that resource, so a trickle at 2 % still adds up. Nothing is
    /// scaled at 100 %, and a negative amount (a cost) never is.
    pub(crate) fn credit_gathered(
        &mut self,
        player: PlayerId,
        resource: DefId,
        amount: i64,
    ) -> i64 {
        let pct = self.victory.options.pace.gather_rate;
        let paid = if pct == 100 || amount <= 0 {
            amount
        } else {
            let remainder = self
                .gather_remainders
                .entry((player, resource))
                .or_default();
            let total = i128::from(amount) * i128::from(pct) + i128::from(*remainder);
            *remainder = (total % 100) as i64;
            if *remainder == 0 {
                self.gather_remainders.remove(&(player, resource));
            }
            (total / 100).min(i128::from(i64::MAX)) as i64
        };
        if let Some(p) = self.players.get_mut(&player) {
            let balance = p.resources.entry(resource).or_default();
            *balance = balance.saturating_add(paid);
        }
        paid
    }

    /// Every remainder below one unit of gathered income, in hundredths, per player and
    /// resource, in key order (empty at the default gathering rate).
    pub fn gather_remainders(&self) -> impl Iterator<Item = ((PlayerId, DefId), i64)> + '_ {
        self.gather_remainders.iter().map(|(k, v)| (*k, *v))
    }
}

#[cfg(test)]
mod tests {
    use super::EconomyPace;

    #[test]
    fn values_outside_the_lists_are_refused() {
        assert_eq!(EconomyPace::parse_gather_rate("25"), Some(25));
        assert_eq!(EconomyPace::parse_gather_rate(" 2 % "), Some(2));
        assert_eq!(EconomyPace::parse_gather_rate("30"), None);
        assert_eq!(EconomyPace::parse_gather_rate("0"), None);
        assert_eq!(EconomyPace::parse_gather_rate("fast"), None);
        assert_eq!(EconomyPace::parse_research_time("1000%"), Some(1000));
        assert_eq!(EconomyPace::parse_research_time("50"), None);
        assert!(EconomyPace::default().is_default());
        assert!(
            !EconomyPace {
                gather_rate: 3,
                research_time: 100
            }
            .is_valid()
        );
    }

    #[test]
    fn scaling_is_exact_at_the_default() {
        let pace = EconomyPace::default();
        assert_eq!(pace.gathered(37), 37);
        assert_eq!(pace.research_ticks(1234), 1234);
        let slow = EconomyPace {
            gather_rate: 25,
            research_time: 400,
        };
        assert_eq!(slow.gathered(37), 9);
        assert_eq!(slow.research_ticks(1234), 4936);
    }
}
