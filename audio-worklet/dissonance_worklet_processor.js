// AudioWorklet processor for Dissonance Lab
// This JavaScript file properly registers the WASM-based AudioWorkletProcessor
// Tricker to do the AudioWorkletProcessor inheritance in rust

// TODO: do we need to do this in javascript? could we do it in rust instead?

// Add TextDecoder shim for AudioWorklet context
// AudioWorklets run in a restricted environment that doesn't have access to TextDecoder,
// but the console_log crate (and wasm-bindgen string conversion) requires it for logging.
// This provides a minimal implementation that handles basic UTF-8 decoding for log messages.
if (typeof TextDecoder === 'undefined') {
    globalThis.TextDecoder = class {
        constructor(encoding = 'utf-8') {
            this.encoding = encoding;
        }
        
        decode(bytes) {
            // Handle undefined/null input
            if (!bytes) {
                return '';
            }
            
            // Convert to Uint8Array if needed
            if (!(bytes instanceof Uint8Array)) {
                if (bytes.buffer) {
                    bytes = new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength);
                } else {
                    return '';
                }
            }
            
            // Simple UTF-8 decoder for basic ASCII strings
            // This handles the common case for log messages
            let result = '';
            for (let i = 0; i < bytes.length; i++) {
                const byte = bytes[i];
                if (byte < 128) {
                    result += String.fromCharCode(byte);
                } else {
                    // For non-ASCII, just use replacement character
                    result += '�';
                }
            }
            return result;
        }
    };
}

class DissonanceWorkletProcessor extends AudioWorkletProcessor {
    constructor(options) {
        super();
        this.wasmProcessor = null;
        this.initialized = false;
        
        // Extract WASM data from constructor options
        const { wasmBytes, jsGlueCode } = options.processorOptions || {};
        
        if (wasmBytes && jsGlueCode) {
            this.initializeWasm(wasmBytes, jsGlueCode)
                .then(() => {
                    this.initialized = true;
                    this.port.postMessage({ type: 'init-complete' });
                })
                .catch(err => {
                    console.error('[DissonanceWorkletProcessor] Failed to initialize WASM processor:', err);
                    this.port.postMessage({ type: 'init-error', error: err.message });
                });
        } else {
            console.error('[DissonanceWorkletProcessor] Missing wasmBytes or jsGlueCode in constructor options');
        }
        
        // Handle messages from the main thread
        this.port.onmessage = (event) => {
            if (this.initialized && this.wasmProcessor) {
                this.wasmProcessor.handle_message(event.data);
            } else {
                console.warn('[DissonanceWorkletProcessor] Received message before initialization:', event.data);
            }
        };
    }

    process(inputs, outputs, parameters) {
        if (this.initialized && this.wasmProcessor) {
            return this.wasmProcessor.process(inputs, outputs, parameters);
        }
        
        // Fill with silence while initializing
        for (let output of outputs) {
            for (let channel of output) {
                channel.fill(0);
            }
        }
        
        return true; // Keep processor alive
    }

    async initializeWasm(wasmBytes, jsGlueCode) {
        // The no-modules target creates an IIFE that assigns to a local wasm_bindgen variable
        // We need to wrap the code to capture this variable
        const wrappedCode = `
            (function() {
                ${jsGlueCode}
                return wasm_bindgen;
            })()
        `;
        
        let wasmBindgen;
        try {
            wasmBindgen = eval(wrappedCode);
        } catch (error) {
            console.error('[DissonanceWorkletProcessor] Failed to capture wasm_bindgen:', error);
            throw new Error(`Failed to evaluate WASM glue code: ${error.message}`);
        }
        
        if (!wasmBindgen || typeof wasmBindgen !== 'function') {
            console.error('[DissonanceWorkletProcessor] wasm_bindgen is not a function:', typeof wasmBindgen);
            throw new Error('Failed to capture valid wasm_bindgen function');
        }
        
        // Initialize the WASM module with the provided bytes
        await wasmBindgen(wasmBytes);
        
        this.wasmProcessor = new wasmBindgen.DissonanceProcessor();
        this.wasmProcessor.set_port(this.port);
    }
}

registerProcessor('dissonance-processor', DissonanceWorkletProcessor);
