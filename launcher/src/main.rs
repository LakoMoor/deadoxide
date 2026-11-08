#![recursion_limit = "1024"]
//! DeadOxide launcher on GPUI Fast, laid out like Horizon's: an instance sidebar, a top nav (Home / Mods / Progress / Patch notes),
//! a hero with Play, and an overview with screenshots and the patch log.
//! Play starts the Rust client (read-only over the player's own install) or the original game with an isolated `-cachedir`.
use deadoxide_launcher::*;
use gpui::{prelude::*, *};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{mpsc, Arc};
use std::time::Duration;

const BG: u32 = 0x0b0908;
const SIDE: u32 = 0x120d0b;
const CARD: u32 = 0x18110e;
const LINE: u32 = 0x2b1f18;
const TEXT: u32 = 0xf0e8df;
const MUTED: u32 = 0x9d8f82;
const RUST: u32 = 0xe8692a;
const RUST2: u32 = 0xff9a55;
const BAD: u32 = 0xe07070;
const OK: u32 = 0x6fcf85;
const VERSION: &str = env!("CARGO_PKG_VERSION");

const ICON: &[u8] = include_bytes!("../../assets/icon-256.png");
const SHOTS: [(&str, &[u8]); 3] = [
    (
        "Real game: the living room at spawn",
        include_bytes!("../../site/assets/gallery/game_clean2.png"),
    ),
    (
        "Port: the same frame",
        include_bytes!("../../site/assets/gallery/port_lit_room.png"),
    ),
    (
        "Port: cell 41_39 on the GPU",
        include_bytes!("../../site/assets/gallery/port_gpu_cell41_39.png"),
    ),
];

enum Msg {
    Registry(Result<Vec<Mod>, String>),
    Progress(Result<Progress, String>),
    Changelog(Result<Vec<Release>, String>),
    Installed(String, Result<PathBuf, String>),
}

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Home,
    Mods,
    Progress,
    Notes,
}
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Port,
    Original,
}

struct Launcher {
    page: Page,
    mode: Mode,
    game: Option<PathBuf>,
    profile: PathBuf,
    client: Option<PathBuf>,
    child: Option<Child>,
    status: String,
    mods: Vec<Mod>,
    mods_err: Option<String>,
    progress: Option<Progress>,
    prog_err: Option<String>,
    notes: Vec<Release>,
    busy: Vec<String>,
    base: String,
    icon: Arc<Image>,
    shots: Vec<Arc<Image>>,
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
        let page = match std::env::var("DEADOXIDE_TAB").ok().as_deref() {
            Some("1") => Page::Mods,
            Some("2") => Page::Progress,
            Some("3") => Page::Notes,
            _ => Page::Home,
        };
        let me = Launcher {
            page,
            mode: Mode::Port,
            game,
            profile,
            client: find_client(),
            child: None,
            status: String::new(),
            mods: vec![],
            mods_err: None,
            progress: None,
            prog_err: None,
            notes: vec![],
            busy: vec![],
            base,
            icon: Arc::new(Image::from_bytes(ImageFormat::Png, ICON.to_vec())),
            shots: SHOTS
                .iter()
                .map(|(_, b)| Arc::new(Image::from_bytes(ImageFormat::Png, b.to_vec())))
                .collect(),
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
            let c = get_bytes(&format!("{base}changelog.json"))
                .and_then(|b| serde_json::from_slice::<Changelog>(&b).map_err(|e| e.to_string()))
                .map(|c| c.releases);
            let _ = tx.send(Msg::Changelog(c));
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
                Msg::Changelog(Ok(v)) => self.notes = v,
                Msg::Changelog(Err(_)) => {}
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
    fn can_play(&self) -> bool {
        self.child.is_none()
            && self.game.is_some()
            && match self.mode {
                Mode::Port => self.client.is_some(),
                Mode::Original => true,
            }
    }
    fn play(&mut self) {
        if !self.can_play() {
            return;
        }
        match self.mode {
            Mode::Port => self.play_port(),
            Mode::Original => self.play_original(),
        }
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
    fn open_profile(&mut self) {
        let _ = std::fs::create_dir_all(&self.profile);
        self.status = match Command::new("explorer").arg(&self.profile).spawn() {
            Ok(_) => format!("Opened {}", self.profile.display()),
            Err(e) => format!("Could not open the folder: {e}"),
        };
    }
}

fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    primary: bool,
    enabled: bool,
) -> Stateful<Div> {
    let (bg, fg) = if !enabled {
        (0x1c1410, 0x6b5f54)
    } else if primary {
        (RUST, 0x1a0e07)
    } else {
        (CARD, TEXT)
    };
    let b = div()
        .id(id)
        .px_5()
        .py_3()
        .rounded_full()
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
    div().h(px(8.)).w_full().rounded_full().bg(rgb(LINE)).child(
        div()
            .h_full()
            .w(relative((pct / 100.0).clamp(0.0, 1.0)))
            .rounded_full()
            .bg(rgb(RUST)),
    )
}

fn panel() -> Div {
    div()
        .rounded_xl()
        .border_1()
        .border_color(rgb(LINE))
        .bg(rgb(CARD))
}

impl Render for Launcher {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (dot, state) = match (&self.game, &self.client) {
            (None, _) => (BAD, "Game not found"),
            (Some(_), None) => (RUST2, "Client not built"),
            _ if self.child.is_some() => (OK, "Running"),
            _ => (OK, "Ready"),
        };

        // ---- sidebar: instances, like Horizon's ----
        let sidebar = div()
            .w(px(250.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .bg(rgb(SIDE))
            .border_r_1()
            .border_color(rgb(LINE))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(div().text_color(rgb(MUTED)).child("Instances")),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .p_3()
                    .rounded_lg()
                    .bg(rgb(CARD))
                    .border_1()
                    .border_color(rgb(LINE))
                    .child(img(self.icon.clone()).size(px(38.)).rounded_lg())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(div().font_weight(FontWeight::BOLD).child("Project Zomboid"))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .text_sm()
                                    .text_color(rgb(MUTED))
                                    .child(div().size(px(8.)).rounded_full().bg(rgb(dot)))
                                    .child(state),
                            ),
                    ),
            )
            .child(div().flex_1())
            .child(
                panel()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::BOLD)
                            .child("Own your copy"),
                    )
                    .child(div().text_sm().text_color(rgb(MUTED)).child(
                        "DeadOxide reads your Project Zomboid install. No game files are shipped.",
                    )),
            )
            .child(
                div()
                    .text_color(rgb(MUTED))
                    .child("Source: LakoMoor/deadoxide"),
            )
            .child(
                div()
                    .text_color(rgb(MUTED))
                    .child("Website: lakomoor.github.io/deadoxide"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(MUTED))
                    .child(format!("Launcher {VERSION}")),
            );

        // ---- top nav ----
        let nav = [
            (Page::Home, "Home"),
            (Page::Mods, "Mods"),
            (Page::Progress, "Progress"),
            (Page::Notes, "Patch notes"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (p, n))| {
            let on = self.page == p;
            div()
                .id(("nav", i))
                .px_4()
                .py_2()
                .rounded_lg()
                .cursor_pointer()
                .text_color(rgb(if on { TEXT } else { MUTED }))
                .bg(rgb(if on { LINE } else { CARD }))
                .child(n)
                .on_click(cx.listener(move |t, _, _, cx| {
                    t.page = p;
                    cx.notify();
                }))
        });
        let topbar = div()
            .flex()
            .items_center()
            .gap_1()
            .p_1()
            .rounded_xl()
            .bg(rgb(CARD))
            .children(nav);

        // ---- pages ----
        let body: AnyElement = match self.page {
            Page::Home => {
                let steps = [
                    (
                        "01",
                        "Game install",
                        match &self.game {
                            Some(g) => g.display().to_string(),
                            None => "Not found. Set PZ_ROOT to your Project Zomboid folder.".into(),
                        },
                        self.game.is_some(),
                    ),
                    (
                        "02",
                        "Port client",
                        match &self.client {
                            Some(c) => c.display().to_string(),
                            None => {
                                "Not built. Run: cargo build --release -p zomboid-client".into()
                            }
                        },
                        self.client.is_some(),
                    ),
                    (
                        "03",
                        "Profile",
                        format!(
                            "{} (kept apart from your own saves)",
                            self.profile.display()
                        ),
                        true,
                    ),
                ];
                let rows = steps.into_iter().map(|(n, t, d, ok)| {
                    div()
                        .flex()
                        .items_center()
                        .gap_4()
                        .py_3()
                        .border_t_1()
                        .border_color(rgb(LINE))
                        .child(div().w(px(28.)).text_color(rgb(MUTED)).child(n))
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .child(div().font_weight(FontWeight::SEMIBOLD).child(t))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(rgb(if ok { MUTED } else { BAD }))
                                        .child(d),
                                ),
                        )
                });
                let mode = |id: &'static str,
                            label: &'static str,
                            m: Mode,
                            cur: Mode,
                            cx: &mut Context<Self>| {
                    div()
                        .id(id)
                        .px_4()
                        .py_2()
                        .rounded_full()
                        .cursor_pointer()
                        .bg(rgb(if cur == m { TEXT } else { CARD }))
                        .text_color(rgb(if cur == m { 0x1a0e07 } else { MUTED }))
                        .child(label)
                        .on_click(cx.listener(move |t, _, _, cx| {
                            t.mode = m;
                            cx.notify();
                        }))
                };
                let play_label = if self.child.is_some() { "Stop" } else { "Play" };
                let hero = div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .rounded_lg()
                                    .bg(rgb(CARD))
                                    .border_1()
                                    .border_color(rgb(LINE))
                                    .child("Rust Edition"),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .text_color(rgb(MUTED))
                                    .child("Java Edition"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_4()
                            .child(img(self.icon.clone()).size(px(64.)).rounded_xl())
                            .child(
                                div()
                                    .text_size(px(54.))
                                    .font_weight(FontWeight::BOLD)
                                    .child("DeadOxide"),
                            ),
                    )
                    .child(
                        div()
                            .text_color(rgb(MUTED))
                            .child(match self.progress.as_ref() {
                                Some(p) => format!(
                                    "Project Zomboid in Rust. {:.1}% ported; not playable yet.",
                                    p.overall
                                ),
                                None => "Project Zomboid in Rust.".to_string(),
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .mt_2()
                            .child(
                                button(
                                    "play",
                                    play_label,
                                    true,
                                    self.can_play() || self.child.is_some(),
                                )
                                .px_8()
                                .on_click(cx.listener(
                                    |t, _, _, cx| {
                                        if t.child.is_some() {
                                            t.stop();
                                            t.status = "Stopped".into();
                                        } else {
                                            t.play();
                                        }
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .p_1()
                                    .rounded_full()
                                    .bg(rgb(CARD))
                                    .border_1()
                                    .border_color(rgb(LINE))
                                    .child(mode(
                                        "mode-port",
                                        "Port client",
                                        Mode::Port,
                                        self.mode,
                                        cx,
                                    ))
                                    .child(mode(
                                        "mode-orig",
                                        "Original game",
                                        Mode::Original,
                                        self.mode,
                                        cx,
                                    )),
                            )
                            .child(
                                button("folder", "Open profile folder", false, true).on_click(
                                    cx.listener(|t, _, _, cx| {
                                        t.open_profile();
                                        cx.notify();
                                    }),
                                ),
                            ),
                    );
                let shots =
                    div()
                        .flex()
                        .w_full()
                        .gap_3()
                        .children(self.shots.iter().enumerate().map(|(i, s)| {
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    img(s.clone())
                                        .w_full()
                                        .h(px(150.))
                                        .object_fit(ObjectFit::Cover)
                                        .rounded_lg(),
                                )
                                .child(div().text_sm().text_color(rgb(MUTED)).child(SHOTS[i].0))
                        }));
                let latest = self.notes.first().cloned();
                let log = panel()
                    .w(px(300.))
                    .flex_none()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::BOLD)
                                    .child("Patch log"),
                            )
                            .child(div().text_sm().text_color(rgb(MUTED)).child("All notes")),
                    )
                    .children(latest.into_iter().flat_map(|r| {
                        let mut v: Vec<AnyElement> = vec![div()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(format!("v{} · {}", r.version, r.date))
                            .into_any_element()];
                        if !r.new.is_empty() {
                            v.push(
                                div()
                                    .mt_1()
                                    .text_sm()
                                    .text_color(rgb(MUTED))
                                    .child("New")
                                    .into_any_element(),
                            );
                            v.extend(r.new.iter().map(|s| {
                                div().text_sm().child(format!("• {s}")).into_any_element()
                            }));
                        }
                        if !r.fixed.is_empty() {
                            v.push(
                                div()
                                    .mt_1()
                                    .text_sm()
                                    .text_color(rgb(MUTED))
                                    .child("Fixed")
                                    .into_any_element(),
                            );
                            v.extend(r.fixed.iter().map(|s| {
                                div().text_sm().child(format!("• {s}")).into_any_element()
                            }));
                        }
                        v
                    }));
                div()
                    .flex()
                    .flex_col()
                    .gap_5()
                    .child(hero)
                    .child(
                        panel()
                            .px_5()
                            .pb_2()
                            .child(div().flex().flex_col().children(rows)),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(
                                panel()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .p_4()
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    .child(
                                        div()
                                            .text_lg()
                                            .font_weight(FontWeight::BOLD)
                                            .child("Screenshots"),
                                    )
                                    .child(shots),
                            )
                            .child(log),
                    )
                    .into_any_element()
            }
            Page::Mods => {
                let mut col = div().flex().flex_col().gap_3();
                col = col.child(
                    div()
                        .flex()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(div().text_2xl().font_weight(FontWeight::BOLD).child("Mods"))
                                .child(div().text_sm().text_color(rgb(MUTED)).child(format!(
                                    "Installed into {}",
                                    self.mods_dir().display()
                                ))),
                        )
                        .child(
                            button("refresh", "Refresh", false, true).on_click(cx.listener(
                                |t, _, _, cx| {
                                    t.mods_err = None;
                                    t.fetch();
                                    cx.notify();
                                },
                            )),
                        ),
                );
                if let Some(e) = &self.mods_err {
                    col = col.child(
                        div()
                            .text_color(rgb(BAD))
                            .child(format!("Catalog unavailable: {e}")),
                    );
                }
                for (i, m) in self.mods.iter().enumerate() {
                    let busy = self.busy.contains(&m.id);
                    let done = self.mods_dir().join(&m.id).exists();
                    let label = if busy {
                        "Downloading…"
                    } else if done {
                        "Reinstall"
                    } else {
                        "Install"
                    };
                    let mm = m.clone();
                    col = col.child(
                        panel()
                            .p_4()
                            .flex()
                            .items_center()
                            .gap_4()
                            .child(
                                div()
                                    .size(px(48.))
                                    .rounded_lg()
                                    .bg(rgb(LINE))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_color(rgb(RUST2))
                                    .font_weight(FontWeight::BOLD)
                                    .child(m.name.chars().next().unwrap_or('M').to_string()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .flex()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::BOLD)
                                                    .child(m.name.clone()),
                                            )
                                            .child(
                                                div().text_color(rgb(MUTED)).child(format!(
                                                    "v{} · {}",
                                                    m.version, m.author
                                                )),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(rgb(MUTED))
                                            .child(m.description.clone()),
                                    ),
                            )
                            .child(button(("install", i), label, true, !busy).on_click(
                                cx.listener(move |t, _, _, cx| {
                                    if t.busy.contains(&mm.id) {
                                        return;
                                    }
                                    t.busy.push(mm.id.clone());
                                    let (tx, base, dir, m) =
                                        (t.tx.clone(), t.base.clone(), t.mods_dir(), mm.clone());
                                    std::thread::spawn(move || {
                                        let r = install(&m, &base, &dir);
                                        let _ = tx.send(Msg::Installed(m.id, r));
                                    });
                                    cx.notify();
                                }),
                            )),
                    );
                }
                col.into_any_element()
            }
            Page::Progress => {
                let mut col = div().flex().flex_col().gap_2();
                if let Some(p) = &self.progress {
                    col = col
                        .child(
                            div()
                                .text_2xl()
                                .font_weight(FontWeight::BOLD)
                                .child("Progress"),
                        )
                        .child(
                            div()
                                .text_size(px(48.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(RUST2))
                                .child(format!("{:.1}% ported", p.overall)),
                        )
                        .child(bar(p.overall))
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(MUTED))
                                .child(format!("data from {}", p.generated)),
                        );
                    for a in &p.areas {
                        col = col
                            .child(
                                div()
                                    .mt_2()
                                    .flex()
                                    .justify_between()
                                    .child(format!(
                                        "{} ({})",
                                        a.name,
                                        if a.measured {
                                            "measured"
                                        } else {
                                            "author estimate"
                                        }
                                    ))
                                    .child(format!("{:.1}%", a.percent)),
                            )
                            .child(bar(a.percent));
                    }
                } else if let Some(e) = &self.prog_err {
                    col = col.child(
                        div()
                            .text_color(rgb(BAD))
                            .child(format!("Could not load progress: {e}")),
                    );
                } else {
                    col = col.child("Loading…");
                }
                col.into_any_element()
            }
            Page::Notes => {
                let mut col = div().flex().flex_col().gap_3().child(
                    div()
                        .text_2xl()
                        .font_weight(FontWeight::BOLD)
                        .child("Patch notes"),
                );
                for r in &self.notes {
                    let mut p = panel().p_4().flex().flex_col().gap_1().child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .child(format!("v{} · {}", r.version, r.date)),
                    );
                    for s in &r.new {
                        p = p.child(div().child(format!("New: {s}")));
                    }
                    for s in &r.fixed {
                        p = p.child(div().text_color(rgb(MUTED)).child(format!("Fixed: {s}")));
                    }
                    col = col.child(p);
                }
                col.into_any_element()
            }
        };

        let status = if self.status.is_empty() {
            "Ready".to_string()
        } else {
            self.status.clone()
        };
        div()
            .size_full()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .flex()
            .child(sidebar)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_6()
                            .py_4()
                            .child(topbar)
                            .child(div().text_sm().text_color(rgb(MUTED)).child(status)),
                    )
                    .child(
                        div()
                            .id("scroll")
                            .flex_1()
                            .min_w_0()
                            .w_full()
                            .overflow_y_scroll()
                            .px_6()
                            .pb_6()
                            .child(div().w_full().child(body)),
                    ),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1180.), px(780.)), cx);
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
