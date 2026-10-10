//! The anti-touch lock driven through a whole app frame, the way a finger
//! drives it: the input goes through the app's own `raw_input_hook`, so what is
//! tested is the wiring as well as the filter.

use std::cell::RefCell;
use std::rc::Rc;

use eframe::App;
use eframe::egui;
use sdroxide_types::{Command, RadioController, RadioEvent};

use super::SdroxideApp;

/// Keeps every command the app sends.
struct Sent(Rc<RefCell<Vec<Command>>>);

impl RadioController for Sent {
    fn send(&mut self, cmd: Command) {
        self.0.borrow_mut().push(cmd);
    }
    fn poll_event(&mut self) -> Option<RadioEvent> {
        None
    }
}

struct Rig {
    ctx: egui::Context,
    app: SdroxideApp,
    sent: Rc<RefCell<Vec<Command>>>,
    t: f64,
    size: egui::Vec2,
}

impl Rig {
    fn frame(&mut self, events: Vec<egui::Event>, dt: f64) -> egui::FullOutput {
        self.t += dt;
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.size)),
            time: Some(self.t),
            events,
            ..Default::default()
        };
        self.app.raw_input_hook(&self.ctx, &mut input);
        let app = &mut self.app;
        let out = self.ctx.run_ui(input, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest()));
        out
    }

    fn button(&mut self, at: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// A tap: down and up a frame apart, the way a touch screen delivers one.
    fn tap(&mut self, at: egui::Pos2) {
        let down = self.button(at, true);
        self.frame(vec![egui::Event::PointerMoved(at), down], 0.05).drop_without_applying_deltas();
        let up = self.button(at, false);
        self.frame(vec![up], 0.05).drop_without_applying_deltas();
        self.frame(vec![egui::Event::PointerGone], 0.05).drop_without_applying_deltas();
    }

    fn retunes(&self) -> usize {
        self.sent.borrow().iter().filter(|c| matches!(c, Command::SetVfo { .. })).count()
    }
}

/// Where a label is drawn, by its text.
fn find_text(shapes: &[egui::epaint::ClippedShape], text: &str) -> Option<egui::Pos2> {
    fn walk(s: &egui::Shape, text: &str) -> Option<egui::Pos2> {
        match s {
            egui::Shape::Text(t) if t.galley.text() == text => {
                Some(t.pos + t.galley.rect.center().to_vec2())
            }
            egui::Shape::Vec(v) => v.iter().find_map(|s| walk(s, text)),
            _ => None,
        }
    }
    shapes.iter().find_map(|c| walk(&c.shape, text))
}

/// The request: once locked, a finger on the screen must not move the dial —
/// and a 3 s hold on the padlock, and only that, gives the screen back.
#[test]
fn a_locked_screen_does_not_tune_and_a_three_second_hold_unlocks_it() {
    let _guard = crate::multi::frame_test_lock();
    let dir = std::env::temp_dir().join(format!("sdroxide-touch-lock-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

    let sent = Rc::new(RefCell::new(Vec::new()));
    let ctx = egui::Context::default();
    let app = SdroxideApp::new_tab(&ctx, None, None, Box::new(Sent(sent.clone())), 0, true);
    // A tablet, the tier the padlock was asked for.
    let size = egui::vec2(1000.0, 900.0);
    assert_eq!(
        crate::layout::tier_for(size, sdroxide_types::LayoutMode::Auto),
        crate::layout::Tier::Tablet
    );
    let mut rig = Rig { ctx: ctx.clone(), app, sent, t: 0.0, size };

    // A few frames for the layout to settle: the first passes size things.
    for _ in 0..3 {
        rig.frame(Vec::new(), 0.05).drop_without_applying_deltas();
    }
    let out = rig.frame(Vec::new(), 0.05);
    let plus = find_text(&out.shapes, "+").expect("the tune-step row's + is on the strip");
    out.drop_without_applying_deltas();
    let lock = crate::touch_lock::padlock_rects(&ctx)[0].center();

    // Unlocked, + tunes: the control works, so the rest of the test means something.
    rig.tap(plus);
    assert_eq!(rig.retunes(), 1, "+ did not tune on an unlocked screen");

    rig.tap(lock);
    assert!(crate::touch_lock::is_locked(&ctx), "a tap on the padlock did not lock");
    rig.tap(plus);
    assert_eq!(rig.retunes(), 1, "+ tuned through the lock");

    // A hold that lifts short of 3 s leaves the screen locked.
    let down = rig.button(lock, true);
    rig.frame(vec![egui::Event::PointerMoved(lock), down], 0.05).drop_without_applying_deltas();
    for _ in 0..10 {
        rig.frame(Vec::new(), 0.2).drop_without_applying_deltas();
    }
    let up = rig.button(lock, false);
    rig.frame(vec![up], 0.05).drop_without_applying_deltas();
    assert!(crate::touch_lock::is_locked(&ctx), "a 2 s hold unlocked");

    // The whole 3 s does.
    let down = rig.button(lock, true);
    rig.frame(vec![egui::Event::PointerMoved(lock), down], 0.05).drop_without_applying_deltas();
    for _ in 0..16 {
        rig.frame(Vec::new(), 0.2).drop_without_applying_deltas();
    }
    assert!(!crate::touch_lock::is_locked(&ctx), "a 3 s hold did not unlock");
    // ...and the release that ends it does not lock again.
    let up = rig.button(lock, false);
    rig.frame(vec![up], 0.05).drop_without_applying_deltas();
    rig.frame(vec![egui::Event::PointerGone], 0.05).drop_without_applying_deltas();
    assert!(!crate::touch_lock::is_locked(&ctx), "lifting off the hold locked again");

    rig.tap(plus);
    assert_eq!(rig.retunes(), 2, "+ did not tune once unlocked");
}
