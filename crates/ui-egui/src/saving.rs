//! The save under way, as a small bar at the bottom right (where the update's download shows): a big project takes
//! seconds to write, and the window says so instead of seeming stuck. The kernel writes the bodies in one go, so how
//! far along it is comes from how long this project's last save took (or from its size the first time).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use egui::{Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use shapecraft_doc::Document;

use crate::theme::{self, Tokens, radius};

/// A save shorter than this comes and goes unseen.
const SHOW_AFTER: Duration = Duration::from_millis(350);
/// How long "Saved" stays once a bar that showed has finished.
const SAVED_FOR: Duration = Duration::from_millis(1400);
/// The kernel writes about this many faces a second (Apple M-series; slower machines only make the bar wait at the end).
const FACES_A_SECOND: f32 = 3400.0;

/// What to do once the project is saved: closing it, or leaving it for something else, waits for the bar.
#[derive(Clone)]
pub enum After {
    Close,
    /// The home screen's Get Started or Tutorials page.
    Section(crate::landing::Section),
    New(Option<PathBuf>),
    Open(PathBuf),
    Quit,
    InstallUpdate {
        restart: bool,
    },
}

/// The save bar's own state, beside the save itself.
#[derive(Default)]
pub struct SaveBar {
    /// How long each project's last save took.
    took: Vec<(PathBuf, f32)>,
    /// When a bar that showed finished.
    saved_at: Option<Instant>,
}

impl SaveBar {
    /// How long saving this document should take, in seconds.
    pub fn expected(&self, doc: &Document, dir: &Path) -> f32 {
        if let Some((_, s)) = self.took.iter().find(|(d, _)| d == dir) {
            return s.max(0.05);
        }
        let faces: usize =
            doc.bodies.iter().filter(|b| !b.is_mesh()).map(|b| b.mesh.triangle_faces.iter().max().map_or(0, |m| *m as usize + 1)).sum();
        (faces as f32 / FACES_A_SECOND).max(0.5)
    }

    /// A save ended well after `took`.
    pub fn done(&mut self, dir: &Path, took: Duration) {
        self.took.retain(|(d, _)| d != dir);
        // The projects of a session: a handful. The oldest goes past that.
        if self.took.len() >= 32 {
            self.took.remove(0);
        }
        self.took.push((dir.to_path_buf(), took.as_secs_f32()));
        if took >= SHOW_AFTER {
            self.saved_at = Some(Instant::now());
        }
    }

    /// The bar. `running`: when the save began and how long it should take. `leaving`: the project closes once it's
    /// saved, so the window waits for it. `lift`: how far up the corner is taken (the update's pill).
    pub fn show(&mut self, ui: &mut Ui, full: Rect, running: Option<(Instant, f32)>, leaving: bool, lift: f32) {
        let line = match running {
            Some((started, expected)) => {
                let ago = started.elapsed();
                if ago < SHOW_AFTER && !leaving {
                    ui.ctx().request_repaint_after(SHOW_AFTER - ago);
                    return;
                }
                ui.ctx().request_repaint_after(Duration::from_millis(33));
                Some((fraction(ago.as_secs_f32(), expected), false))
            }
            None => match self.saved_at {
                Some(at) if at.elapsed() < SAVED_FOR => {
                    ui.ctx().request_repaint_after(SAVED_FOR - at.elapsed());
                    Some((1.0, true))
                }
                _ => {
                    self.saved_at = None;
                    None
                }
            },
        };
        let Some((f, saved)) = line else { return };
        if leaving && !saved {
            // The project is on its way out: nothing more can be changed in it, and the window says it's waiting.
            egui::Area::new(ui.scope_id().with("save-veil")).order(egui::Order::Foreground).fixed_pos(full.min).show(ui.ctx(), |ui| {
                let r = ui.allocate_rect(full, Sense::click_and_drag());
                ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha(70));
                r.on_hover_cursor(egui::CursorIcon::Progress);
            });
        }
        let order = if leaving { egui::Order::Tooltip } else { egui::Order::Middle };
        egui::Area::new(ui.scope_id().with("save-bar")).order(order).interactable(false).fixed_pos(full.min).show(ui.ctx(), |ui| {
            let p = ui.painter().clone();
            let text = if saved {
                crate::i18n::tr("Saved").to_string()
            } else {
                let what = if leaving { "Saving before closing…" } else { "Saving…" };
                format!("{} {}%", crate::i18n::tr(what), (f * 100.0).floor() as u32)
            };
            let text = p.layout_no_wrap(text, theme::body(11.5), Tokens::text_strong());
            let w = (30.0 + text.size().x + 18.0).max(190.0).min(full.width() - 32.0);
            let bottom = full.bottom() - 16.0 - lift;
            let r = Rect::from_min_max(pos2(full.right() - 16.0 - w, bottom - 38.0), pos2(full.right() - 16.0, bottom));
            p.rect_filled(r.translate(vec2(0.0, 2.0)), radius(14), Color32::from_black_alpha(50));
            p.rect_filled(r, radius(14), Tokens::panel());
            p.rect_stroke(r, radius(14), Stroke::new(1.0, Tokens::panel_border()), StrokeKind::Inside);
            // A small disc turning while it's written, a tick once it is.
            let mark = pos2(r.left() + 18.0, r.top() + 15.0);
            if saved {
                let pen = Stroke::new(1.8, theme::hex(0x34c759));
                p.line_segment([mark + vec2(-4.5, 0.5), mark + vec2(-1.2, 3.8)], pen);
                p.line_segment([mark + vec2(-1.2, 3.8), mark + vec2(5.0, -3.8)], pen);
            } else {
                let turn = ui.input(|i| i.time) as f32 * 4.0;
                let arc = (0..=14).map(|i| {
                    let a = turn + i as f32 / 14.0 * 4.4;
                    mark + vec2(a.cos(), a.sin()) * 5.0
                });
                p.add(egui::Shape::line(arc.collect(), Stroke::new(1.6, Tokens::accent())));
            }
            p.galley(pos2(r.left() + 32.0, mark.y - text.size().y / 2.0), text, Tokens::text_strong());
            let track = Rect::from_min_max(pos2(r.left() + 14.0, r.bottom() - 10.0), pos2(r.right() - 14.0, r.bottom() - 6.0));
            p.rect_filled(track, radius(2), Tokens::button_active());
            let mut fill = track;
            fill.set_width(track.width() * f.clamp(0.03, 1.0));
            p.rect_filled(fill, radius(2), if saved { theme::hex(0x34c759) } else { Tokens::accent() });
        });
    }
}

/// How far along a save that should take `expected` seconds is after `t`: on time up to nine tenths, then slower and
/// slower (it never says it's done before it is).
fn fraction(t: f32, expected: f32) -> f32 {
    let r = t / expected.max(0.05);
    if r < 0.9 { r.max(0.0) } else { 0.9 + 0.09 * (1.0 - (-(r - 0.9)).exp()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_only_moves_forward_and_never_fills_early() {
        let mut last = -1.0;
        for i in 0..2000 {
            let f = fraction(i as f32 * 0.05, 8.0);
            assert!(f >= last && f < 1.0, "{f} after {last}");
            last = f;
        }
        assert!(last > 0.98);
        assert_eq!(fraction(f32::NAN, 1.0).max(0.0), fraction(f32::NAN, 1.0).max(0.0));
    }

    #[test]
    fn the_next_save_expects_what_the_last_took() {
        let mut bar = SaveBar::default();
        let (doc, dir) = (Document::new("Big"), Path::new("/tmp/big"));
        assert!(bar.expected(&doc, dir) <= 0.5);
        bar.done(dir, Duration::from_secs(8));
        assert_eq!(bar.expected(&doc, dir), 8.0);
        assert!(bar.saved_at.is_some());
    }
}
