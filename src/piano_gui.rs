use egui::{Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

pub struct PianoGui {
    pressed_keys: u16,
}

impl PianoGui {
    pub fn new() -> Self {
        Self { pressed_keys: 0 }
    }

    pub fn draw(&mut self, ui: &mut Ui, ctx: &egui::Context) -> Option<Action> {
        let mut action = None;
        let piano_width = 600.0;
        let piano_height = 200.0;
        let (response, painter) =
            ui.allocate_painter(vec2(piano_width, piano_height), Sense::empty());
        let rect = response.rect;
        painter.rect_filled(rect, 1.0, Color32::GRAY);
        const MARGIN: f32 = 2.0;
        let keys_rect = rect.shrink(MARGIN);
        const NUM_WHITE_KEYS: usize = 7;
        let white_width = piano_width / NUM_WHITE_KEYS as f32;
        for white_key in 0..NUM_WHITE_KEYS {
            let key_rect = Rect::from_min_size(
                pos2(
                    keys_rect.min.x + white_key as f32 / NUM_WHITE_KEYS as f32 * keys_rect.width(),
                    keys_rect.min.y,
                ),
                vec2(white_width, keys_rect.height()),
            );
            let semitone = white_key_to_semitone(white_key);
            let pressed = (self.pressed_keys >> semitone & 1) != 0;
            let note = wmidi::Note::C4.step(semitone).unwrap();
            painter.rect(
                key_rect,
                2.0,
                if pressed {
                    Color32::BLUE
                } else {
                    Color32::WHITE
                },
                Stroke::new(2.0, Color32::BLACK),
                StrokeKind::Inside,
            );
            let key_id = ui.id().with(format!("white{white_key}"));
            let key_response = ui.interact(key_rect, key_id, Sense::click());
            if key_response.is_pointer_button_down_on()
                && !ui.data(|r| r.get_temp::<bool>(key_id).unwrap_or(false))
            {
                ui.data_mut(|r| r.insert_temp(key_id, true));
                debug_assert!(action.is_none());
                action = Some(Action::Pressed(note));
            } else if !key_response.is_pointer_button_down_on()
                && ui.data(|r| r.get_temp::<bool>(key_id).unwrap_or(false))
            {
                ui.data_mut(|r| r.insert_temp(key_id, false));
                debug_assert!(action.is_none());
                action = Some(Action::Released(note));
            }
        }
        let black_size = vec2(white_width * 0.6, keys_rect.height() * 0.6);
        const NUM_BLACK_KEYS: usize = 5;
        for black_key in 0..NUM_BLACK_KEYS {
            let white_key = match black_key {
                0 => 0,
                1 => 1,
                2 => 3,
                3 => 4,
                4 => 5,
                _ => panic!("Invalid black key index"),
            };
            let semitone = black_key_to_semitone(black_key);
            // let white_key = semitone as usize / 2;
            // let white_width = piano_width / NUM_WHITE_KEYS as f32;
            // let white_key_rect = Rect::from_min_size(
            //     pos2(
            //         keys_rect.min.x + white_key as f32 / NUM_WHITE_KEYS as f32 * keys_rect.width(),
            //         keys_rect.min.y,
            //     ),
            //     vec2(white_width, keys_rect.height()),
            // );
            let key_rect = Rect::from_min_size(
                pos2(
                    keys_rect.min.x + white_width * (white_key as f32 + 1.0) - 0.5 * black_size.x,
                    keys_rect.min.y,
                ),
                black_size,
            );
            let pressed = (self.pressed_keys >> semitone & 1) != 0;
            let note = wmidi::Note::C4.step(semitone).unwrap();
            painter.rect_filled(
                key_rect,
                2.0,
                if pressed {
                    Color32::BLUE
                } else {
                    Color32::BLACK
                },
                // Stroke::new(2.0, Color32::WHITE),
                // StrokeKind::Inside,
            );
            let key_id = ui.id().with(format!("black{black_key}"));
            let key_response = ui.interact(key_rect, key_id, Sense::click());
            if key_response.is_pointer_button_down_on()
                && !ui.data(|r| r.get_temp::<bool>(key_id).unwrap_or(false))
            {
                ui.data_mut(|r| r.insert_temp(key_id, true));
                debug_assert!(action.is_none());
                action = Some(Action::Pressed(note));
            } else if !key_response.is_pointer_button_down_on()
                && ui.data(|r| r.get_temp::<bool>(key_id).unwrap_or(false))
            {
                ui.data_mut(|r| r.insert_temp(key_id, false));
                debug_assert!(action.is_none());
                action = Some(Action::Released(note));
            }
        }
        action
        // const WHITE_KEY_NUM: usize = 7;
        // let white_key_width = piano_width / WHITE_KEY_NUM as f32;
        // let white_key_height = piano_height;
        // let black_key_width = white_key_width * 0.6;
        // let black_key_height = piano_height * 0.6;

        // // Create a canvas to draw on
        // let (response, painter) =
        //     ui.allocate_painter(vec2(piano_width, piano_height), Sense::click_and_drag());
        // let rect = response.rect;

        // // Draw the piano frame
        // painter.rect_filled(rect, 0.0, Color32::from_rgb(120, 120, 120));

        // // Define key mapping from visual position to semitone index
        // // In a standard piano layout: C, D, E, F, G, A, B for white keys
        // // The black keys are: C#, D#, F#, G#, A#
        // // The semitone sequence is: C(0), C#(1), D(2), D#(3), E(4), F(5), F#(6), G(7), G#(8), A(9), A#(10), B(11)
        // let white_key_to_semitone = [0, 2, 4, 5, 7, 9, 11]; // C, D, E, F, G, A, B
        // let black_key_positions_to_semitone = [
        //     (0, 1),  // After C comes C#
        //     (1, 3),  // After D comes D#
        //     (3, 6),  // After F comes F#
        //     (4, 8),  // After G comes G#
        //     (5, 10), // After A comes A#
        // ];

        // // First, draw all white keys
        // for i in 0..WHITE_KEY_NUM {
        //     let key_rect = Rect::from_min_size(
        //         pos2(rect.min.x + i as f32 * white_key_width, rect.min.y),
        //         vec2(white_key_width, white_key_height),
        //     );

        //     // Map white key position to semitone index
        //     let semitone = white_key_to_semitone[i];
        //     let key_bit = semitone;

        //     let is_pressed = (self.pressed_keys & (1 << key_bit)) != 0;
        //     let key_color = if is_pressed {
        //         Color32::from_rgb(200, 200, 255)
        //     } else {
        //         Color32::WHITE
        //     };

        //     painter.rect_filled(key_rect, 0.0, key_color);
        //     painter.rect_stroke(
        //         key_rect,
        //         0.0,
        //         Stroke::new(1.0, Color32::BLACK),
        //         StrokeKind::Middle,
        //     );
        // }

        // // Create a map to track which regions are already handled by black keys
        // let mut handled_regions: Vec<Rect> = Vec::new();

        // let mut action = Action::None;

        // // Handle black keys - draw and check interactions
        // for &(white_key_idx, semitone) in &black_key_positions_to_semitone {
        //     let offset = white_key_width - (black_key_width / 2.0);
        //     let key_rect = Rect::from_min_size(
        //         pos2(
        //             rect.min.x + white_key_idx as f32 * white_key_width + offset,
        //             rect.min.y,
        //         ),
        //         vec2(black_key_width, black_key_height),
        //     );

        //     // Add to handled regions
        //     handled_regions.push(key_rect);

        //     // Check if key is being pressed
        //     let key_id = ui.id().with(format!("black_key_{}", semitone));
        //     let key_response = ui.interact(key_rect, key_id, Sense::click_and_drag());

        //     // Set or clear the bit based on interaction
        //     let key_bit = semitone;
        //     if key_response.dragged() || key_response.clicked() {
        //         self.pressed_keys |= 1 << key_bit; // Set the bit
        //         action = Action::Pressed(wmidi::Note::C4.step(semitone).unwrap());
        //     } else if key_response.drag_stopped()
        //         || (key_response.hovered() && ctx.input(|i| i.pointer.any_released()))
        //     {
        //         self.pressed_keys &= !(1 << key_bit); // Clear the bit
        //         action = Action::Released(wmidi::Note::C4.step(semitone).unwrap());
        //     }

        //     // Draw key with appropriate color based on pressed state
        //     let is_pressed = (self.pressed_keys & (1 << key_bit)) != 0;
        //     let key_color = if is_pressed {
        //         Color32::from_rgb(50, 50, 100)
        //     } else {
        //         Color32::BLACK
        //     };

        //     painter.rect_filled(key_rect, 0.0, key_color);
        // }

        // // Now process interactions for white keys, but only in areas not covered by black keys
        // for i in 0..WHITE_KEY_NUM {
        //     let key_rect = Rect::from_min_size(
        //         pos2(rect.min.x + i as f32 * white_key_width, rect.min.y),
        //         vec2(white_key_width, white_key_height),
        //     );

        //     // Check if the pointer is within this key's rect and not in any black key rect
        //     if response
        //         .hover_pos()
        //         .map_or(false, |pos| key_rect.contains(pos))
        //     {
        //         let overlapping_with_black = handled_regions.iter().any(|black_rect| {
        //             response
        //                 .hover_pos()
        //                 .map_or(false, |pos| black_rect.contains(pos))
        //         });

        //         if !overlapping_with_black {
        //             let semitone = white_key_to_semitone[i];
        //             let key_bit = semitone;

        //             let key_id = ui.id().with(format!("white_key_{}", semitone));
        //             let key_response = ui.interact(key_rect, key_id, Sense::click_and_drag());

        //             // Set or clear the bit based on interaction
        //             if key_response.dragged() || key_response.clicked() {
        //                 self.pressed_keys |= 1 << key_bit; // Set the bit
        //                 action = Action::Pressed(wmidi::Note::C4.step(semitone).unwrap());
        //             } else if key_response.drag_stopped()
        //                 || (key_response.hovered() && ctx.input(|i| i.pointer.any_released()))
        //             {
        //                 self.pressed_keys &= !(1 << key_bit); // Clear the bit
        //                 action = Action::Released(wmidi::Note::C4.step(semitone).unwrap());
        //             }
        //         }
        //     } else if response.clicked_elsewhere() {
        //         // Clear key when clicking elsewhere
        //         let semitone = white_key_to_semitone[i];
        //         let key_bit = semitone;
        //         self.pressed_keys &= !(1 << key_bit);
        //     }
        // }
        // action
    }
}

pub enum Action {
    Pressed(wmidi::Note),
    Released(wmidi::Note),
}

fn white_key_to_semitone(key: usize) -> i8 {
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

fn black_key_to_semitone(key: usize) -> i8 {
    match key {
        0 => 1,
        1 => 3,
        2 => 6,
        3 => 8,
        4 => 10,
        _ => panic!("Invalid black key index"),
    }
}
