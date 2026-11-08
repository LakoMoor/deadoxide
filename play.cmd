@echo off
rem Zomboid-rs launcher: builds (release) and opens the Rust client over your own Project Zomboid install in ..
rem   play.cmd                      opens the Rosewood spawn region (cell 32_45)
rem   play.cmd 41_39                another cell (cx_cy), or cellA:cellB for a rectangle of cells
rem   play.cmd save [folder]         start at the saved character position (default: the isolated profile save)
rem   play.cmd shot out.png         headless screenshot of the spawn sample (writes screenshots\out.png)
rem Keys in the window: see docs/PORT.md (WASD move, mouse drag pan, wheel zoom, PageUp/PageDown floors).
setlocal
cd /d "%~dp0"
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
if /i "%~1"=="shot" (
  cargo run -q --release -p zomboid-client -- .. 32_45 --shot screenshots\%~2 -216704 639720 1.0 1920 1050 --player 8303.5 11689.5 --level 2 --lit-room
  goto :eof
)
if /i "%~1"=="save" (
  rem start where the real saved character stands (save folder from the isolated profile; pass another folder as the 2nd argument)
  set "SAVE=%~2"
  if "%SAVE%"=="" for /d %%D in ("profileSavesSandbox*") do set "SAVE=%%D"
  cargo run -q --release -p zomboid-client -- .. --save "%SAVE%"
  goto :eof
)
set "CELL=%~1"
if "%CELL%"=="" set "CELL=32_45"
cargo run -q --release -p zomboid-client -- .. %CELL%
