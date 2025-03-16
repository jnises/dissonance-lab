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
        // Check if shift key is held down
        let shift_pressed = ui.input(|i| i.modifiers.shift);
        const NUM_WHITE_KEYS: usize = 7;
        const NUM_BLACK_KEYS: usize = 5;
        let white_width = PIANO_WIDTH / NUM_WHITE_KEYS as f32;
        #[derive(strum_macros::Display)]
        enum Color {
            White,
            Black,
        };
        for color in [Color::White, Color::Black] {
            let key_size = match color {
                Color::White => vec2(white_width, keys_rect.height()),
                Color::Black => vec2(white_width * 0.6, keys_rect.height() * 0.6),
            };
            let num_keys = match color {
                Color::White => NUM_WHITE_KEYS,
                Color::Black => NUM_BLACK_KEYS,
            };
            for key in 0..num_keys {
                // TODO: make D, E and A wider to make the lanes behave better.. or make the upper part of the keyboard same width for white and black keys
                let key_id = ui.id().with(format!("{color}{key}"));
                let key_rect = Rect::from_min_size(
                    pos2(
                        keys_rect.min.x + match color {
                            Color::White => key as f32 / NUM_WHITE_KEYS as f32 * keys_rect.width(),
                            Color::Black => white_width * match key {
                                0 => 1.0,
                                1 => 2.0,
                                2 => 4.0,
                                3 => 5.0,
                                4 => 6.0,
                                _ => unreachable!(),
                            } - 0.5 * key_size.x,
                        } ,
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

                    // If shift is not pressed, clear all keys before setting the new one
                    if !shift_pressed {
                        self.selected_keys.fill(false);
                    }

                    self.selected_keys.set(semitone, true);
                } else if !key_response.is_pointer_button_down_on() && mouse_pressed {
                    ui.data_mut(|r| r.insert_temp(key_id, false));
                    debug_assert!(action.is_none());
                    action = Some(Action::Released(note));
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
