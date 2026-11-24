//! In-app tutorials: a few short steps, each starting from its own prepared scene so a mistake never carries on, with
//! only the tools the step needs, an instruction that moves on as you go, and Continue once the step's goal is met.
//!
//! Three beginner courses, then one for each tool worth its own, all on things found around the house. The models and
//! the words are ours.
//!
//! **Everything can be done by clicking.** Every instruction names the one gesture that meets it ([`Show`]): the
//! workspace plays it with an animated pointer, and `shapecraft-snap`'s `play_tutorial` performs the same gesture with
//! real pointer events, so an instruction nobody could follow fails there. Sizes are typed or taken from the
//! instruction's own button; nothing needs a key.
//!
//! Every step also carries a scripted solution: the tests run it to prove the step can be finished, the workspace plays
//! it in the corner (where the step is headed), and Do It for Me runs it.

use std::collections::HashMap;

use glam::DVec3;
use serde_json::Value;
use shapecraft_engine::{SelectionKind, Session};

/// One engine command: its id and its JSON params. String values `"$sketch"`, `"$body"`, `"$face"` (the last ones
/// made) and `"$ref"` (the first body made) are filled in as it runs.
///
/// A sketch made from an origin and a normal (+Z) runs u along -Y and v along +X, so its points are given that way
/// (or it names its `x_dir`); sketches opened on a top face read like the Top view instead (u = +X).
pub type Cmd = (&'static str, &'static str);

/// A point of the model, in mm.
pub type Spot = [f64; 3];

/// The two parts of the Tutorials page.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Course {
    /// For someone who has never modeled anything, in order.
    Beginner,
    /// One tool, for someone who knows the basics.
    Tool,
}

pub struct Lesson {
    pub id: &'static str,
    pub course: Course,
    /// The card's and the window's title.
    pub title: &'static str,
    /// What it teaches, under the title on its card.
    pub about: &'static str,
    pub minutes: u32,
    pub steps: &'static [Step],
    /// Builds the finished part for the card picture.
    pub finished: &'static [Cmd],
}

pub struct Step {
    /// What the step is about, shown large while it starts.
    pub intro: &'static str,
    pub setup: &'static [Cmd],
    pub view: View,
    /// The toolbar while not sketching, and while sketching.
    pub tools: &'static [Tool],
    pub sketch_tools: &'static [Tool],
    /// The instructions, in order; each moves on once its goal is met. The last one's goal finishes the step.
    pub hints: &'static [Hint],
    pub solution: &'static [Cmd],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    /// The default view, fitted to the scene.
    Default,
    /// Face-on to the open sketch.
    Sketch,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Sketch,
    Line,
    Rectangle,
    Circle,
    Arc,
    Extrude,
    Shell,
    Fillet,
    Revolve,
    Sweep,
    Loft,
    CircularPattern,
    Mirror,
    Union,
    Subtract,
    Thread,
    Visualization,
}

impl Tool {
    pub fn bit(self) -> u32 {
        1 << self as u32
    }
}

/// The gesture that meets an instruction.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Show {
    /// Press a toolbar button.
    Tool(Tool),
    /// Click a point of the model.
    At(Spot),
    /// Double-click it.
    Twice(Spot),
    /// Click one point, then another (a rectangle's corners, a circle's middle and edge).
    Two(Spot, Spot),
    /// Pull the running tool's arrow by this many mm.
    Pull(f64),
    /// Drag one of the Move arrows (0 X, 1 Y, 2 Z) by this many mm.
    Move(usize, f64),
    /// Press the instruction's own button, which sets this value.
    Value(&'static str),
    /// Drag the view cube.
    Cube,
    /// Click the view cube's top.
    CubeTop,
    /// Click a mirror plane (0 YZ, 1 XZ, 2 XY).
    Plane(usize),
    /// Click a color.
    Color,
    /// Press Done.
    Done,
}

pub struct Hint {
    pub text: &'static str,
    /// A second line, when the first needs a reason or a name.
    pub tip: Option<&'static str>,
    pub show: Show,
    pub goal: Goal,
}

const fn hint(text: &'static str, show: Show, goal: Goal) -> Hint {
    Hint { text, tip: None, show, goal }
}

const fn tip(text: &'static str, tip: &'static str, show: Show, goal: Goal) -> Hint {
    Hint { text, tip: Some(tip), show, goal }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Goal {
    /// Choosing a plane, or a sketch is open.
    Sketching,
    SketchOpen,
    /// A sketch is open on a body's face.
    SketchOnFace,
    /// A tool is running (its button is lit).
    Running(Tool),
    Lines(usize),
    Circles(usize),
    Picked(Pick),
    /// The running tool has all it needs: Done shows.
    Ready,
    /// The running tool has this many parts.
    Parts(usize),
    NewBody,
    /// This many bodies are left (or fewer).
    Joined(usize),
    /// The bodies grew by at least this many mm³.
    Grew(u32),
    /// The bodies lost at least this many mm³.
    Cut(u32),
    /// The bodies gained faces (a fillet, a thread).
    Rounded,
    Orbited,
    /// Looking straight down.
    FromTop,
    /// A body moved at least 5 mm.
    Moved,
    Visualizing,
    /// A body that had no material has one.
    Finished,
}

impl Goal {
    /// Something that has to go on holding while the next instruction is followed: a step whose sketch was left, or
    /// whose pick was let go, goes back to asking for it instead of waiting for what can't be done any more.
    pub fn holds(self) -> bool {
        matches!(self, Goal::SketchOpen | Goal::SketchOnFace | Goal::Running(_) | Goal::Picked(_) | Goal::Visualizing)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pick {
    Region,
    Face,
    Edge,
    Body,
}

/// What the document looks like, to compare with how the step started.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub bodies: usize,
    pub volume: f64,
    pub faces: usize,
    pub lines: usize,
    pub circles: usize,
    pub finished: usize,
    pub centers: Vec<(u64, DVec3)>,
}

/// What the workspace is doing right now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Live {
    pub picking_plane: bool,
    pub sketch_open: bool,
    pub sketch_on_face: bool,
    pub selection: SelectionKind,
    /// The step's tools whose buttons are lit (bit by bit, see [`Tool::bit`]).
    pub running: u32,
    pub ready: bool,
    /// How many bodies the running tool works on.
    pub parts: usize,
    /// How far the camera has turned since the step started, in degrees.
    pub turned: f64,
    pub from_top: bool,
    pub visualizing: bool,
}

pub fn snapshot(session: &Session) -> Snapshot {
    let doc = &session.doc;
    let mut s = Snapshot { bodies: doc.bodies.len(), ..Snapshot::default() };
    for b in &doc.bodies {
        s.volume += session.kernel.mass_props(&b.shape).map_or(0.0, |m| m.volume);
        s.faces += b.mesh.triangle_faces.iter().max().map_or(0, |m| *m as usize + 1);
        s.finished += usize::from(b.appearance.is_some());
        if let Some((lo, hi)) = b.mesh.bounds() {
            let c = (DVec3::from(lo.map(f64::from)) + DVec3::from(hi.map(f64::from))) / 2.0;
            s.centers.push((b.id.0, c));
        }
    }
    for sk in &doc.sketches {
        for e in &sk.entities {
            match e {
                shapecraft_doc::SketchEntity::Line { .. } => s.lines += 1,
                shapecraft_doc::SketchEntity::Rectangle { .. } => s.lines += 4,
                shapecraft_doc::SketchEntity::Circle { .. } => s.circles += 1,
                _ => {}
            }
        }
    }
    s
}

/// Whether a goal is met, from the step's starting snapshot, the current one and the workspace.
pub fn met(goal: Goal, base: &Snapshot, now: &Snapshot, live: &Live) -> bool {
    match goal {
        Goal::Sketching => live.picking_plane || live.sketch_open,
        Goal::SketchOpen => live.sketch_open,
        Goal::SketchOnFace => live.sketch_on_face,
        Goal::Running(t) => live.running & t.bit() != 0,
        Goal::Lines(n) => now.lines >= base.lines + n,
        Goal::Circles(n) => now.circles >= base.circles + n,
        Goal::Picked(p) => {
            live.selection
                == match p {
                    Pick::Region => SelectionKind::Regions,
                    Pick::Face => SelectionKind::Faces,
                    Pick::Edge => SelectionKind::Edges,
                    Pick::Body => SelectionKind::Bodies,
                }
        }
        Goal::Ready => live.ready,
        Goal::Parts(n) => live.parts >= n,
        Goal::NewBody => now.bodies > base.bodies,
        Goal::Joined(n) => now.bodies <= n && now.bodies < base.bodies,
        Goal::Grew(v) => now.volume >= base.volume + v as f64,
        Goal::Cut(v) => now.volume <= base.volume - v as f64,
        Goal::Rounded => now.faces > base.faces,
        Goal::Orbited => live.turned >= 20.0,
        Goal::FromTop => live.from_top,
        Goal::Moved => now.centers.iter().any(|(id, c)| base.centers.iter().any(|(b, d)| b == id && c.distance(*d) >= 5.0)),
        Goal::Visualizing => live.visualizing,
        Goal::Finished => now.finished > base.finished,
    }
}

/// Whether a goal can be checked from the document alone (the tests run those against the solutions).
pub fn on_document(goal: Goal) -> bool {
    matches!(
        goal,
        Goal::Lines(_)
            | Goal::Circles(_)
            | Goal::NewBody
            | Goal::Joined(_)
            | Goal::Grew(_)
            | Goal::Cut(_)
            | Goal::Rounded
            | Goal::Moved
            | Goal::Finished
    )
}

/// An instruction in the words for a touch screen: taps instead of clicks.
pub fn for_touch(text: &str) -> &str {
    TOUCH_WORDS.iter().find(|(a, _)| *a == text).map_or(text, |(_, b)| b)
}

/// The instructions a touch screen words differently (the rest say "select" and "pull", which hold for both).
pub const TOUCH_WORDS: &[(&str, &str)] = &[
    (
        "Press the button here, or type 95 and press Return: numbers work in every tool",
        "Press the button here, or tap the value to enter one",
    ),
    ("Drag the view cube to turn the view", "Drag with one finger to turn the view"),
    ("Double-click the coaster to select all of it", "Tap the coaster twice to select all of it"),
    ("Double-click the cup to select all of it", "Tap the cup twice to select all of it"),
    ("Double-click the stand to select all of it", "Tap the stand twice to select all of it"),
    ("Double-click the candle to select all of it", "Tap the candle twice to select all of it"),
    ("Double-click the cake to select all of it", "Tap the cake twice to select all of it"),
    ("Double-click the handle to select all of it", "Tap the handle twice to select all of it"),
    ("Double-click the tray to select all of it", "Tap the tray twice to select all of it"),
    ("Double-click the plate to select all of it", "Tap the plate twice to select all of it"),
];

/// Runs commands, filling in the `$` names from what the earlier ones made.
pub fn run(session: &mut Session, cmds: &[Cmd], vars: &mut HashMap<&'static str, Value>) -> Result<(), String> {
    for (id, params) in cmds {
        let p: Value = serde_json::from_str(params).map_err(|e| format!("{id}: {e}"))?;
        run_one(session, id, p, vars)?;
    }
    Ok(())
}

/// Puts face numbers in for `"@x,y,z"`: the face of the last body made that the point lies on.
fn faces(v: &mut Value, session: &Session, vars: &HashMap<&'static str, Value>) {
    match v {
        Value::String(s) if s.starts_with('@') => {
            let at: Vec<f64> = s[1..].split(',').filter_map(|x| x.parse().ok()).collect();
            let body = vars.get("body").and_then(Value::as_u64);
            if let ([x, y, z], Some(b)) = (at.as_slice(), session.doc.bodies.iter().find(|b| Some(b.id.0) == body))
                && let Some(f) = face_at(&b.mesh, DVec3::new(*x, *y, *z))
            {
                *v = Value::from(f);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|x| faces(x, session, vars)),
        Value::Object(o) => o.values_mut().for_each(|x| faces(x, session, vars)),
        _ => {}
    }
}

fn face_at(m: &shapecraft_kernel::Mesh, p: DVec3) -> Option<u32> {
    let at = |i: u32| m.positions.get(i as usize).map(|q| DVec3::from(q.map(f64::from)));
    let mut best: Option<(f64, u32)> = None;
    for (k, t) in m.indices.as_chunks::<3>().0.iter().enumerate() {
        let (Some(a), Some(b), Some(c), Some(face)) = (at(t[0]), at(t[1]), at(t[2]), m.triangle_faces.get(k)) else { continue };
        // Off its plane, and off the triangle in it (by how far the corners' shares fall outside).
        let n = (b - a).cross(c - a);
        let area = n.length();
        if area < 1e-12 {
            continue;
        }
        let share = |u: DVec3, w: DVec3| (n / area).dot((u - p).cross(w - p)) / area;
        let outside = [share(a, b), share(b, c), share(c, a)].iter().fold(0.0f64, |m, s| m.max(-s));
        let d = (p - a).dot(n / area).abs() + outside * area.sqrt();
        if best.is_none_or(|(e, _)| d < e) {
            best = Some((d, *face));
        }
    }
    best.map(|(_, f)| f)
}

fn fill(v: &mut Value, vars: &HashMap<&'static str, Value>) {
    match v {
        Value::String(s) if s.starts_with('$') => {
            if let Some(x) = vars.get(&s[1..]) {
                *v = x.clone();
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|x| fill(x, vars)),
        Value::Object(o) => o.values_mut().for_each(|x| fill(x, vars)),
        _ => {}
    }
}

/// The solution part of the way through (`t` from 0 to 1), for the picture that plays in the corner: sizes grow from
/// nothing, and what has no size (a union, a material) happens half-way.
pub fn part_way(cmds: &[Cmd], t: f64) -> Vec<(&'static str, Value)> {
    const SIZES: &[&str] = &["distance", "radius", "thickness", "size", "angle", "by"];
    const QUIET: &[&str] = &["sketch", "exit_sketching", "select", "deselect_all"];
    let mut out = Vec::new();
    for (id, params) in cmds {
        let Ok(mut p) = serde_json::from_str::<Value>(params) else { continue };
        let mut sized = false;
        if let Some(o) = p.as_object_mut() {
            for (_, v) in o.iter_mut().filter(|(k, _)| SIZES.contains(&k.as_str())) {
                sized |= scale(v, t);
            }
        }
        if if sized { t > 0.0 } else { t >= 0.5 || QUIET.contains(id) } {
            out.push((*id, p));
        }
    }
    out
}

/// Whether a solution has a size to play (or only a before and an after).
pub fn plays(cmds: &[Cmd]) -> bool {
    part_way(cmds, 0.0).len() < part_way(cmds, 0.1).len()
}

fn scale(v: &mut Value, t: f64) -> bool {
    match v {
        Value::Number(n) => {
            let Some(x) = n.as_f64() else { return false };
            *v = Value::from(x * t);
            true
        }
        Value::Array(a) => {
            let mut any = false;
            for x in a {
                any |= scale(x, t);
            }
            any
        }
        _ => false,
    }
}

/// Runs commands worked out at run time (see [`part_way`]).
pub fn run_values(session: &mut Session, cmds: &[(&'static str, Value)], vars: &mut HashMap<&'static str, Value>) -> Result<(), String> {
    for (id, params) in cmds {
        run_one(session, id, params.clone(), vars)?;
    }
    Ok(())
}

fn run_one(session: &mut Session, id: &str, mut p: Value, vars: &mut HashMap<&'static str, Value>) -> Result<(), String> {
    fill(&mut p, vars);
    faces(&mut p, session, vars);
    let out = session.execute(id, &p).map_err(|e| format!("{id}: {e}"))?;
    for key in ["sketch", "body", "face"] {
        if let Some(v) = out.value.get(key).filter(|v| v.is_u64()) {
            vars.insert(key, v.clone());
            if key == "body" {
                vars.entry("ref").or_insert_with(|| v.clone());
            }
        }
    }
    Ok(())
}

/// The lesson after this one on the page.
pub fn next_lesson(id: &str) -> Option<&'static Lesson> {
    LESSONS.iter().position(|l| l.id == id).and_then(|i| LESSONS.get(i + 1))
}

pub fn lesson(id: &str) -> Option<&'static Lesson> {
    LESSONS.iter().find(|l| l.id == id)
}

pub const LESSONS: &[Lesson] =
    &[COASTER, PENCIL_CUP, PHONE_STAND, DRAWER_KNOB, FLOWER_POT, COAT_HOOK, BIRTHDAY_CAKE, SERVING_TRAY, BOTTLE_CAP];

/// Joins command lists at compile time.
macro_rules! cmds {
    ($($part:expr),* $(,)?) => {{
        const LEN: usize = 0 $(+ $part.len())*;
        const ALL: [Cmd; LEN] = {
            let mut out: [Cmd; LEN] = [("", ""); LEN];
            let mut i = 0;
            $(
                let mut k = 0;
                while k < $part.len() {
                    out[i] = $part[k];
                    i += 1;
                    k += 1;
                }
            )*
            let _ = i;
            out
        };
        &ALL
    }};
}

const SKETCH_TOOLS: &[Tool] = &[Tool::Line, Tool::Arc, Tool::Rectangle, Tool::Circle];
const NONE: &[Tool] = &[];

/// Choosing Sketch, then the ground, as three courses begin.
const START_SKETCH: Hint = hint("Select Sketch", Show::Tool(Tool::Sketch), Goal::Sketching);
const PICK_GROUND: Hint =
    tip("Select the ground", "Anywhere on the grid: that's the paper you'll draw on", Show::At([24.0, -24.0, 0.0]), Goal::SketchOpen);
const CHOOSE_VISUALIZATION: Hint = hint("Select Visualize", Show::Tool(Tool::Visualization), Goal::Visualizing);
const CHOOSE_MATERIAL: Hint = tip("Select a color", "Change, up there, has wood, metal and plastic", Show::Color, Goal::Finished);

// ---- Beginner 1: the coaster -------------------------------------------------------------------------------------

const COASTER_SKETCH: &[Cmd] =
    &[("sketch", r#"{"plane": "xy"}"#), ("circle", r#"{"center": [0, 0], "radius": 48}"#), ("exit_sketching", "{}")];
const COASTER_BODY: &[Cmd] = &[("extrude", r#"{"sketch": "$sketch", "region": 0, "distance": 6}"#)];
const COASTER_DIP_SKETCH: &[Cmd] = &[
    ("sketch", r#"{"origin": [0, 0, 6], "normal": [0, 0, 1]}"#),
    ("circle", r#"{"center": [0, 0], "radius": 41}"#),
    ("exit_sketching", "{}"),
];
const COASTER_DIP: &[Cmd] = &[("extrude", r#"{"sketch": "$sketch", "region": 0, "distance": -2, "result": "subtract"}"#)];
const COASTER_SOFT: &[Cmd] =
    &[("select", r#"{"picks": [{"kind": "body", "body": "$body"}]}"#), ("fillet", r#"{"radius": 0.8}"#), ("deselect_all", "{}")];

const COASTER_LOOK: &[Cmd] = &[(
    "set_appearance",
    r#"{"items": ["$body"], "appearance": {"name": "Card", "color": [0.604, 0.416, 0.267], "metal": 0.0, "gloss": 0.5}}"#,
)];

const COASTER: Lesson = Lesson {
    id: "coaster",
    course: Course::Beginner,
    title: "Drink Coaster",
    about: "Look around, draw a circle and pull it into a solid",
    minutes: 5,
    finished: cmds![COASTER_SKETCH, COASTER_BODY, COASTER_DIP_SKETCH, COASTER_DIP, COASTER_SOFT, COASTER_LOOK],
    steps: &[
        Step {
            intro: "First, look around. This is the coaster you'll make.",
            setup: cmds![COASTER_SKETCH, COASTER_BODY, COASTER_DIP_SKETCH, COASTER_DIP, COASTER_SOFT],
            view: View::Default,
            tools: NONE,
            sketch_tools: NONE,
            hints: &[
                tip(
                    "Drag the view cube to turn the view",
                    "It's in the top right corner. You can't break anything by looking",
                    Show::Cube,
                    Goal::Orbited,
                ),
                hint("Select Top on the cube to look straight down", Show::CubeTop, Goal::FromTop),
            ],
            solution: &[],
        },
        Step {
            intro: "Every shape starts as a drawing. Draw a circle.",
            setup: &[],
            view: View::Default,
            tools: &[Tool::Sketch],
            sketch_tools: &[Tool::Circle],
            hints: &[
                START_SKETCH,
                PICK_GROUND,
                hint("Select Circle", Show::Tool(Tool::Circle), Goal::Running(Tool::Circle)),
                tip(
                    "Select the middle, then a point further out",
                    "The second point sets how big the circle is",
                    Show::Two([0.0, 0.0, 0.0], [48.0, 0.0, 0.0]),
                    Goal::Circles(1),
                ),
            ],
            solution: &[("sketch", r#"{"plane": "xy"}"#), ("circle", r#"{"center": [0, 0], "radius": 48}"#)],
        },
        Step {
            intro: "Pull the drawing up and it becomes solid.",
            setup: COASTER_SKETCH,
            view: View::Default,
            tools: &[Tool::Extrude],
            sketch_tools: NONE,
            hints: &[
                hint("Select the inside of the circle", Show::At([12.0, -12.0, 0.0]), Goal::Picked(Pick::Region)),
                hint("Pull the arrow up", Show::Pull(6.0), Goal::NewBody),
            ],
            solution: COASTER_BODY,
        },
        Step {
            intro: "Push a shape in and it cuts. Make a dip for the glass.",
            setup: cmds![COASTER_SKETCH, COASTER_BODY, COASTER_DIP_SKETCH],
            view: View::Default,
            tools: &[Tool::Extrude],
            sketch_tools: NONE,
            hints: &[
                hint("Select the inner circle", Show::At([8.0, -8.0, 6.0]), Goal::Picked(Pick::Region)),
                hint("Push the arrow down, into the coaster", Show::Pull(-2.0), Goal::Cut(200)),
            ],
            solution: COASTER_DIP,
        },
        Step {
            intro: "Take the sharpness off the edge.",
            setup: cmds![COASTER_SKETCH, COASTER_BODY, COASTER_DIP_SKETCH, COASTER_DIP],
            view: View::Default,
            tools: &[Tool::Fillet],
            sketch_tools: NONE,
            hints: &[
                hint("Select the outer edge on top", Show::At([33.94, -33.94, 6.0]), Goal::Picked(Pick::Edge)),
                hint("Set the rounding to 1 mm", Show::Value("1"), Goal::Rounded),
            ],
            solution: &[("select", r#"{"picks": [{"kind": "body", "body": "$body"}]}"#), ("fillet", r#"{"radius": 0.8}"#)],
        },
        Step {
            intro: "Last, choose a color.",
            setup: cmds![COASTER_SKETCH, COASTER_BODY, COASTER_DIP_SKETCH, COASTER_DIP, COASTER_SOFT],
            view: View::Default,
            tools: &[Tool::Visualization],
            sketch_tools: NONE,
            hints: &[
                hint("Double-click the coaster to select all of it", Show::Twice([8.0, -8.0, 4.0]), Goal::Picked(Pick::Body)),
                CHOOSE_VISUALIZATION,
                CHOOSE_MATERIAL,
            ],
            solution: &[(
                "set_appearance",
                r#"{"items": ["$body"], "appearance": {"name": "Red", "color": [0.85, 0.2, 0.17], "metal": 0.0, "gloss": 0.5}}"#,
            )],
        },
    ],
};

// ---- Beginner 2: the pencil cup ----------------------------------------------------------------------------------

const CUP_SKETCH: &[Cmd] = &[("sketch", r#"{"plane": "xy"}"#), ("circle", r#"{"center": [0, 0], "radius": 35}"#), ("exit_sketching", "{}")];
const CUP_BODY: &[Cmd] = &[("extrude", r#"{"sketch": "$sketch", "region": 0, "distance": 95}"#)];
const CUP_SHELL: &[Cmd] = &[("shell", r#"{"body": "$body", "faces": ["$face"], "thickness": 3}"#)];
const CUP_SOFT: &[Cmd] =
    &[("select", r#"{"picks": [{"kind": "body", "body": "$body"}]}"#), ("fillet", r#"{"radius": 1}"#), ("deselect_all", "{}")];

const CUP_LOOK: &[Cmd] = &[(
    "set_appearance",
    r#"{"items": ["$body"], "appearance": {"name": "Card", "color": [0.949, 0.608, 0.173], "metal": 0.0, "gloss": 0.5}}"#,
)];

const PENCIL_CUP: Lesson = Lesson {
    id: "pencil-cup",
    course: Course::Beginner,
    title: "Pencil Cup",
    about: "Exact sizes, hollowing out, and moving things",
    minutes: 5,
    finished: cmds![CUP_SKETCH, CUP_BODY, CUP_SHELL, CUP_SOFT, CUP_LOOK],
    steps: &[
        Step {
            intro: "Draw the bottom of the cup.",
            setup: &[],
            view: View::Default,
            tools: &[Tool::Sketch],
            sketch_tools: SKETCH_TOOLS,
            hints: &[
                START_SKETCH,
                PICK_GROUND,
                hint("Select Circle", Show::Tool(Tool::Circle), Goal::Running(Tool::Circle)),
                hint("Select the middle, then a point further out", Show::Two([0.0, 0.0, 0.0], [35.0, 0.0, 0.0]), Goal::Circles(1)),
            ],
            solution: &[("sketch", r#"{"plane": "xy"}"#), ("circle", r#"{"center": [0, 0], "radius": 35}"#)],
        },
        Step {
            intro: "Make it exactly 95 mm tall.",
            setup: CUP_SKETCH,
            view: View::Default,
            tools: &[Tool::Extrude],
            sketch_tools: NONE,
            hints: &[
                hint("Select the inside of the circle", Show::At([10.0, -10.0, 0.0]), Goal::Picked(Pick::Region)),
                tip(
                    "Set the height to 95 mm",
                    "Press the button here, or type 95 and press Return: numbers work in every tool",
                    Show::Value("95"),
                    Goal::NewBody,
                ),
            ],
            solution: CUP_BODY,
        },
        Step {
            intro: "Hollow it out with Shell.",
            setup: cmds![CUP_SKETCH, CUP_BODY],
            view: View::Default,
            tools: &[Tool::Shell],
            sketch_tools: NONE,
            hints: &[
                tip(
                    "Select the top face",
                    "The face you select is the one that opens",
                    Show::At([0.0, 0.0, 95.0]),
                    Goal::Picked(Pick::Face),
                ),
                hint("Select Shell", Show::Tool(Tool::Shell), Goal::Running(Tool::Shell)),
                hint("Set the walls to 3 mm", Show::Value("3"), Goal::Cut(20000)),
            ],
            solution: CUP_SHELL,
        },
        Step {
            intro: "Round the rim so it's soft to the touch.",
            setup: cmds![CUP_SKETCH, CUP_BODY, CUP_SHELL],
            view: View::Default,
            tools: &[Tool::Fillet],
            sketch_tools: NONE,
            hints: &[
                hint("Select the rim's outer edge", Show::At([24.75, -24.75, 95.0]), Goal::Picked(Pick::Edge)),
                hint("Set the rounding to 1 mm", Show::Value("1"), Goal::Rounded),
            ],
            solution: &[("select", r#"{"picks": [{"kind": "body", "body": "$body"}]}"#), ("fillet", r#"{"radius": 1}"#)],
        },
        Step {
            intro: "Put the cup where you want it.",
            setup: cmds![CUP_SKETCH, CUP_BODY, CUP_SHELL, CUP_SOFT],
            view: View::Default,
            tools: NONE,
            sketch_tools: NONE,
            hints: &[
                hint("Double-click the cup to select all of it", Show::Twice([24.75, -24.75, 45.0]), Goal::Picked(Pick::Body)),
                tip("Drag an arrow to slide the cup", "The arcs between the arrows turn it", Show::Move(0, 25.0), Goal::Moved),
            ],
            solution: &[("translate", r#"{"by": [25, 0, 0], "items": ["$body"]}"#)],
        },
        Step {
            intro: "Give the cup a color.",
            setup: cmds![CUP_SKETCH, CUP_BODY, CUP_SHELL, CUP_SOFT],
            view: View::Default,
            tools: &[Tool::Visualization],
            sketch_tools: NONE,
            hints: &[
                hint("Double-click the cup to select all of it", Show::Twice([24.75, -24.75, 45.0]), Goal::Picked(Pick::Body)),
                CHOOSE_VISUALIZATION,
                CHOOSE_MATERIAL,
            ],
            solution: &[(
                "set_appearance",
                r#"{"items": ["$body"], "appearance": {"name": "Orange", "color": [0.95, 0.6, 0.15], "metal": 0.0, "gloss": 0.5}}"#,
            )],
        },
    ],
};

// ---- Beginner 3: the phone stand ---------------------------------------------------------------------------------

const STAND_SKETCH: &[Cmd] =
    &[("sketch", r#"{"plane": "xy"}"#), ("rectangle", r#"{"center": [0, 0], "size": [90, 70]}"#), ("exit_sketching", "{}")];
const STAND_BASE: &[Cmd] = &[("extrude", r#"{"sketch": "$sketch", "region": 0, "distance": 8}"#)];
const STAND_BACK_SKETCH: &[Cmd] = &[
    ("sketch", r#"{"origin": [0, 0, 8], "normal": [0, 0, 1]}"#),
    ("rectangle", r#"{"center": [-28, 0], "size": [14, 90]}"#),
    ("exit_sketching", "{}"),
];
const STAND_BACK: &[Cmd] = &[("extrude", r#"{"sketch": "$sketch", "region": 0, "distance": 70, "result": "union"}"#)];
const STAND_SLOT_SKETCH: &[Cmd] = &[
    ("sketch", r#"{"origin": [0, 0, 8], "normal": [0, 0, 1]}"#),
    ("rectangle", r#"{"center": [12, 0], "size": [12, 90]}"#),
    ("exit_sketching", "{}"),
];
const STAND_SLOT: &[Cmd] = &[("extrude", r#"{"sketch": "$sketch", "region": 0, "distance": -4, "result": "subtract"}"#)];
const STAND_SOFT: &[Cmd] =
    &[("select", r#"{"picks": [{"kind": "body", "body": "$body"}]}"#), ("fillet", r#"{"radius": 1.5}"#), ("deselect_all", "{}")];

const STAND_LOOK: &[Cmd] = &[(
    "set_appearance",
    r#"{"items": ["$body"], "appearance": {"name": "Card", "color": [0.247, 0.655, 0.765], "metal": 0.0, "gloss": 0.5}}"#,
)];

const PHONE_STAND: Lesson = Lesson {
    id: "phone-stand",
    course: Course::Beginner,
    title: "Phone Stand",
    about: "Draw on a part, add to it and cut into it",
    minutes: 7,
    finished: cmds![STAND_SKETCH, STAND_BASE, STAND_BACK_SKETCH, STAND_BACK, STAND_SLOT_SKETCH, STAND_SLOT, STAND_SOFT, STAND_LOOK],
    steps: &[
        Step {
            intro: "Draw the base of the stand.",
            setup: &[],
            view: View::Default,
            tools: &[Tool::Sketch],
            sketch_tools: SKETCH_TOOLS,
            hints: &[
                START_SKETCH,
                PICK_GROUND,
                hint("Select Rectangle", Show::Tool(Tool::Rectangle), Goal::Running(Tool::Rectangle)),
                hint("Select one corner, then the opposite one", Show::Two([-45.0, -35.0, 0.0], [45.0, 35.0, 0.0]), Goal::Lines(4)),
            ],
            solution: &[("sketch", r#"{"plane": "xy"}"#), ("rectangle", r#"{"center": [0, 0], "size": [90, 70]}"#)],
        },
        Step {
            intro: "Make the base 8 mm thick.",
            setup: STAND_SKETCH,
            view: View::Default,
            tools: &[Tool::Extrude],
            sketch_tools: NONE,
            hints: &[
                hint("Select the inside of the rectangle", Show::At([18.0, -14.0, 0.0]), Goal::Picked(Pick::Region)),
                hint("Set the thickness to 8 mm", Show::Value("8"), Goal::NewBody),
            ],
            solution: STAND_BASE,
        },
        Step {
            intro: "You can draw right on a part.",
            setup: cmds![STAND_SKETCH, STAND_BASE],
            view: View::Default,
            tools: &[Tool::Sketch],
            sketch_tools: SKETCH_TOOLS,
            hints: &[
                hint("Select the top face", Show::At([10.0, -8.0, 8.0]), Goal::Picked(Pick::Face)),
                hint("Select Sketch", Show::Tool(Tool::Sketch), Goal::SketchOnFace),
                hint("Select Rectangle", Show::Tool(Tool::Rectangle), Goal::Running(Tool::Rectangle)),
                tip(
                    "Draw a strip along the back edge",
                    "From the back left corner to a point on the right edge",
                    Show::Two([-45.0, 35.0, 8.0], [45.0, 21.0, 8.0]),
                    Goal::Lines(4),
                ),
            ],
            solution: &[("sketch", r#"{"body": "$body", "face": "$face"}"#), ("rectangle", r#"{"center": [0, 28], "size": [90, 14]}"#)],
        },
        Step {
            intro: "Pull the strip up: it becomes part of the base.",
            setup: cmds![STAND_SKETCH, STAND_BASE, STAND_BACK_SKETCH],
            view: View::Default,
            tools: &[Tool::Extrude],
            sketch_tools: NONE,
            hints: &[
                hint("Select the strip", Show::At([0.0, 28.0, 8.0]), Goal::Picked(Pick::Region)),
                hint("Set the height to 70 mm", Show::Value("70"), Goal::Grew(5000)),
            ],
            solution: STAND_BACK,
        },
        Step {
            intro: "Cut a slot for the phone to rest in.",
            setup: cmds![STAND_SKETCH, STAND_BASE, STAND_BACK_SKETCH, STAND_BACK, STAND_SLOT_SKETCH],
            view: View::Default,
            tools: &[Tool::Extrude],
            sketch_tools: NONE,
            hints: &[
                hint("Select the slot", Show::At([0.0, -12.0, 8.0]), Goal::Picked(Pick::Region)),
                hint("Push the arrow down, into the base", Show::Pull(-4.0), Goal::Cut(200)),
            ],
            solution: STAND_SLOT,
        },
        Step {
            intro: "Round off a sharp edge.",
            setup: cmds![STAND_SKETCH, STAND_BASE, STAND_BACK_SKETCH, STAND_BACK, STAND_SLOT_SKETCH, STAND_SLOT],
            view: View::Default,
            tools: &[Tool::Fillet],
            sketch_tools: NONE,
            hints: &[
                hint("Select the top front edge", Show::At([0.0, 21.0, 78.0]), Goal::Picked(Pick::Edge)),
                hint("Pull the arrow to round the edge", Show::Pull(3.0), Goal::Rounded),
            ],
            solution: &[("select", r#"{"picks": [{"kind": "body", "body": "$body"}]}"#), ("fillet", r#"{"radius": 1.5}"#)],
        },
        Step {
            intro: "Give the stand a color.",
            setup: cmds![STAND_SKETCH, STAND_BASE, STAND_BACK_SKETCH, STAND_BACK, STAND_SLOT_SKETCH, STAND_SLOT, STAND_SOFT],
            view: View::Default,
            tools: &[Tool::Visualization],
            sketch_tools: NONE,
            hints: &[
                hint("Double-click the stand to select all of it", Show::Twice([0.0, 21.0, 45.0]), Goal::Picked(Pick::Body)),
                CHOOSE_VISUALIZATION,
                CHOOSE_MATERIAL,
            ],
            solution: &[(
                "set_appearance",
                r#"{"items": ["$body"], "appearance": {"name": "Orange", "color": [0.95, 0.6, 0.15], "metal": 0.0, "gloss": 0.5}}"#,
            )],
        },
    ],
};

// ---- Revolve: the drawer knob ------------------------------------------------------------------------------------

const KNOB_SKETCH: &[Cmd] = &[
    ("sketch", r#"{"plane": "xz"}"#),
    ("line", r#"{"a": [0, 0], "b": [14, 0]}"#),
    ("line", r#"{"a": [14, 0], "b": [14, 3]}"#),
    ("line", r#"{"a": [14, 3], "b": [6, 7]}"#),
    ("line", r#"{"a": [6, 7], "b": [6, 16]}"#),
    ("line", r#"{"a": [6, 16], "b": [17, 23]}"#),
    ("line", r#"{"a": [17, 23], "b": [17, 29]}"#),
    ("line", r#"{"a": [17, 29], "b": [0, 33]}"#),
    ("line", r#"{"a": [0, 33], "b": [0, 0]}"#),
    ("exit_sketching", "{}"),
];
const KNOB_BODY: &[Cmd] = &[("revolve", r#"{"sketch": "$sketch", "region": 0, "axis": {"line": 7}, "angle": 360}"#)];
const KNOB_HALF: &[Cmd] = &[("revolve", r#"{"sketch": "$sketch", "region": 0, "axis": {"line": 7}, "angle": 180}"#)];
const KNOB_SOFT: &[Cmd] =
    &[("select", r#"{"picks": [{"kind": "body", "body": "$body"}]}"#), ("fillet", r#"{"radius": 0.8}"#), ("deselect_all", "{}")];

const KNOB_LOOK: &[Cmd] = &[(
    "set_appearance",
    r#"{"items": ["$body"], "appearance": {"name": "Card", "color": [0.851, 0.643, 0.255], "metal": 0.0, "gloss": 0.5}}"#,
)];

const DRAWER_KNOB: Lesson = Lesson {
    id: "drawer-knob",
    course: Course::Tool,
    title: "Revolve: a Drawer Knob",
    about: "Spin a profile around an axis",
    minutes: 3,
    finished: cmds![KNOB_SKETCH, KNOB_BODY, KNOB_SOFT, KNOB_LOOK],
    steps: &[
        Step {
            intro: "Half a knob's outline, spun all the way round, is a knob.",
            setup: KNOB_SKETCH,
            view: View::Default,
            tools: &[Tool::Revolve],
            sketch_tools: NONE,
            hints: &[
                hint("Select the inside of the outline", Show::At([9.0, 0.0, 24.0]), Goal::Picked(Pick::Region)),
                hint("Select Revolve", Show::Tool(Tool::Revolve), Goal::Running(Tool::Revolve)),
                tip(
                    "Select the straight edge down the middle",
                    "It's the axis the outline spins around",
                    Show::At([0.0, 0.0, 11.0]),
                    Goal::Ready,
                ),
                hint("Select Done", Show::Done, Goal::NewBody),
            ],
            solution: KNOB_BODY,
        },
        Step {
            intro: "It doesn't have to go all the way round. Stop half-way.",
            setup: KNOB_SKETCH,
            view: View::Default,
            tools: &[Tool::Revolve],
            sketch_tools: NONE,
            hints: &[
                hint("Select the inside of the outline", Show::At([9.0, 0.0, 24.0]), Goal::Picked(Pick::Region)),
                hint("Select Revolve", Show::Tool(Tool::Revolve), Goal::Running(Tool::Revolve)),
                hint("Select the straight edge down the middle", Show::At([0.0, 0.0, 11.0]), Goal::Ready),
                tip("Set the angle to 180°", "Or drag the arrow on the dashed circle", Show::Value("180"), Goal::NewBody),
            ],
            solution: KNOB_HALF,
        },
        Step {
            intro: "Soften the front so it's nice to pull.",
            setup: cmds![KNOB_SKETCH, KNOB_BODY],
            view: View::Default,
            tools: &[Tool::Fillet],
            sketch_tools: NONE,
            hints: &[
                hint("Select the widest edge", Show::At([12.02, -12.02, 29.0]), Goal::Picked(Pick::Edge)),
                hint("Set the rounding to 1 mm", Show::Value("1"), Goal::Rounded),
            ],
            solution: &[("select", r#"{"picks": [{"kind": "body", "body": "$body"}]}"#), ("fillet", r#"{"radius": 0.8}"#)],
        },
    ],
};

// ---- Loft: the flower pot ----------------------------------------------------------------------------------------

const POT_SKETCHES: &[Cmd] = &[
    ("sketch", r#"{"plane": "xy"}"#),
    ("circle", r#"{"center": [0, 0], "radius": 30}"#),
    ("exit_sketching", "{}"),
    ("sketch", r#"{"origin": [0, 0, 75], "normal": [0, 0, 1], "x_dir": [1, 0, 0]}"#),
    ("circle", r#"{"center": [0, 0], "radius": 46}"#),
    ("exit_sketching", "{}"),
];
const POT_BODY: &[Cmd] = &[("loft", r#"{"profiles": [{"sketch": 1, "region": 0}, {"sketch": 2, "region": 0}]}"#), ("deselect_all", "{}")];
const POT_HOLE_SKETCH: &[Cmd] = &[
    ("sketch", r#"{"origin": [0, 0, 75], "normal": [0, 0, 1], "x_dir": [1, 0, 0]}"#),
    ("circle", r#"{"center": [0, 0], "radius": 27}"#),
    ("exit_sketching", "{}"),
];
const POT_HOLE: &[Cmd] =
    &[("extrude", r#"{"sketch": "$sketch", "region": 0, "distance": -68, "result": "subtract"}"#), ("deselect_all", "{}")];

const POT_LOOK: &[Cmd] = &[(
    "set_appearance",
    r#"{"items": ["$body"], "appearance": {"name": "Card", "color": [0.761, 0.392, 0.235], "metal": 0.0, "gloss": 0.5}}"#,
)];

const FLOWER_POT: Lesson = Lesson {
    id: "flower-pot",
    course: Course::Tool,
    title: "Loft: a Flower Pot",
    about: "Blend one shape into another",
    minutes: 3,
    finished: cmds![POT_SKETCHES, POT_BODY, POT_HOLE_SKETCH, POT_HOLE, POT_LOOK],
    steps: &[
        Step {
            intro: "Small at the bottom, wide at the top. Loft makes everything between.",
            setup: POT_SKETCHES,
            view: View::Default,
            tools: &[Tool::Loft],
            sketch_tools: NONE,
            hints: &[
                hint("Select the inside of the small circle", Show::At([14.0, -14.0, 0.0]), Goal::Picked(Pick::Region)),
                hint("Select Loft", Show::Tool(Tool::Loft), Goal::Running(Tool::Loft)),
                hint("Select the inside of the big circle", Show::At([-6.0, 6.0, 75.0]), Goal::Ready),
                hint("Select Done", Show::Done, Goal::NewBody),
            ],
            solution: POT_BODY,
        },
        Step {
            intro: "Make room for the plant and it's a pot.",
            setup: cmds![POT_SKETCHES, POT_BODY, POT_HOLE_SKETCH],
            view: View::Default,
            tools: &[Tool::Extrude],
            sketch_tools: NONE,
            hints: &[
                hint("Select the inner circle", Show::At([8.0, -8.0, 75.0]), Goal::Picked(Pick::Region)),
                tip("Set the depth to -65 mm", "A minus goes down, into the pot", Show::Value("-65"), Goal::Cut(20000)),
            ],
            solution: POT_HOLE,
        },
    ],
};

// ---- Sweep and Union: the coat hook ------------------------------------------------------------------------------

const HOOK_PLATE: &[Cmd] = &[
    ("sketch", r#"{"plane": "xz"}"#),
    ("rectangle", r#"{"center": [0, 12], "size": [36, 60]}"#),
    ("exit_sketching", "{}"),
    ("extrude", r#"{"sketch": "$sketch", "region": 0, "distance": -6}"#),
];
const HOOK_SKETCHES: &[Cmd] = &[
    ("sketch", r#"{"plane": "yz"}"#),
    ("arc", r#"{"a": [0, 0], "mid": [-23.33, 9.67], "b": [-33, 33]}"#),
    ("exit_sketching", "{}"),
    ("sketch", r#"{"plane": "xz"}"#),
    ("circle", r#"{"center": [0, 0], "radius": 6}"#),
    ("exit_sketching", "{}"),
];
const HOOK_BODY: &[Cmd] =
    &[("sweep", r#"{"profile": {"sketch": 4, "region": 0}, "path": {"sketch": 3, "entities": [0]}}"#), ("deselect_all", "{}")];
const HOOK_JOIN: &[Cmd] = &[("union", r#"{"target": 2, "tools": ["$body"]}"#), ("deselect_all", "{}")];

const HOOK_LOOK: &[Cmd] = &[(
    "set_appearance",
    r#"{"items": ["$ref"], "appearance": {"name": "Card", "color": [0.561, 0.702, 0.353], "metal": 0.0, "gloss": 0.5}}"#,
)];

const COAT_HOOK: Lesson = Lesson {
    id: "coat-hook",
    course: Course::Tool,
    title: "Sweep: a Coat Hook",
    about: "Run a shape along a path, then join parts with Union",
    minutes: 4,
    finished: cmds![HOOK_PLATE, HOOK_SKETCHES, HOOK_BODY, HOOK_JOIN, HOOK_LOOK],
    steps: &[
        Step {
            intro: "Sweep runs a shape along a line, like squeezing out toothpaste.",
            setup: cmds![HOOK_PLATE, HOOK_SKETCHES],
            view: View::Default,
            tools: &[Tool::Sweep],
            sketch_tools: NONE,
            hints: &[
                hint("Select the inside of the circle", Show::At([3.0, 0.0, -2.0]), Goal::Picked(Pick::Region)),
                hint("Select Sweep", Show::Tool(Tool::Sweep), Goal::Running(Tool::Sweep)),
                hint("Select the curved line", Show::At([0.0, -28.58, 16.5]), Goal::Ready),
                hint("Select Done", Show::Done, Goal::NewBody),
            ],
            solution: HOOK_BODY,
        },
        Step {
            intro: "The hook and the plate are two parts. Union makes them one.",
            setup: cmds![HOOK_PLATE, HOOK_SKETCHES, HOOK_BODY],
            view: View::Default,
            tools: &[Tool::Union],
            sketch_tools: NONE,
            hints: &[
                hint("Double-click the plate to select all of it", Show::Twice([10.0, 0.0, 34.0]), Goal::Picked(Pick::Body)),
                hint("Select Union", Show::Tool(Tool::Union), Goal::Running(Tool::Union)),
                hint("Select the hook", Show::At([4.0, -28.0, 20.0]), Goal::Parts(2)),
                hint("Select Done", Show::Done, Goal::Joined(1)),
            ],
            solution: HOOK_JOIN,
        },
    ],
};

// ---- Circular pattern and Subtract: the birthday cake ------------------------------------------------------------

const CAKE_BODY: &[Cmd] = &[
    ("sketch", r#"{"plane": "xy"}"#),
    ("circle", r#"{"center": [0, 0], "radius": 60}"#),
    ("exit_sketching", "{}"),
    ("extrude", r#"{"sketch": "$sketch", "region": 0, "distance": 36}"#),
];
const CAKE_CANDLE: &[Cmd] = &[
    ("sketch", r#"{"origin": [0, 0, 36], "normal": [0, 0, 1], "x_dir": [1, 0, 0]}"#),
    ("circle", r#"{"center": [31, -31], "radius": 4}"#),
    ("exit_sketching", "{}"),
    ("extrude", r#"{"sketch": "$sketch", "region": 0, "distance": 30, "result": "new"}"#),
];
const CAKE_CANDLES: &[Cmd] =
    &[("pattern_circular", r#"{"origin": [0, 0, 0], "axis": [0, 0, 1], "count": 6, "items": [4]}"#), ("deselect_all", "{}")];
const CAKE_WEDGE: &[Cmd] = &[
    ("sketch", r#"{"plane": "xy"}"#),
    ("line", r#"{"a": [0, 0], "b": [91.8, -6.4]}"#),
    ("line", r#"{"a": [91.8, -6.4], "b": [82.7, -40.3]}"#),
    ("line", r#"{"a": [82.7, -40.3], "b": [0, 0]}"#),
    ("exit_sketching", "{}"),
    ("extrude", r#"{"sketch": "$sketch", "region": 0, "distance": 60, "result": "new"}"#),
];
const CAKE_CUT: &[Cmd] = &[("subtract", r#"{"target": 2, "tools": ["$body"]}"#), ("deselect_all", "{}")];

const CAKE_LOOK: &[Cmd] =
    &[("set_appearance", r#"{"items": [2], "appearance": {"name": "Card", "color": [0.898, 0.561, 0.69], "metal": 0.0, "gloss": 0.5}}"#)];
const CAKE_FLAMES: &[Cmd] = &[(
    "set_appearance",
    r#"{"items": [4, 5, 6, 7, 8, 9], "appearance": {"name": "Card", "color": [1.0, 0.941, 0.784], "metal": 0.0, "gloss": 0.5}}"#,
)];

const BIRTHDAY_CAKE: Lesson = Lesson {
    id: "birthday-cake",
    course: Course::Tool,
    title: "Pattern: a Birthday Cake",
    about: "Copy a part around a circle, and cut one part with another",
    minutes: 4,
    finished: cmds![CAKE_BODY, CAKE_CANDLE, CAKE_CANDLES, CAKE_WEDGE, CAKE_CUT, CAKE_LOOK, CAKE_FLAMES],
    steps: &[
        Step {
            intro: "One candle becomes six with Circular Pattern.",
            setup: cmds![CAKE_BODY, CAKE_CANDLE],
            view: View::Default,
            tools: &[Tool::CircularPattern],
            sketch_tools: NONE,
            hints: &[
                hint("Double-click the candle to select all of it", Show::Twice([33.8, -33.8, 54.0]), Goal::Picked(Pick::Body)),
                hint("Select Pattern - Circular", Show::Tool(Tool::CircularPattern), Goal::Running(Tool::CircularPattern)),
                tip("Select the cake's round edge", "The candles go around its middle", Show::At([-42.43, -42.43, 36.0]), Goal::Ready),
                hint("Select Done", Show::Done, Goal::NewBody),
            ],
            solution: CAKE_CANDLES,
        },
        Step {
            intro: "Cut a slice: Subtract takes one part out of another.",
            setup: cmds![CAKE_BODY, CAKE_CANDLE, CAKE_CANDLES, CAKE_WEDGE],
            view: View::Default,
            tools: &[Tool::Subtract],
            sketch_tools: NONE,
            hints: &[
                hint("Double-click the cake to select all of it", Show::Twice([-20.0, -20.0, 36.0]), Goal::Picked(Pick::Body)),
                hint("Select Subtract", Show::Tool(Tool::Subtract), Goal::Running(Tool::Subtract)),
                hint("Select the wedge", Show::At([62.0, -16.6, 60.0]), Goal::Parts(2)),
                hint("Select Done", Show::Done, Goal::Cut(5000)),
            ],
            solution: &[("subtract", r#"{"target": 2, "tools": ["$body"]}"#), ("deselect_all", "{}")],
        },
    ],
};

// ---- Mirror: the serving tray ------------------------------------------------------------------------------------

const TRAY_BODY: &[Cmd] = &[
    ("sketch", r#"{"plane": "xy"}"#),
    ("rectangle", r#"{"center": [0, 0], "size": [160, 100]}"#),
    ("exit_sketching", "{}"),
    ("extrude", r#"{"sketch": "$sketch", "region": 0, "distance": 10}"#),
];
const TRAY_HANDLE: &[Cmd] = &[
    ("sketch", r#"{"origin": [0, 0, 10], "normal": [0, 0, 1], "x_dir": [1, 0, 0]}"#),
    ("rectangle", r#"{"center": [70, 0], "size": [12, 60]}"#),
    ("exit_sketching", "{}"),
    ("extrude", r#"{"sketch": "$sketch", "region": 0, "distance": 26, "result": "new"}"#),
];
const TRAY_MIRROR: &[Cmd] =
    &[("mirror", r#"{"origin": [0, 0, 0], "normal": [1, 0, 0], "items": [4], "copy": true}"#), ("deselect_all", "{}")];

const TRAY_LOOK: &[Cmd] = &[(
    "set_appearance",
    r#"{"items": [2, 4, 5], "appearance": {"name": "Card", "color": [0.725, 0.541, 0.353], "metal": 0.0, "gloss": 0.5}}"#,
)];

const SERVING_TRAY: Lesson = Lesson {
    id: "serving-tray",
    course: Course::Tool,
    title: "Mirror: a Serving Tray",
    about: "Make one side, get the other for free",
    minutes: 3,
    finished: cmds![TRAY_BODY, TRAY_HANDLE, TRAY_MIRROR, TRAY_LOOK],
    steps: &[
        Step {
            intro: "Model one handle. Mirror makes the other.",
            setup: cmds![TRAY_BODY, TRAY_HANDLE],
            view: View::Default,
            tools: &[Tool::Mirror],
            sketch_tools: NONE,
            hints: &[
                hint("Double-click the handle to select all of it", Show::Twice([70.0, -10.0, 36.0]), Goal::Picked(Pick::Body)),
                hint("Select Mirror", Show::Tool(Tool::Mirror), Goal::Running(Tool::Mirror)),
                tip("Select the plane down the middle of the tray", "The copy appears on its other side", Show::Plane(0), Goal::NewBody),
            ],
            solution: TRAY_MIRROR,
        },
        Step {
            intro: "Join the tray and both handles into one part.",
            setup: cmds![TRAY_BODY, TRAY_HANDLE, TRAY_MIRROR],
            view: View::Default,
            tools: &[Tool::Union],
            sketch_tools: NONE,
            hints: &[
                hint("Double-click the tray to select all of it", Show::Twice([10.0, -25.0, 10.0]), Goal::Picked(Pick::Body)),
                hint("Select Union", Show::Tool(Tool::Union), Goal::Running(Tool::Union)),
                hint("Select both handles", Show::Two([70.0, -10.0, 36.0], [-70.0, -10.0, 36.0]), Goal::Parts(3)),
                hint("Select Done", Show::Done, Goal::Joined(1)),
            ],
            solution: &[("union", r#"{"target": 2, "tools": [4, 5]}"#), ("deselect_all", "{}")],
        },
    ],
};

// ---- Thread: the bottle and its cap ------------------------------------------------------------------------------

const BOTTLE: &[Cmd] = &[
    ("sketch", r#"{"plane": "xy"}"#),
    ("circle", r#"{"center": [0, 0], "radius": 22}"#),
    ("exit_sketching", "{}"),
    ("extrude", r#"{"sketch": "$sketch", "region": 0, "distance": 30}"#),
    ("sketch", r#"{"origin": [0, 0, 30], "normal": [0, 0, 1], "x_dir": [1, 0, 0]}"#),
    ("circle", r#"{"center": [0, 0], "radius": 12}"#),
    ("exit_sketching", "{}"),
    ("extrude", r#"{"sketch": "$sketch", "region": 0, "distance": 18, "result": "union"}"#),
    ("deselect_all", "{}"),
];
const BOTTLE_THREAD: &[Cmd] = &[
    ("select", r#"{"picks": [{"kind": "face", "body": "$body", "face": "@8.485,-8.485,40"}]}"#),
    ("thread", "{}"),
    ("deselect_all", "{}"),
];

const BOTTLE_LOOK: &[Cmd] = &[(
    "set_appearance",
    r#"{"items": ["$body"], "appearance": {"name": "Card", "color": [0.31, 0.616, 0.561], "metal": 0.0, "gloss": 0.5}}"#,
)];

const BOTTLE_CAP: Lesson = Lesson {
    id: "bottle-neck",
    course: Course::Tool,
    title: "Thread: a Bottle Neck",
    about: "Screw threads that really screw on",
    minutes: 2,
    finished: cmds![BOTTLE, BOTTLE_THREAD, BOTTLE_LOOK],
    steps: &[Step {
        intro: "Any round neck or hole can take a screw thread.",
        setup: BOTTLE,
        view: View::Default,
        tools: &[Tool::Thread],
        sketch_tools: NONE,
        hints: &[
            hint("Select the side of the neck", Show::At([8.485, -8.485, 40.0]), Goal::Picked(Pick::Face)),
            tip("Select Thread", "The size that fits is chosen for you", Show::Tool(Tool::Thread), Goal::Rounded),
        ],
        solution: BOTTLE_THREAD,
    }],
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn session() -> Session {
        Session::new(Arc::new(shapecraft_kernel_occt::OcctKernel::new()), "Tutorial")
    }

    /// Every step's scene builds, and its scripted solution meets every goal that the document can show.
    #[test]
    fn every_step_can_be_finished() {
        for lesson in LESSONS {
            let mut s = session();
            run(&mut s, lesson.finished, &mut HashMap::new()).unwrap_or_else(|e| panic!("{} finished: {e}", lesson.id));
            assert!(!s.doc.bodies.is_empty(), "{} finished has no body", lesson.id);
            for (i, step) in lesson.steps.iter().enumerate() {
                let mut s = session();
                let mut vars = HashMap::new();
                run(&mut s, step.setup, &mut vars).unwrap_or_else(|e| panic!("{} step {}: setup: {e}", lesson.id, i + 1));
                let base = snapshot(&s);
                run(&mut s, step.solution, &mut vars).unwrap_or_else(|e| panic!("{} step {}: solution: {e}", lesson.id, i + 1));
                let now = snapshot(&s);
                let live = Live {
                    picking_plane: false,
                    sketch_open: s.active_sketch.is_some(),
                    sketch_on_face: false,
                    selection: s.selection_kind(),
                    running: 0,
                    ready: false,
                    parts: 0,
                    turned: 0.0,
                    from_top: false,
                    visualizing: false,
                };
                for h in step.hints.iter().filter(|h| on_document(h.goal)) {
                    assert!(met(h.goal, &base, &now, &live), "{} step {}: {:?} not met by the solution", lesson.id, i + 1, h.goal);
                }
                // A goal mustn't be met before the step is done (it would skip the step).
                for h in step.hints.iter().filter(|h| on_document(h.goal)) {
                    assert!(!met(h.goal, &base, &base, &live), "{} step {}: {:?} already met at the start", lesson.id, i + 1, h.goal);
                }
                // And the corner's picture of it plays: part of the way through is either built or refused, never a crash.
                for t in [0.0, 0.3, 0.6] {
                    let mut s = session();
                    let mut vars = HashMap::new();
                    let _ = run(&mut s, step.setup, &mut vars);
                    let _ = run_values(&mut s, &part_way(step.solution, t), &mut vars);
                }
            }
        }
    }

    /// Nothing asks for a tool the step doesn't put on screen, and every step ends on something the document shows
    /// or the camera does.
    #[test]
    fn every_instruction_can_be_followed_with_what_is_on_screen() {
        for lesson in LESSONS {
            for (i, step) in lesson.steps.iter().enumerate() {
                let name = format!("{} step {}", lesson.id, i + 1);
                assert!(!step.hints.is_empty(), "{name} asks for nothing");
                for h in step.hints {
                    if let Show::Tool(t) = h.show {
                        assert!(step.tools.contains(&t) || step.sketch_tools.contains(&t), "{name}: {t:?} isn't on screen");
                    }
                    if matches!(h.goal, Goal::SketchOpen | Goal::SketchOnFace) {
                        assert!(step.tools.contains(&Tool::Sketch), "{name}: no way into a sketch");
                    }
                    if matches!(h.goal, Goal::Lines(_) | Goal::Circles(_)) {
                        assert!(!step.sketch_tools.is_empty(), "{name}: nothing to draw with");
                    }
                }
            }
        }
        assert_eq!(LESSONS.iter().filter(|l| l.course == Course::Beginner).count(), 3);
    }

    /// On a touch screen nothing asks for a click.
    #[test]
    fn touch_words_name_no_clicks() {
        for lesson in LESSONS {
            for text in lesson.steps.iter().flat_map(|s| s.hints).flat_map(|h| std::iter::once(h.text).chain(h.tip)) {
                for k in ["lick", "Return", "keyboard", "type "] {
                    assert!(!for_touch(text).contains(k), "{}: {text}", lesson.id);
                }
            }
        }
        for (a, _) in TOUCH_WORDS {
            assert!(
                LESSONS.iter().flat_map(|l| l.steps).flat_map(|s| s.hints).any(|h| h.text == *a || h.tip == Some(*a)),
                "nobody says {a}"
            );
        }
    }

    #[test]
    fn placeholders_fill_in() {
        let mut vars = HashMap::new();
        vars.insert("body", Value::from(7));
        let mut v: Value = serde_json::from_str(r#"{"items": ["$body"], "x": "$none"}"#).unwrap_or_default();
        fill(&mut v, &vars);
        assert_eq!(v["items"][0], 7);
        assert_eq!(v["x"], "$none");
    }
}
