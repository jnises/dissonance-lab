use bitvec::{BitArr, order::Msb0};
use egui::{Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

pub struct PianoGui {
    pressed_keys: BitArr!(for 12, in u16, Msb0),
}

impl PianoGui {
    pub fn new() -> Self {
        Self {
            pressed_keys: Default::default(),
        }
    }

    pub fn draw(&mut self, ui: &mut Ui) -> Option<Action> {
        let mut action = None;
        let piano_width = 600.0;
        let piano_height = 200.0;
        let (response, painter) =
            ui.allocate_painter(vec2(piano_width, piano_height), Sense::empty());
        let rect = response.rect;
        painter.rect_filled(rect, 1.0, ui.visuals().panel_fill);
        const MARGIN: f32 = 2.0;
        let keys_rect = rect.shrink(MARGIN);
        const NUM_WHITE_KEYS: usize = 7;
        let white_width = piano_width / NUM_WHITE_KEYS as f32;

        // Check if shift key is held down
        let shift_pressed = ui.input(|i| i.modifiers.shift);

        for white_key in 0..NUM_WHITE_KEYS {
            let key_id = ui.id().with(format!("white{white_key}"));
            let key_rect = Rect::from_min_size(
                pos2(
                    keys_rect.min.x + white_key as f32 / NUM_WHITE_KEYS as f32 * keys_rect.width(),
                    keys_rect.min.y,
                ),
                vec2(white_width, keys_rect.height()),
            );
            let semitone = white_key_to_semitone(white_key);
            let pressed = self.pressed_keys[semitone];
            let note = wmidi::Note::C4.step(semitone as i8).unwrap();
            painter.rect(
                key_rect,
                2.0,
                if pressed {
                    ui.visuals().selection.bg_fill
                } else {
                    Color32::WHITE
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
                    self.pressed_keys.fill(false);
                }

                self.pressed_keys.set(semitone, true);
            } else if !key_response.is_pointer_button_down_on() && mouse_pressed {
                ui.data_mut(|r| r.insert_temp(key_id, false));
                debug_assert!(action.is_none());
                action = Some(Action::Released(note));
                if !shift_pressed {
                    self.pressed_keys.set(semitone, false);
                }
            }
        }
        let black_size = vec2(white_width * 0.6, keys_rect.height() * 0.6);
        const NUM_BLACK_KEYS: usize = 5;
        for black_key in 0..NUM_BLACK_KEYS {
            let key_id = ui.id().with(format!("black{black_key}"));
            let white_key = match black_key {
                0 => 0,
                1 => 1,
                2 => 3,
                3 => 4,
                4 => 5,
                _ => panic!("Invalid black key index"),
            };
            let semitone = black_key_to_semitone(black_key);
            let key_rect = Rect::from_min_size(
                pos2(
                    keys_rect.min.x + white_width * (white_key as f32 + 1.0) - 0.5 * black_size.x,
                    keys_rect.min.y,
                ),
                black_size,
            );
            let pressed = self.pressed_keys[semitone];
            let note = wmidi::Note::C4.step(semitone.try_into().unwrap()).unwrap();
            painter.rect(
                key_rect,
                2.0,
                if pressed {
                    ui.visuals().selection.bg_fill
                } else {
                    Color32::BLACK
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
                    self.pressed_keys.fill(false);
                }

                self.pressed_keys.set(semitone, true);
            } else if !key_response.is_pointer_button_down_on() && mouse_pressed {
                ui.data_mut(|r| r.insert_temp(key_id, false));
                debug_assert!(action.is_none());
                action = Some(Action::Released(note));
                if !shift_pressed {
                    self.pressed_keys.set(semitone, false);
                }
            }
        }
        action
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
