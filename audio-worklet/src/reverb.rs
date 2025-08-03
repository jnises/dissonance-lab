// AI generated

const MS_TO_S: f32 = 0.001;

// Delay times in ms for comb filters based on classic Schroeder reverb
const COMB_FILTER_DELAYS_MS: [f32; 4] = [29.7, 37.1, 41.1, 43.7];
const COMB_FILTER_FEEDBACK: f32 = 0.84;
const COMB_FILTER_DAMPING: f32 = 0.2;

// Delay times for allpass filters
const ALLPASS_FILTER_DELAYS_MS: [f32; 2] = [5.0, 1.7];
const ALLPASS_FILTER_FEEDBACK: f32 = 0.5;

const DEFAULT_ROOM_SIZE: f32 = 0.5;
const DEFAULT_DAMPING: f32 = 0.5;
const DEFAULT_WET_LEVEL: f32 = 0.33;
const DEFAULT_DRY_LEVEL: f32 = 0.4;
const DEFAULT_WIDTH: f32 = 1.0;

/// Shroeder reverb
pub struct Reverb {
    // Reverb parameters
    room_size: f32,
    damping: f32,
    wet_level: f32,
    dry_level: f32,
    width: f32,

    // Comb filters for main reverb body
    comb_filters: Vec<CombFilter>,
    // All-pass filters for diffusion
    allpass_filters: Vec<AllpassFilter>,
}

struct CombFilter {
    delay_line: Vec<f32>,
    index: usize,
    feedback: f32,
    damping: f32,
    dampening_value: f32,
}

struct AllpassFilter {
    delay_line: Vec<f32>,
    index: usize,
    feedback: f32,
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        let comb_filters = COMB_FILTER_DELAYS_MS
            .iter()
            .map(|delay| {
                let buffer_size = (delay * MS_TO_S * sample_rate) as usize;
                CombFilter {
                    delay_line: vec![0.0; buffer_size],
                    index: 0,
                    feedback: COMB_FILTER_FEEDBACK,
                    damping: COMB_FILTER_DAMPING,
                    dampening_value: 0.0,
                }
            })
            .collect();

        let allpass_filters = ALLPASS_FILTER_DELAYS_MS
            .iter()
            .map(|delay| {
                let buffer_size = (delay * MS_TO_S * sample_rate) as usize;
                AllpassFilter {
                    delay_line: vec![0.0; buffer_size],
                    index: 0,
                    feedback: ALLPASS_FILTER_FEEDBACK,
                }
            })
            .collect();

        Reverb {
            room_size: DEFAULT_ROOM_SIZE,
            damping: DEFAULT_DAMPING,
            wet_level: DEFAULT_WET_LEVEL,
            dry_level: DEFAULT_DRY_LEVEL,
            width: DEFAULT_WIDTH,
            comb_filters,
            allpass_filters,
        }
    }

    pub fn set_room_size(&mut self, size: f32) {
        self.room_size = size.clamp(0.0, 1.0);
        self.update_parameters();
    }

    pub fn set_damping(&mut self, damping: f32) {
        self.damping = damping.clamp(0.0, 1.0);
        self.update_parameters();
    }

    pub fn set_wet_level(&mut self, level: f32) {
        self.wet_level = level.clamp(0.0, 1.0);
    }

    pub fn set_dry_level(&mut self, level: f32) {
        self.dry_level = level.clamp(0.0, 1.0);
    }

    pub fn set_width(&mut self, width: f32) {
        self.width = width.clamp(0.0, 1.0);
    }

    fn update_parameters(&mut self) {
        const ROOM_SIZE_FACTOR: f32 = 0.6;
        const ROOM_SIZE_OFFSET: f32 = 0.4;

        for filter in &mut self.comb_filters {
            filter.feedback = self.room_size * ROOM_SIZE_FACTOR + ROOM_SIZE_OFFSET;
            filter.damping = self.damping;
        }
    }

    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let mut output = 0.0;

        for filter in &mut self.comb_filters {
            output += filter.process(input);
        }
        output /= self.comb_filters.len() as f32;

        for filter in &mut self.allpass_filters {
            output = filter.process(output);
        }

        self.dry_level * input + self.wet_level * output
    }
}

impl CombFilter {
    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.delay_line[self.index];
        self.dampening_value = output * (1.0 - self.damping) + self.dampening_value * self.damping;
        let new_value = input + self.dampening_value * self.feedback;
        self.delay_line[self.index] = new_value;
        self.index = (self.index + 1) % self.delay_line.len();
        output
    }
}

impl AllpassFilter {
    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.delay_line[self.index];
        let output = -input * self.feedback + delayed;
        self.delay_line[self.index] = input + delayed * self.feedback;
        self.index = (self.index + 1) % self.delay_line.len();
        output
    }
}
