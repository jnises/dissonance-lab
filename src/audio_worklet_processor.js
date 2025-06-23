// ADSR Envelope class for handling amplitude envelope
class ADSREnvelope {
    constructor(params = {}) {
        this.attack = params.attack || 0.01;
        this.decay = params.decay || 0.1;
        this.sustain = params.sustain || 0.6;
        this.release = params.release || 0.2;
    }
    
    // Set new envelope parameters
    setParameters(params) {
        if (params.attack !== undefined) this.attack = params.attack;
        if (params.decay !== undefined) this.decay = params.decay;
        if (params.sustain !== undefined) this.sustain = params.sustain;
        if (params.release !== undefined) this.release = params.release;
    }
    
    // Calculate envelope value based on voice state and timing
    calculate(voice, currentTime, amplitudeJitter = 0) {
        // Time since note started
        const timeSinceStart = currentTime - voice.startTime;
        
        // Add slight wobble/variation to envelope for more organic sounds
        const envVariation = 1 + (Math.random() * 2 - 1) * amplitudeJitter;
        
        let gain = 0;
        
        switch (voice.state) {
            case 'attack':
                if (timeSinceStart < this.attack) {
                    // Ramp up during attack phase with slight variations
                    gain = (timeSinceStart / this.attack) * envVariation;
                } else {
                    // Move to decay phase
                    voice.state = 'decay';
                }
                break;
                
            case 'decay':
                if (timeSinceStart < this.attack + this.decay) {
                    // Decay from peak (1.0) to sustain level
                    const decayProgress = (timeSinceStart - this.attack) / this.decay;
                    gain = (1.0 - (1.0 - this.sustain) * decayProgress) * envVariation;
                } else {
                    // Move to sustain phase
                    voice.state = 'sustain';
                    gain = this.sustain * envVariation;
                }
                break;
                
            case 'sustain':
                // Add subtle fluctuations to sustain for variations
                gain = this.sustain * envVariation;
                break;
                
            case 'release':
                const timeSinceRelease = currentTime - voice.releaseStart;
                if (timeSinceRelease < this.release) {
                    // Ramp down during release phase
                    gain = this.sustain * (1 - timeSinceRelease / this.release);
                } else {
                    // Voice is done, return 0 and we'll remove it after
                    gain = 0;
                    return 0;
                }
                break;
        }
        
        voice.gain = gain;
        return gain * voice.velocity;
    }
}

class GuitarStringProcessor extends AudioWorkletProcessor {
    constructor() {
        super();
        // Voice management
        this.voices = new Map();
        this.maxVoices = 6; // Typical guitar has 6 strings
        
        // ADSR parameters adjusted for guitar sounds
        this.adsrParams = {
            attack: 0.005,   // Very quick attack for guitar pluck
            decay: 0.1,      // Fast initial decay
            sustain: 0.7,    // Sustain level for held notes
            release: 0.3     // Release time
        };
        
        // Create ADSR envelope
        this.adsr = new ADSREnvelope(this.adsrParams);

        // Guitar synthesis parameters
        this.guitarParams = {
            // String damping coefficient (lower = longer sustain)
            damping: 0.995,
            // String tension (affects initial brightness)
            brightness: 0.8,
            // String stiffness (affects inharmonicity)
            stiffness: 0.4,
            // Body resonance amount
            bodyResonance: 0.7,
            // Pick position (0.0-1.0, affects harmonic content)
            pickPosition: 0.2,
            // String-to-string coupling (sympathetic resonance)
            coupling: 0.02
        };

        // Guitar modes
        this.guitarMode = 'normal'; // 'normal', 'palm-muted', 'harmonics'
        
        // Randomization parameters for natural variations
        this.randomness = {
            pitchJitter: 0.0003,  // Subtle random pitch variations
            pluckJitter: 0.1      // Variations in pluck strength and position
        };

        this.updateGuitarMode();

        this.port.onmessage = (event) => {
            switch (event.data.type) {
                case 'NoteOn':
                    this.noteOn(event.data.note, event.data.velocity);
                    break;
                case 'NoteOff':
                    this.noteOff(event.data.note);
                    break;
                case 'UpdateADSR':
                    if (event.data.adsr) {
                        this.adsrParams = { ...this.adsrParams, ...event.data.adsr };
                        this.adsr.setParameters(this.adsrParams);
                    }
                    break;
                case 'UpdateGuitar':
                    if (event.data.guitarParams) {
                        this.guitarParams = { ...this.guitarParams, ...event.data.guitarParams };
                    }
                    break;
                case 'SetGuitarMode':
                    this.guitarMode = event.data.mode || 'normal';
                    this.updateGuitarMode();
                    break;
            }
        };
    }
    
    // Update parameters based on guitar playing mode
    updateGuitarMode() {
        switch (this.guitarMode) {
            case 'normal':
                // Standard guitar sounds
                this.adsrParams = {
                    attack: 0.005, 
                    decay: 0.1, 
                    sustain: 0.7, 
                    release: 0.3
                };
                this.adsr.setParameters(this.adsrParams);
                this.guitarParams.damping = 0.995;
                this.guitarParams.brightness = 0.8;
                this.guitarParams.pickPosition = 0.2;
                break;
                
            case 'palm-muted':
                // Palm-muted guitar technique
                this.adsrParams = {
                    attack: 0.001, 
                    decay: 0.05, 
                    sustain: 0.3, 
                    release: 0.1
                };
                this.adsr.setParameters(this.adsrParams);
                this.guitarParams.damping = 0.96;
                this.guitarParams.brightness = 0.5;
                this.guitarParams.pickPosition = 0.15;
                break;
                
            case 'harmonics':
                // Natural harmonics
                this.adsrParams = {
                    attack: 0.002, 
                    decay: 0.3, 
                    sustain: 0.8, 
                    release: 0.5
                };
                this.adsr.setParameters(this.adsrParams);
                this.guitarParams.damping = 0.998;
                this.guitarParams.brightness = 1.0;
                this.guitarParams.pickPosition = 0.5; // Node position for harmonics
                break;
        }
    }
    
    // Convert MIDI note to frequency
    noteToFrequency(note) {
        return 440 * Math.pow(2, (note - 69) / 12);
    }
    
    // Initialize a Karplus-Strong string delay line
    initializeDelayLine(frequency) {
        // Calculate delay length for this frequency
        const delayLength = Math.round(sampleRate / frequency);
        
        // Initialize with noise (pluck)
        const delayLine = new Float32Array(delayLength);
        for (let i = 0; i < delayLength; i++) {
            delayLine[i] = Math.random() * 2 - 1;
        }
        
        // Apply pick position filtering to simulate picking at a specific point
        this.applyPickPositionFilter(delayLine, this.guitarParams.pickPosition);
        
        return {
            buffer: delayLine,
            position: 0,
            length: delayLength
        };
    }
    
    // Apply pick position filter to initial excitation
    applyPickPositionFilter(buffer, pickPosition) {
        // Simulate the effect of picking position on initial spectrum
        const combFreq = 1.0 / (pickPosition * buffer.length);
        
        // Apply a comb filter effect
        for (let i = 1; i < buffer.length; i++) {
            const phase = (i / buffer.length) / combFreq;
            const scaleFactor = 0.5 - 0.5 * Math.cos(2 * Math.PI * phase);
            buffer[i] *= scaleFactor;
        }
        
        // Normalize the buffer after filtering
        let max = 0;
        for (let i = 0; i < buffer.length; i++) {
            max = Math.max(max, Math.abs(buffer[i]));
        }
        
        if (max > 0) {
            for (let i = 0; i < buffer.length; i++) {
                buffer[i] /= max;
            }
        }
    }
    
    // Start a new voice (pluck a string)
    noteOn(note, velocity) {
        // Normalize velocity to 0.0-1.0
        const normalizedVelocity = velocity / 127;
        
        // If we already have this note playing, turn it off first
        if (this.voices.has(note)) {
            this.voices.get(note).state = 'release';
            this.voices.get(note).releaseStart = currentTime;
        }
        
        // Apply slight random variations for naturalness
        const pitchVariation = 1 + (Math.random() * 2 - 1) * this.randomness.pitchJitter;
        const baseFreq = this.noteToFrequency(note) * pitchVariation;
        
        // Create a delay line for Karplus-Strong string
        const delayLine = this.initializeDelayLine(baseFreq);
        
        // Create a new voice with guitar-specific parameters
        this.voices.set(note, {
            note: note,
            frequency: baseFreq,
            delayLine: delayLine,
            velocity: normalizedVelocity,
            startTime: currentTime,
            state: 'attack',
            gain: 0,
            releaseStart: 0,
            // Resonant filter state for string and body filtering
            stringFilter: {
                y1: 0,
                y2: 0
            },
            bodyFilter: {
                y1: 0,
                y2: 0,
                // Slight variation in body resonance for each string
                resonanceFreq: 200 + Math.random() * 2000
            }
        });
        
        // Apply sympathetic resonance to other strings
        if (this.guitarParams.coupling > 0) {
            this.applySympatheticResonance(note, normalizedVelocity);
        }
        
        // If we've exceeded the max voices, remove the oldest one
        if (this.voices.size > this.maxVoices) {
            let oldestNote = this.voices.keys().next().value;
            let oldestTime = Infinity;
            
            for (const [noteNum, voice] of this.voices.entries()) {
                if (voice.startTime < oldestTime) {
                    oldestTime = voice.startTime;
                    oldestNote = noteNum;
                }
            }
            
            this.voices.delete(oldestNote);
        }
    }
    
    // Apply sympathetic resonance to other strings
    applySympatheticResonance(playedNote, velocity) {
        const playedFreq = this.noteToFrequency(playedNote);
        
        // Excite other strings based on harmonic relationships
        for (const [noteNum, voice] of this.voices.entries()) {
            if (noteNum !== playedNote) {
                const freq = voice.frequency;
                
                // Calculate frequency ratio to check for harmonic relationship
                const ratio = playedFreq > freq ? playedFreq / freq : freq / playedFreq;
                const harmonicCloseness = 1 / (Math.abs(Math.round(ratio) - ratio) + 0.01);
                
                // Apply coupling force proportional to harmonic relationship and velocity
                if (harmonicCloseness > 5) {  // Is it close to a harmonic?
                    const couplingForce = velocity * this.guitarParams.coupling * 
                                         harmonicCloseness / 10;
                                         
                    // Disturb the other string's delay line slightly to simulate coupling
                    const dl = voice.delayLine;
                    for (let i = 0; i < dl.buffer.length; i++) {
                        dl.buffer[i] += (Math.random() * 2 - 1) * couplingForce;
                    }
                }
            }
        }
    }
    
    // Release a voice
    noteOff(note) {
        if (this.voices.has(note)) {
            const voice = this.voices.get(note);
            voice.state = 'release';
            voice.releaseStart = currentTime;
        }
    }
    
    // Calculate the ADSR envelope level using the ADSREnvelope class
    calculateEnvelope(voice) {
        return this.adsr.calculate(voice, currentTime, this.randomness.pluckJitter * 0.1);
    }
    
    // Apply Karplus-Strong string algorithm and body filtering
    processString(voice, outputBuffer, sampleCount) {
        const dl = voice.delayLine;
        const damping = this.guitarParams.damping;
        const brightness = this.guitarParams.brightness;
        const stiffness = this.guitarParams.stiffness;
        
        // Calculate body resonance coefficient
        const bodyCoeff = this.guitarParams.bodyResonance;
        
        // Process each sample
        for (let i = 0; i < sampleCount; i++) {
            // Read from the delay line
            const pos = dl.position;
            const nextPos = (pos + 1) % dl.length;
            
            // Get the current and next sample
            const sample = dl.buffer[pos];
            const nextSample = dl.buffer[nextPos];
            
            // Karplus-Strong low-pass filter with adjustable brightness
            const stringOut = damping * (sample + brightness * (nextSample - sample));
            
            // Simple inharmonicity simulation (string stiffness)
            const stiffOut = voice.stringFilter.y1;
            const allpassOut = stringOut - stiffness * stiffOut;
            voice.stringFilter.y1 = allpassOut;
            
            // Body resonance filter - simple resonant filter
            const bodyResonFreq = voice.bodyFilter.resonanceFreq / sampleRate;
            const bodyQ = 0.9; // Resonance sharpness
            const bodyW0 = 2 * Math.PI * bodyResonFreq;
            const bodyAlpha = Math.sin(bodyW0) / (2 * bodyQ);
            
            // Resonant filter calculation (one-pole)
            const bodyInput = allpassOut + bodyCoeff * voice.bodyFilter.y1;
            const bodyOut = bodyInput * (1 - bodyAlpha) + voice.bodyFilter.y1 * bodyAlpha;
            voice.bodyFilter.y1 = bodyOut;
            
            // Write the filtered output back to the delay line
            dl.buffer[pos] = allpassOut;
            
            // Combine string and body resonance output
            outputBuffer[i] += (allpassOut * 0.7 + bodyOut * 0.3);
            
            // Move the delay line position
            dl.position = nextPos;
        }
    }

    process(inputs, outputs, parameters) {
        const output = outputs[0];
        
        // Initialize output channels with silence
        for (let channel = 0; channel < output.length; ++channel) {
            output[channel].fill(0);
        }
        
        // Process each active voice
        const finishedVoices = [];
        
        for (const [note, voice] of this.voices.entries()) {
            // Calculate the envelope value for this voice
            const envelopeGain = this.calculateEnvelope(voice);
            
            // If the voice is finished (release completed), mark for removal
            if (voice.state === 'release' && envelopeGain === 0) {
                finishedVoices.push(note);
                continue;
            }
            
            // Temporary buffer for this string's output
            const stringOutput = new Float32Array(output[0].length);
            
            // Process the string using Karplus-Strong algorithm
            this.processString(voice, stringOutput, output[0].length);
            
            // Apply envelope and add to output channels
            for (let channel = 0; channel < output.length; ++channel) {
                const outputChannel = output[channel];
                for (let i = 0; i < outputChannel.length; ++i) {
                    outputChannel[i] += stringOutput[i] * envelopeGain * voice.velocity;
                }
            }
        }
        
        // Remove finished voices
        finishedVoices.forEach(note => this.voices.delete(note));
        
        // Return true to keep the processor alive
        return true;
    }
}

registerProcessor('sine-processor', GuitarStringProcessor);