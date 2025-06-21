// AudioWorklet processor with basic synthesis
console.log('Loading js-audio-processor.js script');

class DissonanceAudioProcessor extends AudioWorkletProcessor {
    constructor() {
        super();
        this.phase = 0;
        this.frequency = 440; // A4
        this.amplitude = 0.1;
        this.isPlaying = false;
        
        console.log('Dissonance audio processor created, sample rate:', sampleRate);
        
        // Handle messages from the main thread
        this.port.onmessage = (event) => {
            console.log('AudioWorklet received message:', event.data);
            this.handleMessage(event.data);
        };
    }
    
    handleMessage(message) {
        if (message && message.type) {
            switch (message.type) {
                case 'midi':
                    this.handleMidi(message.data);
                    break;
                case 'config':
                    this.handleConfig(message.data);
                    break;
                default:
                    console.log('Unknown message type:', message.type);
            }
        }
    }
    
    handleMidi(midiData) {
        // Simple MIDI handling - just note on/off
        if (midiData.status === 144) { // Note on
            this.frequency = 440 * Math.pow(2, (midiData.data1 - 69) / 12);
            this.isPlaying = midiData.data2 > 0;
            console.log('Note on:', this.frequency, 'Hz');
        } else if (midiData.status === 128) { // Note off
            this.isPlaying = false;
            console.log('Note off');
        }
    }
    
    handleConfig(config) {
        console.log('Config updated:', config);
    }
    
    process(inputs, outputs, parameters) {
        const output = outputs[0];
        
        if (output && output.length > 0) {
            const outputChannel = output[0];
            
            for (let i = 0; i < outputChannel.length; i++) {
                let sample = 0;
                
                if (this.isPlaying) {
                    // Simple sine wave synthesis
                    sample = Math.sin(this.phase) * this.amplitude;
                    this.phase += 2 * Math.PI * this.frequency / sampleRate;
                    
                    // Keep phase in bounds
                    if (this.phase > 2 * Math.PI) {
                        this.phase -= 2 * Math.PI;
                    }
                }
                
                outputChannel[i] = sample;
            }
        }
        
        return true;
    }
}

console.log('Registering audio-processor with AudioWorkletGlobalScope');
registerProcessor('audio-processor', DissonanceAudioProcessor);
console.log('audio-processor registered successfully');
