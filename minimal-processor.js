// Minimal AudioWorklet processor for testing
class MinimalProcessor extends AudioWorkletProcessor {
    constructor() {
        super();
        console.log('MinimalProcessor created');
    }
    
    process(inputs, outputs, parameters) {
        // Just pass silence
        return true;
    }
}

registerProcessor('minimal-processor', MinimalProcessor);
