use bitvec::{BitArr, order::Msb0};
use egui::{Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

pub const PIANO_WIDTH: f32 = 600.0;
pub const PIANO_HEIGHT: f32 = 200.0;

pub type KeySet = BitArr!(for 12, in u16, Msb0);

pub struct PianoGui {
    selected_keys: KeySet,
}

impl PianoGui {
    pub fn new() -> Self {
        Self {
            selected_keys: Default::default(),
        }
    }

    pub fn show(&mut self, ui: &mut Ui) -> (Option<Action>, Rect) {
        let mut action = None;
        let (response, painter) =
            ui.allocate_painter(vec2(PIANO_WIDTH, PIANO_HEIGHT), Sense::empty());
        let rect = response.rect;
        painter.rect_filled(rect, 1.0, ui.visuals().panel_fill);
        const MARGIN: f32 = 2.0;
        let keys_rect = rect.shrink(MARGIN);
        let shift_pressed = ui.input(|i| i.modifiers.shift);
        const NUM_WHITE_KEYS: usize = 7;
        const NUM_BLACK_KEYS: usize = 5;
        #[derive(strum_macros::Display)]
        enum Color {
            White,
            Black,
        };
        for color in [Color::White, Color::Black] {
            let num_keys = match color {
                Color::White => NUM_WHITE_KEYS,
                Color::Black => NUM_BLACK_KEYS,
            };
            let x = match color {
                Color::White => vec![0.0, 1.5, 3.5, 5.0, 6.5, 8.5, 10.5],
                Color::Black => vec![1.0, 3.0, 6.0, 8.0, 10.0],
            };
            for key in 0..num_keys {
                let key_id = ui.id().with(format!("{color}{key}"));
                let key_size = match color {
                    Color::White => vec2(
                        (x.get(key + 1).unwrap_or(&12.0) - x[key]) / 12.0 * keys_rect.width(),
                        keys_rect.height(),
                    ),
                    Color::Black => vec2(keys_rect.width() / 12.0, keys_rect.height() * 0.6),
                };
                let key_rect = Rect::from_min_size(
                    pos2(
                        keys_rect.min.x + x[key] / 12.0 * keys_rect.width(),
                        keys_rect.min.y,
                    ),
                    key_size,
                );
                let semitone = match color {
                    Color::White => white_key_to_semitone(key),
                    Color::Black => black_key_to_semitone(key),
                };
                let pressed = self.selected_keys[semitone];
                let note = wmidi::Note::C4.step(semitone as i8).unwrap();
                painter.rect(
                    key_rect,
                    2.0,
                    if pressed {
                        ui.visuals().selection.bg_fill
                    } else {
                        match color {
                            Color::White => Color32::WHITE,
                            Color::Black => Color32::BLACK,
                        }
                    },
                    Stroke::new(2.0, Color32::BLACK),
                    StrokeKind::Inside,
                );
                let key_response = ui.interact(key_rect, key_id, Sense::click());
                let mouse_pressed = ui.data(|r| r.get_temp::<bool>(key_id).unwrap_or(false));
                if key_response.is_pointer_button_down_on() && !mouse_pressed {
                    ui.data_mut(|r| r.insert_temp(key_id, true));
                    debug_assert!(action.is_none());
                    action = Some(Action::Pressed(note));
                    if !shift_pressed {
                        self.selected_keys.fill(false);
                    }
                    self.selected_keys.set(semitone, true);
                } else if !key_response.is_pointer_button_down_on() && mouse_pressed {
                    ui.data_mut(|r| r.insert_temp(key_id, false));
                    debug_assert!(action.is_none());
                    action = Some(Action::Released(note));
                    if !shift_pressed {
                        self.selected_keys.set(semitone, false);
                    }
                }
            }
        }
        (action, keys_rect)
    }

    pub fn selected_keys(&self) -> &KeySet {
        &self.selected_keys
    }
}

pub enum Action {
    Pressed(wmidi::Note),
    Released(wmidi::Note),
}

fn white_key_to_semitone(key: usize) -> usize {
    match key {
        0 => 0,
        1 => 2,
        2 => 4,
        3 => 5,
        4 => 7,
        5 => 9,
        6 => 11,
        _ => panic!("Invalid white key index"),
    }
}

fn black_key_to_semitone(key: usize) -> usize {
    match key {
        0 => 1,
        1 => 3,
        2 => 6,
        3 => 8,
        4 => 10,
        _ => panic!("Invalid black key index"),
    }
}
