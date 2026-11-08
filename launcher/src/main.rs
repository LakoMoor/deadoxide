#![recursion_limit = "1024"]
//! DeadOxide launcher on GPUI Fast (the same UI stack as Horizon's launcher). Tabs: Play / Mods / Progress / About.
//! Play starts the Rust client (read-only over the player's own install) or the original game with an isolated `-cachedir`.
//! Mods come from a registry (`registry.json`): download -> verify sha256 -> extract into `<profile>/mods`.
use gpui::{prelude::*, *};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::mpsc;
use std::time::Duration;

use deadoxide_launcher::*;
const BG: u32 = 0x0e0b0a;
const CARD: u32 = 0x1d1713;
const LINE: u32 = 0x34281f;
const TEXT: u32 = 0xece3da;
const MUTED: u32 = 0xa89a8c;
const RUST: u32 = 0xe8692a;
const RUST2: u32 = 0xff9a55;
const BAD: u32 = 0xe07070;

enum Msg {
    Registry(Result<Vec<Mod>, String>),
    Progress(Result<Progress, String>),
    Installed(String, Result<PathBuf, String>),
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
                        Ok(p) => format!("Mod {id} installed: {}", p.display()),
                        Err(e) => format!("Mod {id}: error: {e}"),
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
                "Port client started".into()
            }
            Err(e) => format!("Could not start the client: {e}"),
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
                "Original game started with a separate profile".into()
            }
            Err(e) => format!("Could not start the game: {e}"),
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
        let tabs = ["Play", "Mods", "Progress", "About"]
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
                    None => div().text_color(rgb(BAD)).child("Project Zomboid install not found. Set the PZ_ROOT environment variable."),
                });
                col = col.child(div().text_color(rgb(MUTED)).child(format!("Profile (separate from your own saves): {}", self.profile.display())));
                let mut row = div().flex().gap_3().child(button("play-port", "Play the port client", true, can_port).on_click(cx.listener(|t, _, _, cx| {
                    if t.game.is_some() && t.client.is_some() && t.child.is_none() {
                        t.play_port();
                        cx.notify();
                    }
                })));
                row = row.child(button("play-orig", "Play the original (separate profile)", false, can_orig).on_click(cx.listener(|t, _, _, cx| {
                    if t.game.is_some() && t.child.is_none() {
                        t.play_original();
                        cx.notify();
                    }
                })));
                if running {
                    row = row.child(button("stop", "Stop", false, true).on_click(cx.listener(|t, _, _, cx| {
                        t.stop();
                        t.status = "Stopped".into();
                        cx.notify();
                    })));
                }
                col = col.child(row);
                if self.client.is_none() {
                    col = col.child(div().text_color(rgb(MUTED)).child("Port client not found. Build it: cargo build --release -p zomboid-client"));
                }
                col.child(div().text_color(rgb(MUTED)).child("The port client only shows the world for now; it is not playable: zombies, inventory and sound are not done.")).into_any_element()
            }
            1 => {
                let mut col = div().flex().flex_col().gap_3();
                col = col.child(
                    div()
                        .flex()
                        .justify_between()
                        .items_center()
                        .child(div().text_color(rgb(MUTED)).child(format!("Mods folder: {}", self.mods_dir().display())))
                        .child(button("refresh", "Refresh", false, true).on_click(cx.listener(|t, _, _, cx| {
                            t.mods_err = None;
                            t.fetch();
                            cx.notify();
                        }))),
                );
                if let Some(e) = &self.mods_err {
                    col = col.child(div().text_color(rgb(BAD)).child(format!("Catalog unavailable: {e}")));
                }
                for (i, m) in self.mods.iter().enumerate() {
                    let busy = self.busy.contains(&m.id);
                    let done = self.mods_dir().join(&m.id).exists();
                    let label = if busy { "Downloading…" } else if done { "Reinstall" } else { "Install" };
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
                        .child(div().text_2xl().font_weight(FontWeight::BOLD).text_color(rgb(RUST2)).child(format!("Ported: {:.1}%", p.overall)))
                        .child(bar(p.overall))
                        .child(div().text_color(rgb(MUTED)).child(format!("data from {}", p.generated)));
                    for a in &p.areas {
                        col = col
                            .child(div().mt_2().flex().justify_between().child(format!("{} ({})", a.name, if a.measured { "measured" } else { "author estimate" })).child(format!("{:.1}%", a.percent)))
                            .child(bar(a.percent));
                    }
                } else if let Some(e) = &self.prog_err {
                    col = col.child(div().text_color(rgb(BAD)).child(format!("Could not load progress: {e}")));
                } else {
                    col = col.child("Loading…");
                }
                col.into_any_element()
            }
            _ => div()
                .flex()
                .flex_col()
                .gap_2()
                .child(div().text_2xl().font_weight(FontWeight::BOLD).child("DeadOxide"))
                .child("Unofficial compatible runtime for Project Zomboid in Rust. You need your own copy of the game; no game files are distributed.")
                .child(div().text_color(rgb(RUST2)).child(SITE))
                .child(div().text_color(rgb(RUST2)).child("https://github.com/LakoMoor/deadoxide"))
                .child(div().text_color(rgb(MUTED)).child("Mods are installed only after a sha256 check against the catalog."))
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
                        "Ready".to_string()
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
