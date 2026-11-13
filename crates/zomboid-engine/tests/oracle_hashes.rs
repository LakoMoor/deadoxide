//! Checks the port against the ORIGINAL game's output without needing Java: `tools/oracle.sh` stores the sha256 of
//! each Java probe's output once it is byte-identical to `pz-dump`; here the Rust side is recomputed over the
//! player's install and must still hash to the same value. Needs the game in the workspace's parent folder.
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

fn game_root() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    root.join("media/newtiledefinitions.tiles")
        .exists()
        .then_some(root)
}

fn check(case: &str, args: &[&str]) {
    check_as(case, case, args)
}

fn check_as(golden: &str, case: &str, args: &[&str]) {
    let Some(root) = game_root() else {
        eprintln!("SKIPPED {case}: no game install next to the workspace");
        return;
    };
    let want = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../tests/golden/engine/{golden}.sha256")),
    )
    .expect("golden hash");
    let out = Command::new(env!("CARGO_BIN_EXE_pz-dump"))
        .arg(case)
        .arg(&root)
        .args(args)
        .output()
        .expect("run pz-dump");
    assert!(
        out.status.success(),
        "pz-dump {case} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let got: String = Sha256::digest(&out.stdout)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(got, want.trim(), "{case}: Rust output no longer matches the original's (run sh tools/oracle.sh {case} for a diff)");
}

#[test]
fn tiledefs_match_original() {
    check("tiledefs", &[]);
}

#[test]
fn lotheaders_match_original() {
    check("lotheaders", &["Muldraugh, KY"]);
}

#[test]
fn lotpack_matches_original() {
    check("lotpack", &["Muldraugh, KY", "30", "43", "34", "47"]);
}

#[test]
fn metagrid_matches_original() {
    check("metagrid", &["Muldraugh, KY", "30", "43", "34", "47"]);
}

#[test]
fn chunkobjects_match_original() {
    check("chunkobjects", &["1036", "1460", "1039", "1463"]);
}

#[test]
fn collide_matches_original() {
    check("collide", &["1036", "1460", "1039", "1463"]);
}

#[test]
fn entitysprites_match_original() {
    check("entitysprites", &[]);
}

#[test]
fn specialcollide_matches_original() {
    check("specialcollide", &["129", "182"]);
}

#[test]
fn basements_match_original() {
    check("basements", &["30", "43", "34", "47"]);
}

#[test]
fn doortoggle_matches_original() {
    check_as("doortoggle", "specialcollide", &["129", "182", "toggle"]);
}

#[test]
fn los_matches_original() {
    check("los", &["129", "182"]);
}

#[test]
fn los_with_doors_flipped_matches_original() {
    check_as("losdoors", "los", &["129", "182", "toggle"]);
}
