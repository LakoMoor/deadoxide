---
name: zomboid-oracle
description: Launch and drive the real Project Zomboid (42.21.0) in an isolated profile to capture oracle data for the Rust port: screenshots, the stdout log, player position from the save database. Use when the port needs a measured reference (lighting, camera, zones, zombie numbers) rather than a guess.
---

# Zomboid oracle capture (adapted from Horizon's agent-control / launch rules)

Horizon keeps agent runs away from real player data and drives the real client through a control API. Project Zomboid has no
control API, so this drives the window with Win32 input (`tools/pzdrive.ps1`) and reads files.

## Rules that bite

- **Ask before launching** unless the user already said "запускай игру" for this task; the game takes window focus and the user may be at the PC.
- **Never touch the real save/profile.** Always start with `-cachedir=<repo>/profile` (isolated). Saves, settings and logs land there.
- **Capture only the client area of the game window** (the script does); never the whole screen: it exposes the user's other windows.
- Kill only by the exact PID you started. No `taskkill /IM`, no wildcard kills.
- Do not click into the world while the game is running unless you mean to move the character; paused is safer for screenshots.
- Do not commit game files (`media/`, `projectzomboid.jar`), decompiled sources or the profile folder.

## Launch (Windows, from the game folder)

```powershell
cd E:\Games\steamapps\common\ProjectZomboid
$p = Start-Process jre64\bin\java.exe -PassThru -ArgumentList '--enable-native-access=ALL-UNNAMED','--add-exports=java.base/jdk.internal.misc=ALL-UNNAMED','-Xmx3072m','-XX:+UseZGC','-Djava.library.path=win64/;.','-cp','.;projectzomboid.jar','zombie.gameStates.MainScreenState','-nosteam',"-cachedir=$PWD\zomboid-rs\profile" -RedirectStandardOutput zomboid-rs\profile\stdout.txt -RedirectStandardError zomboid-rs\profile\stderr.txt
$env:PZ_PID = $p.Id
```

Main menu is reached when `stdout.txt` shows `binkId`/`namedColorToTable ... ProgressYellow` (the menu build). The user must have accepted the legal dialog once.

## Drive and capture

```powershell
powershell -File zomboid-rs\tools\pzdrive.ps1 rect                         # client area size/origin
powershell -File zomboid-rs\tools\pzdrive.ps1 click 277 553                # "Continue" in the 1920x1050 menu (measured)
powershell -File zomboid-rs\tools\pzdrive.ps1 click 960 500                # "click to start" screen
powershell -File zomboid-rs\tools\pzdrive.ps1 click 1527 839               # close the survival-guide dialog
powershell -File zomboid-rs\tools\pzdrive.ps1 shot zomboid-rs\screenshots\game_x.png
```

Measured click points at 1920x1050: Continue (277,553), start (960,500), guide Close (1527,839), info-panel X (308,108), inventory X (328,8), loot X (968,8).
Actions available: `rect key <hex-vk> click <x> <y> wheel <x> <y> shot <path>`. Set `$env:PZ_PID` first.

## Read facts without pixels

```sh
node --experimental-sqlite zomboid-rs/tools/readplayer.mjs "<profile>/Saves/<mode>/<save>/players.db"   # player x,y,z (copy the db first; read-only)
tail profile/stdout.txt                                                                                  # the real Lua/Java log (golden for tests/golden/lua_real)
```

## Compare with the port

```sh
zomboid-client .. 32_45 --shot screenshots/port.png -216704 639720 1.0 1920 1050 --player 8303.5 11689.5 --level 2
zomboid-ctl imgdiff .. screenshots/game.png screenshots/port.png 650 300 1280 600     # luma diff + best shift in a region
node zomboid-rs/tools/region_luma.mjs screenshots/game.png screenshots/port.png         # mean luma per named region
```

Camera model (one sample, zoom 1, z=0): the real camera centre is 56 px above the player's feet; the player's tile-centre iso is
`((x-y)*64, (x+y)*32 - z*192)`. Record every new measurement, with its conditions, in `docs/COMPATIBILITY.md`.

## Stop

`Stop-Process -Id $p.Id` (exact PID) when done; say what you captured and what it does **not** cover.
