//! DeadOxide launcher on GPUI Fast (the same UI stack as Horizon's launcher). Tabs: Play / Mods / Progress / About.
//! Play starts the Rust client (read-only over the player's own install) or the original game with an isolated `-cachedir`.
//! Mods come from a registry (`registry.json`): download -> verify sha256 -> extract into `<profile>/mods`.
#![recursion_limit = "512"]
use gpui::{prelude::*, *};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::mpsc;
use std::time::Duration;

const SITE: &str = "https://lakomoor.github.io/deadoxide/";
const BG: u32 = 0x0e0b0a;
const CARD: u32 = 0x1d1713;
const LINE: u32 = 0x34281f;
const TEXT: u32 = 0xece3da;
const MUTED: u32 = 0xa89a8c;
const RUST: u32 = 0xe8692a;
const RUST2: u32 = 0xff9a55;
const BAD: u32 = 0xe07070;

#[derive(Deserialize, Clone, Debug)]
struct Mod {
    id: String,
    name: String,
    version: String,
    author: String,
    description: String,
    url: String,
    sha256: String,
}
#[derive(Deserialize)]
struct Registry {
    mods: Vec<Mod>,
}
#[derive(Deserialize, Clone)]
struct Area {
    name: String,
    percent: f32,
    measured: bool,
}
#[derive(Deserialize, Clone)]
struct Progress {
    overall: f32,
    generated: String,
    areas: Vec<Area>,
}

enum Msg {
    Registry(Result<Vec<Mod>, String>),
    Progress(Result<Progress, String>),
    Installed(String, Result<PathBuf, String>),
}

fn get_bytes(url: &str) -> Result<Vec<u8>, String> {
    if let Some(p) = url.strip_prefix("file:") {
        let mut v = std::fs::read(p).map_err(|e| e.to_string())?;
        if v.starts_with(&[0xEF, 0xBB, 0xBF]) {
            v.drain(..3);
        }
        return Ok(v);
    }
    let mut r = ureq::get(url).call().map_err(|e| format!("{url}: {e}"))?;
    let mut v = Vec::new();
    r.body_mut()
        .as_reader()
        .take(64 << 20)
        .read_to_end(&mut v)
        .map_err(|e| e.to_string())?;
    if v.starts_with(&[0xEF, 0xBB, 0xBF]) {
        v.drain(..3);
    }
    Ok(v)
}

/// Verify sha256, then extract. Entries that would escape `dest` (zip-slip) are rejected.
fn install(m: &Mod, base: &str, mods_dir: &Path) -> Result<PathBuf, String> {
    let url = if m.url.contains("://") || m.url.starts_with("file:") {
        m.url.clone()
    } else {
        format!("{base}{}", m.url)
    };
    let data = get_bytes(&url)?;
    let got: String = Sha256::digest(&data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if !got.eq_ignore_ascii_case(&m.sha256) {
        return Err(format!(
            "sha256 не совпал: ожидали {}, получили {got}",
            m.sha256
        ));
    }
    let dest = mods_dir.join(&m.id);
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(data)).map_err(|e| e.to_string())?;
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|e| e.to_string())?;
        let rel = f
            .enclosed_name()
            .ok_or_else(|| format!("небезопасный путь в архиве: {}", f.name()))?;
        let out = dest.join(rel);
        if f.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = out.parent() {
                std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut w = std::fs::File::create(&out).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, &mut w).map_err(|e| e.to_string())?;
        }
    }
    Ok(dest)
}

fn find_game() -> Option<PathBuf> {
    let mut c: Vec<PathBuf> = vec![];
    if let Ok(p) = std::env::var("PZ_ROOT") {
        c.push(p.into());
    }
    let mut d = std::env::current_dir().ok();
    while let Some(p) = d {
        c.push(p.clone());
        d = p.parent().map(Path::to_path_buf);
    }
    for pf in [
        "C:/Program Files (x86)/Steam",
        "C:/Program Files/Steam",
        "D:/Steam",
        "D:/SteamLibrary",
        "E:/Games",
        "E:/SteamLibrary",
    ] {
        c.push(Path::new(pf).join("steamapps/common/ProjectZomboid"));
    }
    c.into_iter()
        .find(|p| p.join("projectzomboid.jar").exists())
}

fn find_client() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "zomboid-client.exe"
    } else {
        "zomboid-client"
    };
    let mut c = vec![];
    if let Ok(e) = std::env::current_exe() {
        if let Some(d) = e.parent() {
            c.push(d.join(name));
        }
    }
    c.push(PathBuf::from("target/release").join(name));
    c.push(PathBuf::from("zomboid-rs/target/release").join(name));
    c.into_iter().find(|p| p.exists())
}

fn find_players_db(dir: &Path) -> Option<PathBuf> {
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Some(f) = find_players_db(&p) {
                return Some(f);
            }
        } else if p.file_name().is_some_and(|n| n == "players.db") {
            return Some(p);
        }
    }
    None
}

struct Launcher {
    tab: usize,
    game: Option<PathBuf>,
    profile: PathBuf,
    client: Option<PathBuf>,
    child: Option<Child>,
    status: String,
    mods: Vec<Mod>,
    mods_err: Option<String>,
    progress: Option<Progress>,
    prog_err: Option<String>,
    busy: Vec<String>,
    base: String,
    tx: mpsc::Sender<Msg>,
    rx: mpsc::Receiver<Msg>,
}

impl Launcher {
    fn new(cx: &mut Context<Self>) -> Self {
        let (tx, rx) = mpsc::channel();
        let base = std::env::var("DEADOXIDE_SITE").unwrap_or_else(|_| SITE.to_string());
        let game = find_game();
        let profile = game
            .as_ref()
            .map(|g| g.join("zomboid-rs/profile"))
            .unwrap_or_else(|| PathBuf::from("profile"));
        let tab = std::env::var("DEADOXIDE_TAB")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let me = Launcher {
            tab,
            game,
            profile,
            client: find_client(),
            child: None,
            status: String::new(),
            mods: vec![],
            mods_err: None,
            progress: None,
            prog_err: None,
            busy: vec![],
            base,
            tx,
            rx,
        };
        me.fetch();
        // Worker threads report over a channel; this loop moves the results into the view.
        cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(200))
                .await;
            if this.update(cx, |t, cx| t.poll(cx)).is_err() {
                break;
            }
        })
        .detach();
        me
    }
    fn fetch(&self) {
        let (tx, base) = (self.tx.clone(), self.base.clone());
        std::thread::spawn(move || {
            let reg = get_bytes(&format!("{base}registry.json"))
                .and_then(|b| serde_json::from_slice::<Registry>(&b).map_err(|e| e.to_string()))
                .map(|r| r.mods);
            let _ = tx.send(Msg::Registry(reg));
            let p = get_bytes(&format!("{base}progress.json"))
                .and_then(|b| serde_json::from_slice::<Progress>(&b).map_err(|e| e.to_string()));
            let _ = tx.send(Msg::Progress(p));
        });
    }
    fn poll(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        while let Ok(m) = self.rx.try_recv() {
            changed = true;
            match m {
                Msg::Registry(Ok(v)) => {
                    self.mods = v;
                    self.mods_err = None;
                }
                Msg::Registry(Err(e)) => self.mods_err = Some(e),
                Msg::Progress(Ok(p)) => self.progress = Some(p),
                Msg::Progress(Err(e)) => self.prog_err = Some(e),
                Msg::Installed(id, r) => {
                    self.busy.retain(|b| *b != id);
                    self.status = match r {
                        Ok(p) => format!("Мод {id} установлен: {}", p.display()),
                        Err(e) => format!("Мод {id}: ошибка: {e}"),
                    };
                }
            }
        }
        if let Some(c) = &mut self.child {
            if matches!(c.try_wait(), Ok(Some(_))) {
                self.child = None;
                changed = true;
            }
        }
        if changed {
            cx.notify();
        }
    }
    fn mods_dir(&self) -> PathBuf {
        self.profile.join("mods")
    }
    fn play_port(&mut self) {
        let (Some(g), Some(c)) = (&self.game, &self.client) else {
            return;
        };
        let mut cmd = Command::new(c);
        cmd.arg(g).arg("32_45");
        if let Some(db) = find_players_db(&self.profile.join("Saves")) {
            cmd.arg("--save").arg(db);
        }
        self.status = match cmd.spawn() {
            Ok(ch) => {
                self.child = Some(ch);
                "Клиент порта запущен".into()
            }
            Err(e) => format!("Не удалось запустить клиент: {e}"),
        };
    }
    fn play_original(&mut self) {
        let Some(g) = &self.game else { return };
        let _ = std::fs::create_dir_all(&self.profile);
        let r = Command::new(g.join("jre64/bin/java.exe"))
            .current_dir(g)
            .args([
                "--enable-native-access=ALL-UNNAMED",
                "--add-exports=java.base/jdk.internal.misc=ALL-UNNAMED",
                "-Xmx3072m",
                "-XX:+UseZGC",
                "-Djava.library.path=win64/;.",
                "-cp",
                ".;projectzomboid.jar",
                "zombie.gameStates.MainScreenState",
                "-nosteam",
            ])
            .arg(format!("-cachedir={}", self.profile.display()))
            .spawn();
        self.status = match r {
            Ok(ch) => {
                self.child = Some(ch);
                "Оригинальная игра запущена с отдельным профилем".into()
            }
            Err(e) => format!("Не удалось запустить игру: {e}"),
        };
    }
    fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    primary: bool,
    enabled: bool,
) -> Stateful<Div> {
    let (bg, fg) = if !enabled {
        (0x2a1f18, 0x6b5f54)
    } else if primary {
        (RUST, 0x1a0e07)
    } else {
        (CARD, TEXT)
    };
    let b = div()
        .id(id)
        .px_4()
        .py_2()
        .rounded_lg()
        .border_1()
        .border_color(rgb(if primary && enabled { RUST2 } else { LINE }))
        .bg(rgb(bg))
        .text_color(rgb(fg))
        .child(label.into());
    if enabled {
        b.cursor_pointer().hover(|s| s.opacity(0.85))
    } else {
        b
    }
}

fn bar(pct: f32) -> impl IntoElement {
    div()
        .h(px(8.))
        .w_full()
        .rounded_full()
        .bg(rgb(0x2a1f18))
        .child(
            div()
                .h_full()
                .w(relative((pct / 100.0).clamp(0.0, 1.0)))
                .rounded_full()
                .bg(rgb(RUST)),
        )
}

impl Render for Launcher {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tabs = ["Играть", "Моды", "Прогресс", "О проекте"]
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let on = self.tab == i;
                div()
                    .id(("tab", i))
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .cursor_pointer()
                    .text_color(rgb(if on { 0x1a0e07 } else { MUTED }))
                    .bg(rgb(if on { RUST } else { BG }))
                    .child(*n)
                    .on_click(cx.listener(move |t, _, _, cx| {
                        t.tab = i;
                        cx.notify();
                    }))
            });
        let body: AnyElement = match self.tab {
            0 => {
                let running = self.child.is_some();
                let can_port = self.game.is_some() && self.client.is_some() && !running;
                let can_orig = self.game.is_some() && !running;
                let mut col = div().flex().flex_col().gap_3();
                col = col.child(match &self.game {
                    Some(g) => div().child(format!("Project Zomboid: {}", g.display())),
                    None => div().text_color(rgb(BAD)).child("Установка не найдена. Задайте переменную PZ_ROOT."),
                });
                col = col.child(div().text_color(rgb(MUTED)).child(format!("Профиль (отдельный от ваших сохранений): {}", self.profile.display())));
                let mut row = div().flex().gap_3().child(button("play-port", "Запустить клиент порта", true, can_port).on_click(cx.listener(|t, _, _, cx| {
                    if t.game.is_some() && t.client.is_some() && t.child.is_none() {
                        t.play_port();
                        cx.notify();
                    }
                })));
                row = row.child(button("play-orig", "Запустить оригинал (отдельный профиль)", false, can_orig).on_click(cx.listener(|t, _, _, cx| {
                    if t.game.is_some() && t.child.is_none() {
                        t.play_original();
                        cx.notify();
                    }
                })));
                if running {
                    row = row.child(button("stop", "Остановить", false, true).on_click(cx.listener(|t, _, _, cx| {
                        t.stop();
                        t.status = "Остановлено".into();
                        cx.notify();
                    })));
                }
                col = col.child(row);
                if self.client.is_none() {
                    col = col.child(div().text_color(rgb(MUTED)).child("Клиент порта не найден. Соберите: cargo build --release -p zomboid-client"));
                }
                col.child(div().text_color(rgb(MUTED)).child("Клиент порта пока показывает мир, играть в него нельзя: зомби, инвентарь и звук не сделаны.")).into_any_element()
            }
            1 => {
                let mut col = div().flex().flex_col().gap_3();
                col = col.child(
                    div()
                        .flex()
                        .justify_between()
                        .items_center()
                        .child(div().text_color(rgb(MUTED)).child(format!("Папка модов: {}", self.mods_dir().display())))
                        .child(button("refresh", "Обновить", false, true).on_click(cx.listener(|t, _, _, cx| {
                            t.mods_err = None;
                            t.fetch();
                            cx.notify();
                        }))),
                );
                if let Some(e) = &self.mods_err {
                    col = col.child(div().text_color(rgb(BAD)).child(format!("Каталог недоступен: {e}")));
                }
                for (i, m) in self.mods.iter().enumerate() {
                    let busy = self.busy.contains(&m.id);
                    let done = self.mods_dir().join(&m.id).exists();
                    let label = if busy { "Загрузка…" } else if done { "Переустановить" } else { "Установить" };
                    let mm = m.clone();
                    col = col.child(
                        div()
                            .p_3()
                            .rounded_lg()
                            .border_1()
                            .border_color(rgb(LINE))
                            .bg(rgb(CARD))
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .items_center()
                                    .child(div().flex().gap_2().child(div().font_weight(FontWeight::BOLD).child(m.name.clone())).child(div().text_color(rgb(MUTED)).child(format!("v{} · {}", m.version, m.author))))
                                    .child(button(("install", i), label, true, !busy).on_click(cx.listener(move |t, _, _, cx| {
                                        if t.busy.contains(&mm.id) {
                                            return;
                                        }
                                        t.busy.push(mm.id.clone());
                                        let (tx, base, dir, m) = (t.tx.clone(), t.base.clone(), t.mods_dir(), mm.clone());
                                        std::thread::spawn(move || {
                                            let r = install(&m, &base, &dir);
                                            let _ = tx.send(Msg::Installed(m.id, r));
                                        });
                                        cx.notify();
                                    }))),
                            )
                            .child(div().text_color(rgb(MUTED)).child(m.description.clone())),
                    );
                }
                col.into_any_element()
            }
            2 => {
                let mut col = div().flex().flex_col().gap_2();
                if let Some(p) = &self.progress {
                    col = col
                        .child(div().text_2xl().font_weight(FontWeight::BOLD).text_color(rgb(RUST2)).child(format!("Перенесено: {:.1}%", p.overall)))
                        .child(bar(p.overall))
                        .child(div().text_color(rgb(MUTED)).child(format!("данные от {}", p.generated)));
                    for a in &p.areas {
                        col = col
                            .child(div().mt_2().flex().justify_between().child(format!("{} ({})", a.name, if a.measured { "измерено" } else { "оценка автора" })).child(format!("{:.1}%", a.percent)))
                            .child(bar(a.percent));
                    }
                } else if let Some(e) = &self.prog_err {
                    col = col.child(div().text_color(rgb(BAD)).child(format!("Не удалось загрузить прогресс: {e}")));
                } else {
                    col = col.child("Загрузка…");
                }
                col.into_any_element()
            }
            _ => div()
                .flex()
                .flex_col()
                .gap_2()
                .child(div().text_2xl().font_weight(FontWeight::BOLD).child("DeadOxide"))
                .child("Неофициальный совместимый рантайм Project Zomboid на Rust. Нужна ваша собственная копия игры; файлы игры не распространяются.")
                .child(div().text_color(rgb(RUST2)).child(SITE))
                .child(div().text_color(rgb(RUST2)).child("https://github.com/LakoMoor/deadoxide"))
                .child(div().text_color(rgb(MUTED)).child("Моды ставятся только с проверкой sha256 из каталога."))
                .into_any_element(),
        };
        div()
            .size_full()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(LINE))
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(RUST))
                            .child("DeadOxide"),
                    )
                    .children(tabs),
            )
            .child(div().flex_1().p_4().child(body))
            .child(
                div()
                    .px_4()
                    .py_2()
                    .border_t_1()
                    .border_color(rgb(LINE))
                    .text_color(rgb(MUTED))
                    .child(if self.status.is_empty() {
                        "Готово".to_string()
                    } else {
                        self.status.clone()
                    }),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(820.), px(540.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("DeadOxide".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(Launcher::new),
        )
        .expect("open window");
        cx.activate(true);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site() -> String {
        format!(
            "file:{}/../site/",
            env!("CARGO_MANIFEST_DIR").replace(std::path::MAIN_SEPARATOR, "/")
        )
    }
    fn catalog_mod() -> Mod {
        let b = get_bytes(&format!("{}registry.json", site())).unwrap();
        serde_json::from_slice::<Registry>(&b)
            .unwrap()
            .mods
            .remove(0)
    }

    #[test]
    fn installs_the_catalog_mod_into_a_fresh_dir() {
        let dir = std::env::temp_dir().join(format!("deadoxide-test-{}", std::process::id()));
        let out = install(&catalog_mod(), &site(), &dir).unwrap();
        let lua = std::fs::read_to_string(out.join("media/lua/client/HelloDeadOxide.lua")).unwrap();
        assert!(lua.contains("mod loaded"));
        assert!(out.join("mod.info").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wrong_sha256_installs_nothing() {
        let dir = std::env::temp_dir().join(format!("deadoxide-test-bad-{}", std::process::id()));
        let mut m = catalog_mod();
        m.sha256 = "0".repeat(64);
        assert!(install(&m, &site(), &dir).unwrap_err().contains("sha256"));
        assert!(!dir.exists());
    }
}
