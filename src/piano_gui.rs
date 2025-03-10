//use i_overlay::{MultipolygonClipper, PolygonClipper, SimplePolygon, BooleanOp};
use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
};

// Point type is now just an array
pub type Point = [f32; 2];

// Helper function for creating points
pub fn point(x: f32, y: f32) -> Point {
    [x, y]
}

#[derive(Debug, Clone)]
pub struct Polygon {
    pub points: Vec<Point>,
}

impl Polygon {
    pub fn new(points: Vec<Point>) -> Self {
        Self { points }
    }

    // Create a rectangular polygon
    pub fn rectangle(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            points: vec![
                [x, y],
                [x + width, y],
                [x + width, y + height],
                [x, y + height],
            ],
        }
    }

    // // Convert to i_overlay's polygon type (no conversion needed now)
    // fn to_overlay_polygon(&self) -> SimplePolygon<f32> {
    //     SimplePolygon::new(self.points.clone()).expect("Invalid polygon")
    // }

    // // Convert from i_overlay's polygon type (no conversion needed now)
    // fn from_overlay_polygon(polygon: SimplePolygon<f32>) -> Self {
    //     Self::new(polygon.iter().cloned().collect())
    // }

    // Boolean operations using i_overlay

    // Subtract polygon b from polygon a (a - b)
    pub fn difference(a: &Polygon, b: &Polygon) -> Option<Polygon> {
        let mut shapes = a
            .points
            .overlay(&b.points, OverlayRule::Difference, FillRule::EvenOdd);
        if shapes.is_empty() {
            return None;
        }
        assert!(shapes.len() == 1);
        assert!(shapes[0].len() == 1);
        let s = std::mem::take(&mut shapes[0][0]);
        Some(Polygon { points: s })
    }

    // // Union of polygons a and b (a ∪ b)
    // pub fn union(a: &Polygon, b: &Polygon) -> Option<Polygon> {
    //     let overlay_a = a.to_overlay_polygon();
    //     let overlay_b = b.to_overlay_polygon();

    //     let clipper = PolygonClipper::<f32>::new(&overlay_a);
    //     let result = clipper.clip(&overlay_b, BooleanOp::Union);

    //     match result {
    //         Some(multipolygon) => {
    //             // Take the first polygon from the result
    //             // In a complete implementation, we would handle multiple polygons
    //             if multipolygon.is_empty() {
    //                 None
    //             } else {
    //                 let first_polygon = multipolygon.iter().next().unwrap().clone();
    //                 Some(Polygon::from_overlay_polygon(first_polygon))
    //             }
    //         }
    //         None => None,
    //     }
    // }

    // // Intersection of polygons a and b (a ∩ b)
    // pub fn intersection(a: &Polygon, b: &Polygon) -> Option<Polygon> {
    //     let overlay_a = a.to_overlay_polygon();
    //     let overlay_b = b.to_overlay_polygon();

    //     let clipper = PolygonClipper::<f32>::new(&overlay_a);
    //     let result = clipper.clip(&overlay_b, BooleanOp::Intersection);

    //     match result {
    //         Some(multipolygon) => {
    //             // Take the first polygon from the result
    //             // In a complete implementation, we would handle multiple polygons
    //             if multipolygon.is_empty() {
    //                 None
    //             } else {
    //                 let first_polygon = multipolygon.iter().next().unwrap().clone();
    //                 Some(Polygon::from_overlay_polygon(first_polygon))
    //             }
    //         }
    //         None => None,
    //     }
    // }

    // Get bounding box
    fn bounding_box(&self) -> (Point, Point) {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for point in &self.points {
            min_x = min_x.min(point[0]);
            min_y = min_y.min(point[1]);
            max_x = max_x.max(point[0]);
            max_y = max_y.max(point[1]);
        }

        ([min_x, min_y], [max_x, max_y])
    }
}

#[derive(Debug)]
pub enum KeyType {
    White,
    Black,
}

#[derive(Debug)]
pub struct PianoKey {
    pub key_type: KeyType,
    pub midi_note: u8,
    pub shape: Polygon,
}

impl PianoKey {
    // Create a white key with the given MIDI note and position
    pub fn new_white(midi_note: u8, position: usize, black_keys: &[Polygon]) -> Self {
        // Standard dimensions for a piano key (in arbitrary units)
        let white_key_width = 23.5;
        let white_key_length = 150.0;
        let x_offset = position as f32 * white_key_width;

        // Define the initial shape as a rectangle
        let initial_shape = Polygon::rectangle(x_offset, 0.0, white_key_width, white_key_length);

        // Apply boolean difference operations with black keys
        let mut final_shape = initial_shape;
        for black_key in black_keys {
            if let Some(diff_result) = Polygon::difference(&final_shape, black_key) {
                final_shape = diff_result;
            }
        }

        Self {
            key_type: KeyType::White,
            midi_note,
            shape: final_shape,
        }
    }

    // Create a black key with the given MIDI note and position
    pub fn new_black(midi_note: u8, position: f32) -> Self {
        // Standard dimensions for a piano key (in arbitrary units)
        let white_key_width = 23.5;
        let black_key_width = 15.0;
        let black_key_length = 100.0;
        let x_offset = position * white_key_width - (black_key_width / 2.0);

        // Define the shape as a rectangle
        let shape = Polygon::rectangle(x_offset, 0.0, black_key_width, black_key_length);

        Self {
            key_type: KeyType::Black,
            midi_note,
            shape,
        }
    }

    // Get the polygon representation of the key's shape
    pub fn get_polygon(&self) -> &Polygon {
        &self.shape
    }
}

pub struct PianoOctave {
    pub keys: Vec<PianoKey>,
}

impl PianoOctave {
    // Create a standard octave starting from the given MIDI note
    pub fn new(starting_midi_note: u8) -> Self {
        // First, create black key shapes for boolean operations
        let black_keys_positions = [0.5, 1.5, 3.5, 4.5, 5.5];
        let mut black_key_shapes = Vec::new();

        for position in black_keys_positions.iter() {
            let white_key_width = 23.5;
            let black_key_width = 15.0;
            let black_key_length = 100.0;
            let x_offset = position * white_key_width - (black_key_width / 2.0);

            black_key_shapes.push(Polygon::rectangle(
                x_offset,
                0.0,
                black_key_width,
                black_key_length,
            ));
        }

        let mut keys = Vec::new();

        // Create white keys (C, D, E, F, G, A, B) with boolean difference applied
        let white_key_offsets = [0, 2, 4, 5, 7, 9, 11]; // Semitone offsets from C
        for (i, offset) in white_key_offsets.iter().enumerate() {
            let midi_note = starting_midi_note + offset;

            // Determine which black keys affect this white key
            let relevant_black_keys: Vec<Polygon> = black_key_shapes
                .iter()
                .filter(|bk| {
                    let (min, max) = bk.bounding_box();
                    let key_x = i as f32 * 23.5;
                    let key_x_end = key_x + 23.5;

                    // Check if black key overlaps with this white key
                    min[0] < key_x_end && max[0] > key_x
                })
                .cloned()
                .collect();

            keys.push(PianoKey::new_white(midi_note, i, &relevant_black_keys));
        }

        // Create black keys
        let black_key_offsets = [1, 3, 6, 8, 10]; // Semitone offsets from C
        
        for (position, offset) in black_keys_positions
            .iter()
            .zip(black_key_offsets.iter())
        {
            let midi_note = starting_midi_note + offset;
            keys.push(PianoKey::new_black(midi_note, *position));
        }

        Self { keys }
    }

    // // Method to render the piano octave (placeholder for actual rendering logic)
    // pub fn render(&self) {
    //     println!("Piano Octave with {} keys:", self.keys.len());
    //     for key in &self.keys {
    //         println!("  {} Key: {}, Vertices: {}",
    //             match key.key_type {
    //                 KeyType::White => "White",
    //                 KeyType::Black => "Black",
    //             },
    //             key.note_name,
    //             key.shape.points.len()
    //         );
    //     }
    // }

    // Method to get the bounding box of the entire octave
    pub fn get_bounding_box(&self) -> (Point, Point) {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for key in &self.keys {
            let (key_min, key_max) = key.shape.bounding_box();
            min_x = min_x.min(key_min[0]);
            min_y = min_y.min(key_min[1]);
            max_x = max_x.max(key_max[0]);
            max_y = max_y.max(key_max[1]);
        }

        ([min_x, min_y], [max_x, max_y])
    }
}

// fn main() {
//     // Create a piano octave starting from C4
//     let octave = PianoOctave::new("C4");

//     // Render the octave
//     octave.render();

//     // Get and print the bounding box
//     let (min, max) = octave.get_bounding_box();
//     println!("Bounding box: ({}, {}) to ({}, {})", min[0], min[1], max[0], max[1]);

//     // Example of using other boolean operations
//     println!("\nDemonstrating boolean operations with i_overlay:");

//     let rect1 = Polygon::rectangle(10.0, 10.0, 50.0, 50.0);
//     let rect2 = Polygon::rectangle(30.0, 30.0, 50.0, 50.0);

//     if let Some(union_result) = Polygon::union(&rect1, &rect2) {
//         println!("Union result has {} vertices", union_result.points.len());
//     }

//     if let Some(intersection) = Polygon::intersection(&rect1, &rect2) {
//         println!("Intersection result has {} vertices", intersection.points.len());
//     }

//     if let Some(difference) = Polygon::difference(&rect1, &rect2) {
//         println!("Difference result has {} vertices", difference.points.len());
//     }
// }
