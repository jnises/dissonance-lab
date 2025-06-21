// Ultra minimal AudioWorklet processor for testing
console.log('Loading ultra-minimal-processor.js script');

class UltraMinimalProcessor extends AudioWorkletProcessor {
    constructor() {
        super();
        console.log('UltraMinimalProcessor created');
    }
    
    process(inputs, outputs, parameters) {
        return true;
    }
}

console.log('Registering ultra-minimal-processor');
registerProcessor('ultra-minimal-processor', UltraMinimalProcessor);
console.log('ultra-minimal-processor registered successfully');
