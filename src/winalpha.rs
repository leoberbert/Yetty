//! Whole-window translucency for platforms without a window-level alpha call (Linux).
//!
//! The window is created transparent; at the end of each frame every shape painted this frame gets its
//! colors multiplied by `factor`, like `SetLayeredWindowAttributes(LWA_ALPHA)` does on Windows.

use egui::epaint::{ColorMode, PathStroke, Shape};
use egui::{Color32, Context, LayerId, Stroke};

fn fade(c: &mut Color32, f: f32) {
    *c = c.gamma_multiply(f);
}

fn fade_stroke(s: &mut Stroke, f: f32) {
    fade(&mut s.color, f);
}

fn fade_path_stroke(s: &mut PathStroke, f: f32) {
    if let ColorMode::Solid(c) = &mut s.color {
        fade(c, f);
    }
}

fn fade_shape(shape: &mut Shape, f: f32) {
    match shape {
        Shape::Noop | Shape::Callback(_) => {}
        Shape::Vec(v) => v.iter_mut().for_each(|s| fade_shape(s, f)),
        Shape::Circle(s) => {
            fade(&mut s.fill, f);
            fade_stroke(&mut s.stroke, f);
        }
        Shape::Ellipse(s) => {
            fade(&mut s.fill, f);
            fade_stroke(&mut s.stroke, f);
        }
        Shape::LineSegment { stroke, .. } => fade_stroke(stroke, f),
        Shape::Path(s) => {
            fade(&mut s.fill, f);
            fade_path_stroke(&mut s.stroke, f);
        }
        Shape::Rect(s) => {
            fade(&mut s.fill, f);
            fade_stroke(&mut s.stroke, f);
        }
        Shape::Text(s) => s.opacity_factor *= f,
        Shape::Mesh(m) => {
            let m = std::sync::Arc::make_mut(m);
            m.vertices.iter_mut().for_each(|v| fade(&mut v.color, f));
        }
        Shape::QuadraticBezier(s) => {
            fade(&mut s.fill, f);
            fade_path_stroke(&mut s.stroke, f);
        }
        Shape::CubicBezier(s) => {
            fade(&mut s.fill, f);
            fade_path_stroke(&mut s.stroke, f);
        }
    }
}

/// Multiplies the opacity of everything painted so far this frame by `factor` (0..=1).
pub fn apply(ctx: &Context, factor: f32) {
    if factor >= 1.0 {
        return;
    }
    let mut layers = ctx.memory(|m| m.layer_ids().collect::<Vec<_>>());
    if !layers.contains(&LayerId::background()) {
        layers.push(LayerId::background());
    }
    ctx.graphics_mut(|g| {
        for id in layers {
            if let Some(list) = g.get_mut(id) {
                let n = list.all_entries().len();
                for i in 0..n {
                    list.mutate_shape(egui::layers::ShapeIdx(i), |c| {
                        fade_shape(&mut c.shape, factor)
                    });
                }
            }
        }
    });
}
