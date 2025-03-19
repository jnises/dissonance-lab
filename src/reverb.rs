/// Shroeder reverb
/// AI generated. Seems to work alright
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
        // Delay times in ms for comb filters based on classic Schroeder reverb
        let comb_delays = vec![29.7, 37.1, 41.1, 43.7];
        // Delay times for allpass filters
        let allpass_delays = vec![5.0, 1.7];

        let comb_filters = comb_delays
            .into_iter()
            .map(|delay| {
                let buffer_size = (delay * 0.001 * sample_rate) as usize;
                CombFilter {
                    delay_line: vec![0.0; buffer_size],
                    index: 0,
                    feedback: 0.84,
                    damping: 0.2,
                    dampening_value: 0.0,
                }
            })
            .collect();

        let allpass_filters = allpass_delays
            .into_iter()
            .map(|delay| {
                let buffer_size = (delay * 0.001 * sample_rate) as usize;
                AllpassFilter {
                    delay_line: vec![0.0; buffer_size],
                    index: 0,
                    feedback: 0.5,
                }
            })
            .collect();

        Reverb {
            room_size: 0.5,
            damping: 0.5,
            wet_level: 0.33,
            dry_level: 0.4,
            width: 1.0,
            comb_filters,
            allpass_filters,
        }
    }

    #[allow(dead_code)]
    pub fn set_room_size(&mut self, size: f32) {
        self.room_size = size.clamp(0.0, 1.0);
        self.update_parameters();
    }

    #[allow(dead_code)]
    pub fn set_damping(&mut self, damping: f32) {
        self.damping = damping.clamp(0.0, 1.0);
        self.update_parameters();
    }

    #[allow(dead_code)]
    pub fn set_wet_level(&mut self, level: f32) {
        self.wet_level = level.clamp(0.0, 1.0);
    }

    #[allow(dead_code)]
    pub fn set_dry_level(&mut self, level: f32) {
        self.dry_level = level.clamp(0.0, 1.0);
    }

    #[allow(dead_code)]
    pub fn set_width(&mut self, width: f32) {
        self.width = width.clamp(0.0, 1.0);
    }

    fn update_parameters(&mut self) {
        for filter in &mut self.comb_filters {
            filter.feedback = self.room_size * 0.6 + 0.4;
            filter.damping = self.damping;
        }
    }

    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let mut output = 0.0;

        // Process through comb filters in parallel
        for filter in &mut self.comb_filters {
            output += filter.process(input);
        }
        output /= self.comb_filters.len() as f32;

        // Pass the signal through allpass filters in series
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
