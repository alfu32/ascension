//! One body's playing surface: its terrain and shape, and the grids and indexes the
//! simulation keeps for it. Nothing on one surface ever reaches another.

use crate::{
    BodyId, MapGrid, SimError, Topology, avoidance::Cells, geometry::Geometry, pathing::Pathing,
    spatial::SpatialGrid, territory::TerritoryGrid, vision::SurfaceVision,
};
use rom_rules::{DefId, Rules};

/// A planet, system space or single flat map and everything kept per surface.
#[derive(Debug)]
pub struct Surface {
    pub(crate) map: MapGrid,
    pub(crate) geo: Geometry,
    pub(crate) pathing: Pathing,
    pub(crate) spatial: SpatialGrid,
    pub(crate) territory: TerritoryGrid,
    pub(crate) vision: SurfaceVision,
    pub(crate) cells: Cells,
}

impl Surface {
    /// Checks the terrain against the rules and builds the surface's grids.
    pub(crate) fn new(
        rules: &Rules,
        body: BodyId,
        map: MapGrid,
        topology: Topology,
    ) -> Result<Self, SimError> {
        map.validate()?;
        let geo = Geometry::on_body(topology, body).ok_or(SimError::InvalidMap)?;
        if geo.tiles() != map.terrain.len() {
            return Err(SimError::InvalidMap);
        }
        for terrain in &map.terrain {
            if !rules.terrains.contains_key(terrain) {
                return Err(SimError::UnknownTerrain(*terrain));
            }
        }
        Ok(Self {
            pathing: Pathing::new(rules, &map, geo.clone()),
            territory: TerritoryGrid::new(rules, &map),
            cells: Cells::new(&geo),
            spatial: SpatialGrid::default(),
            vision: SurfaceVision::default(),
            map,
            geo,
        })
    }

    /// The body this surface belongs to.
    pub fn body(&self) -> BodyId {
        self.geo.body
    }

    /// The surface's shape.
    pub fn topology(&self) -> Topology {
        self.geo.topology
    }

    /// Terrain storage; a sphere's faces are stacked as one strip.
    pub fn map(&self) -> &MapGrid {
        &self.map
    }

    /// One terrain definition per tile, in dense tile order.
    pub fn terrain(&self) -> &[DefId] {
        &self.map.terrain
    }

    /// Number of tiles.
    pub fn tiles(&self) -> usize {
        self.map.terrain.len()
    }
}
