//! Direct port of the game's Java classes (build pinned in `reference.lock.json`).
//! Module paths mirror Java packages: `zombie.iso.IsoDirections` lives in `zombie::iso::iso_directions`.
//! Every ported item's doc comment opens with the Java symbol it follows; `tools/class_ledger.mjs` counts them.
//! Singletons of the original (`IsoSpriteManager.instance`, `TilePropertyAliasMap.instance`) are explicit values here.
#![forbid(unsafe_code)]
pub mod java;
pub mod zombie;
