//! What the pointer rests on in the world (task 151): the side of the hovered entity, the order
//! a right click on it gives, and the flash on the target of an order just given. The
//! simulation thread names them in the HUD snapshot; the render thread draws them only while
//! its own pick still finds the same entity under the pointer, so an outline or a badge always
//! means "the pointer is on this".

/// Extra reach of a pick, in design pixels (scaled with the HUD): a pointer that is on no pick
/// box still takes the nearest entity whose drawn box is within this many pixels of it.
pub const PICK_SLACK: f32 = 8.0;

/// Game ticks one lit or dark phase of a target flash lasts.
pub const FLASH_PHASE_TICKS: u32 = 3;
/// Game ticks a target flash lasts: two lit phases with a dark one between.
pub const FLASH_TICKS: u32 = FLASH_PHASE_TICKS * 4;

/// Whose an entity is, as the person sees it. The hover outline and the target flash take
/// their colour from it, never from the team colours, so a red enemy and a red self differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// The person's own.
    Own,
    /// A player on the person's team.
    Ally,
    /// A player the person is at war with.
    Enemy,
    /// Nobody's: nature, relics, resource sites.
    Neutral,
}

impl Side {
    /// The colour of the side: own green, ally cyan, enemy red, neutral grey.
    pub fn colour(self) -> [u8; 4] {
        match self {
            Self::Own => [70, 235, 90, 255],
            Self::Ally => [60, 215, 255, 255],
            Self::Enemy => [255, 55, 45, 255],
            Self::Neutral => [185, 185, 185, 255],
        }
    }

    /// The hover ring's colour: the side's, lightened by a third, so it reads apart from the
    /// selection ring of the same side drawn just inside it.
    pub fn hover_colour(self) -> [u8; 4] {
        let [r, g, b, a] = self.colour();
        let light = |c: u8| (u16::from(c) + (255 - u16::from(c)) / 3) as u8;
        [light(r), light(g), light(b), a]
    }

    /// The side of `owner` in a table indexed by player id (as [`crate::HudSnapshot::sides`]):
    /// nobody's for player 0, the person's own for a player the table does not list.
    pub fn of_owner(sides: &[Side], owner: u8) -> Side {
        match owner {
            0 => Self::Neutral,
            _ => sides.get(usize::from(owner)).copied().unwrap_or(Self::Own),
        }
    }

    /// The word the help line starts with.
    pub fn word(self) -> &'static str {
        match self {
            Self::Own => "Yours",
            Self::Ally => "Ally",
            Self::Enemy => "Enemy",
            Self::Neutral => "Neutral",
        }
    }
}

/// The order a right click on the hovered entity would give the selection, in the order the
/// badge prefers them when the selected units would do different things.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OrderBadge {
    /// Attack it.
    Attack,
    /// Build it (an own construction site).
    Build,
    /// Mend it.
    Repair,
    /// Work at it (an own gather site).
    Gather,
    /// Carry it home (a relic).
    Carry,
    /// Board it (a carrier).
    Board,
    /// Run a trade route to it.
    Trade,
    /// Land on it (an airfield).
    Land,
    /// Send new units to work at it (a producer's rally).
    Rally,
}

impl OrderBadge {
    /// The HUD icon drawn in the badge.
    pub fn icon(self) -> &'static str {
        match self {
            Self::Attack => "icons/cmd_attack",
            Self::Build => "icons/cmd_build",
            Self::Repair => "icons/cmd_repair",
            Self::Gather => "icons/stat_workers",
            Self::Carry => "icons/flag",
            Self::Board => "icons/cmd_garrison",
            Self::Trade => "icons/res_wealth",
            Self::Land => "icons/cmd_unload",
            Self::Rally => "icons/cmd_rally",
        }
    }

    /// What the right click does, for the help line.
    pub fn verb(self) -> &'static str {
        match self {
            Self::Attack => "attack",
            Self::Build => "build",
            Self::Repair => "repair",
            Self::Gather => "gather here",
            Self::Carry => "carry it",
            Self::Board => "board",
            Self::Trade => "trade route",
            Self::Land => "land here",
            Self::Rally => "rally here",
        }
    }

    /// True for an order against the target: its flash is red, else green.
    pub fn hostile(self) -> bool {
        self == Self::Attack
    }
}

/// The entity the simulation thread was last told the pointer rests on, as it sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudPointer {
    /// Entity id.
    pub entity: u32,
    /// Whose it is.
    pub side: Side,
    /// The order a right click on it gives the selection; `None` when the selection cannot act
    /// on it (or nothing is selected).
    pub badge: Option<OrderBadge>,
    /// The help line: side, name, owner and health ("Enemy · Corvette (Ashscale Brood) · 80/100").
    pub text: String,
    /// The help line's second row: what a right click on it does ("Right-click: attack", "Trade
    /// route to Ostia: +12 per trip"); empty when it does nothing special.
    pub hint: String,
}

/// The target of an order just given by a right click: its ring flashes twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudFlash {
    /// Entity id of the target.
    pub entity: u32,
    /// True for an attack (red), false for a friendly order (green).
    pub hostile: bool,
    /// Ticks since the order was given.
    pub age_ticks: u32,
}

impl HudFlash {
    /// Whether the ring is lit at this age: two lit phases with a dark one between.
    pub fn lit(&self) -> bool {
        self.age_ticks < FLASH_TICKS && (self.age_ticks / FLASH_PHASE_TICKS).is_multiple_of(2)
    }

    /// The colour of the flash, by the order: red for an attack, green for a friendly order.
    pub fn colour(&self) -> [u8; 4] {
        if self.hostile {
            Side::Enemy.colour()
        } else {
            Side::Own.colour()
        }
    }
}

/// The hover ring the render thread draws: the entity its own pick finds under the pointer,
/// coloured by the side the snapshot gives it, when both name the same entity.
pub fn hover_ring(pointed: Option<u32>, pointer: Option<&HudPointer>) -> Option<(u32, [u8; 4])> {
    let pointer = pointer?;
    (pointed? == pointer.entity).then(|| (pointer.entity, pointer.side.hover_colour()))
}

/// What the world draws for the pointer this frame: the hover ring and the lit flashes, each
/// an entity id and a colour. Kept by the scene and refilled every frame without allocating.
#[derive(Clone, Debug, Default)]
pub(crate) struct PointerMarks {
    /// The hovered entity and its side's colour.
    pub(crate) hover: Option<(u32, [u8; 4])>,
    /// Targets whose flash is lit now, with the order's colour.
    pub(crate) flashes: Vec<(u32, [u8; 4])>,
}

impl PointerMarks {
    /// Takes this frame's hover ring and the lit flashes of `flashes`.
    pub(crate) fn update(&mut self, hover: Option<(u32, [u8; 4])>, flashes: &[HudFlash]) {
        self.hover = hover;
        self.flashes.clear();
        self.flashes.extend(
            flashes
                .iter()
                .filter(|f| f.lit())
                .map(|f| (f.entity, f.colour())),
        );
    }

    /// The colour of the flash lit on `id`, if any.
    pub(crate) fn flash_of(&self, id: u32) -> Option<[u8; 4]> {
        self.flashes.iter().find(|f| f.0 == id).map(|f| f.1)
    }

    /// The colour of the hover ring on `id`, if it is the hovered entity.
    pub(crate) fn hover_of(&self, id: u32) -> Option<[u8; 4]> {
        self.hover.filter(|h| h.0 == id).map(|h| h.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn side_colours_are_distinct_and_not_team_colours() {
        let sides = [Side::Own, Side::Ally, Side::Enemy, Side::Neutral];
        for (i, a) in sides.iter().enumerate() {
            for b in &sides[i + 1..] {
                assert_ne!(a.colour(), b.colour(), "{a:?} and {b:?}");
            }
        }
        // The colours are fixed by side: no player colour decides them.
        let own = Side::Own.colour();
        assert!(own[1] > own[0] && own[1] > own[2], "own is green");
        let enemy = Side::Enemy.colour();
        assert!(enemy[0] > enemy[1] && enemy[0] > enemy[2], "enemy is red");
        let ally = Side::Ally.colour();
        assert!(ally[2] > ally[0] && ally[1] > ally[0], "ally is cyan");
        let neutral = Side::Neutral.colour();
        assert!(
            neutral[0] == neutral[1] && neutral[1] == neutral[2],
            "neutral is grey"
        );
    }

    #[test]
    fn the_hover_colour_is_the_sides_lightened_and_owners_map_to_sides() {
        for side in [Side::Own, Side::Ally, Side::Enemy, Side::Neutral] {
            let (base, hover) = (side.colour(), side.hover_colour());
            assert_ne!(base, hover, "{side:?}");
            assert!((0..3).all(|i| hover[i] >= base[i]), "{side:?} lightened");
        }
        let sides = [Side::Neutral, Side::Own, Side::Enemy, Side::Ally];
        assert_eq!(Side::of_owner(&sides, 0), Side::Neutral);
        assert_eq!(Side::of_owner(&sides, 1), Side::Own);
        assert_eq!(Side::of_owner(&sides, 2), Side::Enemy);
        assert_eq!(Side::of_owner(&sides, 3), Side::Ally);
        assert_eq!(Side::of_owner(&[], 4), Side::Own);
    }

    #[test]
    fn a_flash_is_lit_twice_then_gone() {
        let lit: Vec<bool> = (0..FLASH_TICKS + 3)
            .map(|age_ticks| {
                HudFlash {
                    entity: 1,
                    hostile: true,
                    age_ticks,
                }
                .lit()
            })
            .collect();
        let rises = lit.windows(2).filter(|w| !w[0] && w[1]).count() + usize::from(lit[0]);
        assert_eq!(rises, 2);
        assert!(!lit[FLASH_TICKS as usize]);
        let flash = |hostile| HudFlash {
            entity: 1,
            hostile,
            age_ticks: 0,
        };
        assert_eq!(flash(true).colour(), Side::Enemy.colour());
        assert_eq!(flash(false).colour(), Side::Own.colour());
    }

    #[test]
    fn the_hover_ring_needs_the_render_pick_and_the_snapshot_to_agree() {
        let pointer = HudPointer {
            entity: 7,
            side: Side::Enemy,
            badge: Some(OrderBadge::Attack),
            text: String::new(),
            hint: String::new(),
        };
        assert_eq!(
            hover_ring(Some(7), Some(&pointer)),
            Some((7, Side::Enemy.hover_colour()))
        );
        assert_eq!(hover_ring(Some(8), Some(&pointer)), None);
        assert_eq!(hover_ring(None, Some(&pointer)), None);
        assert_eq!(hover_ring(Some(7), None), None);
    }
}
