use egui::{
    Align2, Color32, FontId, Painter, Pos2, Response, Role, Sense, Shape, Stroke, StrokeKind, Ui,
    WidgetInfo, epaint::QuadraticBezierShape, vec2,
};

use crate::theme;

const ICON_STROKE_WIDTH: f32 = 1.5;
pub const CONTROL_HEIGHT: f32 = 36.0;
const ICON_LABEL_FONT_SIZE: f32 = 12.0;

#[derive(Clone, Copy)]
pub enum Icon {
    Speaker { muted: bool },
    EqualTuning,
    JustTuning,
    Hold { active: bool },
    Reference,
    Polarity { inverted: bool },
}

impl Icon {
    fn paint(self, painter: &Painter, center: Pos2, color: Color32) {
        let stroke = Stroke::new(ICON_STROKE_WIDTH, color);
        let point = |x, y| center + vec2(x, y);
        let line = |a, b| painter.line_segment([a, b], stroke);

        match self {
            Self::Speaker { muted } => {
                painter.add(Shape::closed_line(
                    vec![
                        point(-8.0, -3.0),
                        point(-5.0, -3.0),
                        point(-1.0, -6.0),
                        point(-1.0, 6.0),
                        point(-5.0, 3.0),
                        point(-8.0, 3.0),
                    ],
                    stroke,
                ));
                if muted {
                    line(point(2.0, -2.5), point(7.0, 2.5));
                    line(point(7.0, -2.5), point(2.0, 2.5));
                } else {
                    for (left, top, arc) in [(2.0, 3.0, 5.0), (5.0, 5.5, 9.0)] {
                        painter.add(QuadraticBezierShape::from_points_stroke(
                            [point(left, -top), point(arc, 0.0), point(left, top)],
                            false,
                            Color32::TRANSPARENT,
                            stroke,
                        ));
                    }
                }
            }
            Self::EqualTuning => {
                line(point(-7.0, 4.0), point(7.0, 4.0));
                for x in [-7.0, -3.5, 0.0, 3.5, 7.0] {
                    line(point(x, -4.0), point(x, 4.0));
                }
            }
            Self::JustTuning => {
                // Two and three subdivisions of the same span suggest a simple ratio.
                for (y, marks) in [
                    (-3.0, &[-7.0, 0.0, 7.0][..]),
                    (4.0, &[-7.0, -2.3, 2.3, 7.0]),
                ] {
                    line(point(-7.0, y), point(7.0, y));
                    for &x in marks {
                        line(point(x, y - 2.0), point(x, y + 2.0));
                    }
                }
            }
            Self::Hold { active } => {
                let lock_body = egui::Rect::from_min_max(point(-5.5, -0.5), point(5.5, 6.0));
                painter.rect_stroke(lock_body, 1.0, stroke, StrokeKind::Middle);
                let right_end = if active { -0.5 } else { -3.0 };
                painter.add(Shape::line(
                    vec![
                        point(-3.0, -0.5),
                        point(-3.0, -4.0),
                        point(-1.5, -5.5),
                        point(1.5, -5.5),
                        point(3.0, -4.0),
                        point(3.0, right_end),
                    ],
                    stroke,
                ));
            }
            Self::Reference => {
                painter.circle_stroke(center, 4.5, stroke);
                painter.circle_filled(center, 1.25, color);
            }
            Self::Polarity { inverted } => {
                let direction = if inverted { -1.0 } else { 1.0 };
                painter.add(Shape::line(
                    vec![
                        point(-6.0, -4.0 * direction),
                        point(-6.0, 0.0),
                        point(5.0, 0.0),
                    ],
                    stroke,
                ));
                painter.add(Shape::line(
                    vec![point(2.0, -3.0), point(5.0, 0.0), point(2.0, 3.0)],
                    stroke,
                ));
            }
        }
    }

    pub fn button(
        self,
        ui: &mut Ui,
        name: &str,
        caption: Option<&str>,
        selected: Option<bool>,
    ) -> Response {
        let galley = caption.map(|caption| {
            ui.painter().layout_no_wrap(
                caption.to_owned(),
                FontId::proportional(ICON_LABEL_FONT_SIZE),
                theme::outlines(),
            )
        });
        let width = CONTROL_HEIGHT + galley.as_ref().map_or(0.0, |text| text.size().x);
        let (rect, response) = ui.allocate_exact_size(vec2(width, CONTROL_HEIGHT), Sense::click());
        response.widget_info(|| match selected {
            Some(active) => {
                let role = match self {
                    Self::EqualTuning | Self::JustTuning => Role::RadioButton,
                    _ => Role::CheckBox,
                };
                WidgetInfo::selected(role, response.enabled(), active, name)
            }
            None => WidgetInfo::labeled(Role::Button, response.enabled(), name),
        });

        let active = selected == Some(true);
        let color = if response.hovered() || active {
            theme::outlines()
        } else {
            theme::KEYBOARD_LABEL
        };
        if active || response.hovered() {
            ui.painter()
                .rect_filled(rect, 3.0, theme::outlines().gamma_multiply(0.08));
        }
        if active || response.has_focus() {
            ui.painter().rect_stroke(
                rect,
                3.0,
                Stroke::new(1.0, theme::outlines().gamma_multiply(0.45)),
                StrokeKind::Inside,
            );
        }
        let center = rect.left_center() + vec2(CONTROL_HEIGHT / 2.0, 0.0);
        self.paint(ui.painter(), center, color);
        if let Some(galley) = galley {
            let label_rect = Align2::LEFT_CENTER.anchor_size(
                rect.left_center() + vec2(CONTROL_HEIGHT - 5.0, 0.0),
                galley.size(),
            );
            ui.painter().galley(label_rect.min, galley, color);
        }
        response
    }

    pub fn indicator(self, ui: &mut Ui, active: bool) {
        let (response, painter) = ui.allocate_painter(vec2(16.0, CONTROL_HEIGHT), Sense::hover());
        let color = if active {
            theme::outlines()
        } else {
            theme::KEYBOARD_LABEL
        };
        self.paint(&painter, response.rect.center(), color);
    }
}
