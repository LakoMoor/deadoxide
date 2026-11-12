//! zombie.iso.SpriteDetails.IsoObjectType
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[repr(u8)]
pub enum IsoObjectType {
    normal = 0,
    jukebox = 1,
    wall = 2,
    stairsTW = 3,
    stairsTN = 4,
    stairsMW = 5,
    stairsMN = 6,
    stairsBW = 7,
    stairsBN = 8,
    UNUSED9 = 9,
    UNUSED10 = 10,
    doorW = 11,
    doorN = 12,
    lightswitch = 13,
    radio = 14,
    curtainN = 15,
    curtainS = 16,
    curtainW = 17,
    curtainE = 18,
    doorFrW = 19,
    doorFrN = 20,
    tree = 21,
    windowFN = 22,
    windowFW = 23,
    UNUSED24 = 24,
    WestRoofB = 25,
    WestRoofM = 26,
    WestRoofT = 27,
    isMoveAbleObject = 28,
    MAX = 29,
}

pub const VALUES: [IsoObjectType; 30] = [
    IsoObjectType::normal,
    IsoObjectType::jukebox,
    IsoObjectType::wall,
    IsoObjectType::stairsTW,
    IsoObjectType::stairsTN,
    IsoObjectType::stairsMW,
    IsoObjectType::stairsMN,
    IsoObjectType::stairsBW,
    IsoObjectType::stairsBN,
    IsoObjectType::UNUSED9,
    IsoObjectType::UNUSED10,
    IsoObjectType::doorW,
    IsoObjectType::doorN,
    IsoObjectType::lightswitch,
    IsoObjectType::radio,
    IsoObjectType::curtainN,
    IsoObjectType::curtainS,
    IsoObjectType::curtainW,
    IsoObjectType::curtainE,
    IsoObjectType::doorFrW,
    IsoObjectType::doorFrN,
    IsoObjectType::tree,
    IsoObjectType::windowFN,
    IsoObjectType::windowFW,
    IsoObjectType::UNUSED24,
    IsoObjectType::WestRoofB,
    IsoObjectType::WestRoofM,
    IsoObjectType::WestRoofT,
    IsoObjectType::isMoveAbleObject,
    IsoObjectType::MAX,
];

pub const NAMES: [&str; 30] = [
    "normal",
    "jukebox",
    "wall",
    "stairsTW",
    "stairsTN",
    "stairsMW",
    "stairsMN",
    "stairsBW",
    "stairsBN",
    "UNUSED9",
    "UNUSED10",
    "doorW",
    "doorN",
    "lightswitch",
    "radio",
    "curtainN",
    "curtainS",
    "curtainW",
    "curtainE",
    "doorFrW",
    "doorFrN",
    "tree",
    "windowFN",
    "windowFW",
    "UNUSED24",
    "WestRoofB",
    "WestRoofM",
    "WestRoofT",
    "isMoveAbleObject",
    "MAX",
];

impl IsoObjectType {
    /// zombie.iso.SpriteDetails.IsoObjectType.index
    pub fn index(self) -> usize {
        self as usize
    }
    /// zombie.iso.SpriteDetails.IsoObjectType.fromIndex
    pub fn from_index(value: usize) -> Self {
        VALUES[value]
    }
    /// zombie.iso.SpriteDetails.IsoObjectType.FromString
    pub fn from_string(s: &str) -> Self {
        match NAMES[..NAMES.len() - 1].iter().position(|n| *n == s) {
            Some(i) => VALUES[i],
            None => IsoObjectType::MAX,
        }
    }
    pub fn name(self) -> &'static str {
        NAMES[self as usize]
    }
}
