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
    
    // Sample rate
    sample_rate: f32,
}

struct CombFilter {
    buffer: Vec<f32>,
    buffer_size: usize,
    index: usize,
    feedback: f32,
    damping: f32,
    dampening_value: f32,
}

struct AllpassFilter {
    buffer: Vec<f32>,
    buffer_size: usize,
    index: usize,
    feedback: f32,
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        // Delay times in ms for comb filters based on classic Schroeder reverb
        let comb_delays = vec![29.7, 37.1, 41.1, 43.7];
        // Delay times for allpass filters
        let allpass_delays = vec![5.0, 1.7];
        
        // Create comb filters
        let mut comb_filters = Vec::new();
        for &delay in &comb_delays {
            let buffer_size = (delay * 0.001 * sample_rate) as usize;
            comb_filters.push(CombFilter {
                buffer: vec![0.0; buffer_size],
                buffer_size,
                index: 0,
                feedback: 0.84,
                damping: 0.2,
                dampening_value: 0.0,
            });
        }
        
        // Create allpass filters
        let mut allpass_filters = Vec::new();
        for &delay in &allpass_delays {
            let buffer_size = (delay * 0.001 * sample_rate) as usize;
            allpass_filters.push(AllpassFilter {
                buffer: vec![0.0; buffer_size],
                buffer_size,
                index: 0,
                feedback: 0.5,
            });
        }
        
        Reverb {
            room_size: 0.5,
            damping: 0.5,
            wet_level: 0.33,
            dry_level: 0.4,
            width: 1.0,
            comb_filters,
            allpass_filters,
            sample_rate,
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
        for filter in &mut self.comb_filters {
            filter.feedback = self.room_size * 0.6 + 0.4;
            filter.damping = self.damping;
        }
    }
    
    // Process a single audio sample through the reverb
    pub fn process(&mut self, input: f32) -> f32 {
        // Initialize the output
        let mut output = 0.0;
        
        // Process through comb filters in parallel
        for filter in &mut self.comb_filters {
            output += filter.process(input);
        }
        
        // Average the comb filter outputs
        output /= self.comb_filters.len() as f32;
        
        // Pass the signal through allpass filters in series
        for filter in &mut self.allpass_filters {
            output = filter.process(output);
        }
        
        // Mix dry and wet signals
        let wet_gain = self.wet_level;
        let dry_gain = self.dry_level;
        
        dry_gain * input + wet_gain * output
    }
    
    // Process a stereo sample (left, right) 
    pub fn process_stereo(&mut self, input_left: f32, input_right: f32) -> (f32, f32) {
        // For simple stereo widening, we can process left and right separately
        // and then adjust the width
        let input_mono = (input_left + input_right) * 0.5;
        let reverb_out = self.process(input_mono);
        
        // Widen the stereo image by offsetting left and right
        let left = reverb_out;
        let right = if self.width < 1.0 {
            input_mono * (1.0 - self.width) + reverb_out * self.width
        } else {
            reverb_out
        };
        
        (
            self.dry_level * input_left + left * self.wet_level,
            self.dry_level * input_right + right * self.wet_level
        )
    }
    
    /// Process a buffer of mono samples
    /// 
    /// * `input` - Buffer of input samples
    /// * `output` - Buffer to store the processed samples (must be the same size as input)
    pub fn process_buffer(&mut self, input: &[f32], output: &mut [f32]) {
        assert_eq!(input.len(), output.len(), "Input and output buffers must be the same length");
        
        for i in 0..input.len() {
            output[i] = self.process(input[i]);
        }
    }
    
    /// Process a buffer of stereo samples efficiently
    /// 
    /// * `input_left` - Buffer of left channel input samples
    /// * `input_right` - Buffer of right channel input samples
    /// * `output_left` - Buffer to store the processed left channel samples
    /// * `output_right` - Buffer to store the processed right channel samples
    pub fn process_stereo_buffer(
        &mut self,
        input_left: &[f32],
        input_right: &[f32],
        output_left: &mut [f32],
        output_right: &mut [f32]
    ) {
        let buffer_size = input_left.len();
        assert_eq!(input_right.len(), buffer_size, "Input buffers must be the same length");
        assert_eq!(output_left.len(), buffer_size, "Output buffers must be the same length as input");
        assert_eq!(output_right.len(), buffer_size, "Output buffers must be the same length as input");
        
        // Pre-calculate constants for the entire buffer to avoid repeated calculations
        let wet_gain = self.wet_level;
        let dry_gain = self.dry_level;
        
        // Optional: Create a temporary buffer for the reverb output to avoid recomputing
        let mut reverb_buffer = vec![0.0; buffer_size];
        
        // Step 1: Mix left and right to mono and calculate mono reverb for the whole buffer
        for i in 0..buffer_size {
            let input_mono = (input_left[i] + input_right[i]) * 0.5;
            
            // Calculate reverb for mono signal
            let mut output = 0.0;
            
            // Process through comb filters in parallel
            for filter in &mut self.comb_filters {
                output += filter.process(input_mono);
            }
            
            // Average the comb filter outputs
            output /= self.comb_filters.len() as f32;
            
            // Pass the signal through allpass filters in series
            for filter in &mut self.allpass_filters {
                output = filter.process(output);
            }
            
            reverb_buffer[i] = output;
        }
        
        // Step 2: Apply stereo width and mix dry/wet signals
        for i in 0..buffer_size {
            let reverb_out = reverb_buffer[i];
            let input_mono = (input_left[i] + input_right[i]) * 0.5;
            
            // Apply width to the reverb output
            let left_reverb = reverb_out;
            let right_reverb = if self.width < 1.0 {
                input_mono * (1.0 - self.width) + reverb_out * self.width
            } else {
                reverb_out
            };
            
            // Mix dry and wet signals
            output_left[i] = dry_gain * input_left[i] + wet_gain * left_reverb;
            output_right[i] = dry_gain * input_right[i] + wet_gain * right_reverb;
        }
    }
    
    /// More efficient mono buffer processing implementation
    /// This version processes the comb filters in parallel across the entire buffer first,
    /// then processes all-pass filters on the result.
    pub fn process_buffer_optimized(&mut self, input: &[f32], output: &mut [f32]) {
        assert_eq!(input.len(), output.len(), "Input and output buffers must be the same length");
        let buffer_size = input.len();
        
        // Pre-calculate constants
        let wet_gain = self.wet_level;
        let dry_gain = self.dry_level;
        
        // Step 1: Apply comb filters in parallel to the entire buffer first
        let mut temp_buffer = vec![0.0; buffer_size];
        
        for filter in &mut self.comb_filters {
            let mut filter_output = vec![0.0; buffer_size];
            for i in 0..buffer_size {
                filter_output[i] = filter.process(input[i]);
            }
            
            // Sum into the temp buffer
            for i in 0..buffer_size {
                temp_buffer[i] += filter_output[i];
            }
        }
        
        // Average the comb filter outputs
        let comb_count = self.comb_filters.len() as f32;
        for i in 0..buffer_size {
            temp_buffer[i] /= comb_count;
        }
        
        // Step 2: Apply allpass filters in series to the entire buffer
        for filter in &mut self.allpass_filters {
            for i in 0..buffer_size {
                temp_buffer[i] = filter.process(temp_buffer[i]);
            }
        }
        
        // Step 3: Mix dry and wet signals
        for i in 0..buffer_size {
            output[i] = dry_gain * input[i] + wet_gain * temp_buffer[i];
        }
    }
}

impl CombFilter {
    fn process(&mut self, input: f32) -> f32 {
        // Read the value from the delay line
        let output = self.buffer[self.index];
        
        // Apply damping to the feedback
        self.dampening_value = output * (1.0 - self.damping) + self.dampening_value * self.damping;
        
        // Update the delay line
        let new_value = input + self.dampening_value * self.feedback;
        self.buffer[self.index] = new_value;
        
        // Update the buffer index
        self.index = (self.index + 1) % self.buffer_size;
        
        output
    }
    
    /// Process a buffer through the comb filter
    fn process_buffer(&mut self, input: &[f32], output: &mut [f32]) {
        assert_eq!(input.len(), output.len(), "Input and output buffers must be the same length");
        
        for i in 0..input.len() {
            // Read the value from the delay line
            let delayed = self.buffer[self.index];
            
            // Apply damping to the feedback
            self.dampening_value = delayed * (1.0 - self.damping) + self.dampening_value * self.damping;
            
            // Update the delay line
            let new_value = input[i] + self.dampening_value * self.feedback;
            self.buffer[self.index] = new_value;
            
            // Update the buffer index
            self.index = (self.index + 1) % self.buffer_size;
            
            output[i] = delayed;
        }
    }
}

impl AllpassFilter {
    fn process(&mut self, input: f32) -> f32 {
        // Read the value from the delay line
        let delayed = self.buffer[self.index];
        
        // Calculate output sample
        let output = -input * self.feedback + delayed;
        
        // Update the delay line
        self.buffer[self.index] = input + delayed * self.feedback;
        
        // Update the buffer index
        self.index = (self.index + 1) % self.buffer_size;
        
        output
    }
    
    /// Process a buffer through the allpass filter
    fn process_buffer(&mut self, input: &[f32], output: &mut [f32]) {
        assert_eq!(input.len(), output.len(), "Input and output buffers must be the same length");
        
        for i in 0..input.len() {
            // Read the value from the delay line
            let delayed = self.buffer[self.index];
            
            // Calculate output sample
            output[i] = -input[i] * self.feedback + delayed;
            
            // Update the delay line
            self.buffer[self.index] = input[i] + delayed * self.feedback;
            
            // Update the buffer index
            self.index = (self.index + 1) % self.buffer_size;
        }
    }
}
