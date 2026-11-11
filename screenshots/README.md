# screenshots
All screenshots (real game and port) go here.
- game_ingame.png — real game, Rosewood spawn (oracle)
- port_z0_aligned.png — port with the measured camera offset (cy = feet - 56 px); port_z0_before_alignment.png — before
- port_all_levels.png, port_cpu_cell41_39.png, port_gpu_cell41_39.png — port renders
- port_2026-11-10.png — current Rust client, Rosewood x=8303.5 y=11689.5 z=0, zoom 1.0, 1920x1050, experimental `--lit-room`; red player marker is a placeholder. Original lighting and visibility are not fully ported.
- port_collision_2026-11-10.png — same scenario after the static collision flag correction; a still image does not test movement.
- port_diagonal_2026-11-10.png — same scenario after the diagonal square-collision update; a still image does not test movement.
- port_water_2026-11-10.png — same scenario after the water collision update; the frame contains no tested water and does not test movement.
- port_windows_2026-11-11.png — same scenario after the window-flag collision update; the still does not test movement or window interaction.
- port_adjacent_2026-11-11.png — same scenario after adjacent-window collision changes; the still does not test movement or window interaction.
- port_real_water_2026-11-11.png — Rust cell 0_18, x=1 y=4803 z=0, zoom 1, 1920x1050. Edge of the loaded cell, red placeholder player; no original screenshot comparison exists here.
Command: `zomboid-client .. 32_45 --shot screenshots/<name>.png -216704 639720 1.0 1920 1050 --player 8303.5 11689.5 --level 2`

- `port_save_auto_2026-11-11.png`: port screenshot from the isolated saved player, z=0, experimental room lighting; matches explicit port coordinates byte-for-byte.
- `port_save_z1_auto_2026-11-11.png`: port screenshot from a scratch copy of that database with synthetic z=1; this is not an original upstairs save or a game comparison.
