//! zombie.core.properties.FasciaEdge
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FasciaEdge {
    West,
    North,
    East,
    South,
    NorthWestCorner,
    NorthEastCorner,
    SouthEastCorner,
    SouthWestCorner,
    NorthAndWest,
    NorthAndEast,
    SouthAndEast,
    SouthAndWest,
}
impl FasciaEdge {
    /// java.lang.Enum.valueOf through zombie.util.StringUtils.tryParseEnum.
    pub fn value_of(s: &str) -> Option<Self> {
        Some(match s {
            "West" => FasciaEdge::West,
            "North" => FasciaEdge::North,
            "East" => FasciaEdge::East,
            "South" => FasciaEdge::South,
            "NorthWestCorner" => FasciaEdge::NorthWestCorner,
            "NorthEastCorner" => FasciaEdge::NorthEastCorner,
            "SouthEastCorner" => FasciaEdge::SouthEastCorner,
            "SouthWestCorner" => FasciaEdge::SouthWestCorner,
            "NorthAndWest" => FasciaEdge::NorthAndWest,
            "NorthAndEast" => FasciaEdge::NorthAndEast,
            "SouthAndEast" => FasciaEdge::SouthAndEast,
            "SouthAndWest" => FasciaEdge::SouthAndWest,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            FasciaEdge::West => "West",
            FasciaEdge::North => "North",
            FasciaEdge::East => "East",
            FasciaEdge::South => "South",
            FasciaEdge::NorthWestCorner => "NorthWestCorner",
            FasciaEdge::NorthEastCorner => "NorthEastCorner",
            FasciaEdge::SouthEastCorner => "SouthEastCorner",
            FasciaEdge::SouthWestCorner => "SouthWestCorner",
            FasciaEdge::NorthAndWest => "NorthAndWest",
            FasciaEdge::NorthAndEast => "NorthAndEast",
            FasciaEdge::SouthAndEast => "SouthAndEast",
            FasciaEdge::SouthAndWest => "SouthAndWest",
        }
    }
}
