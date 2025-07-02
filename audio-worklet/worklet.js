// AudioWorklet processor for Dissonance Lab
// This JavaScript file properly registers the WASM-based AudioWorkletProcessor

class DissonanceWorkletProcessor extends AudioWorkletProcessor {
    constructor(options) {
        super();
        this.wasmProcessor = null;
        this.initialized = false;
        
        // Extract WASM data from constructor options
        const { wasmBytes, jsGlueCode } = options.processorOptions || {};
        
        if (wasmBytes && jsGlueCode) {
            // Initialize WASM immediately in constructor
            this.initializeWasm(wasmBytes, jsGlueCode)
                .then(() => {
                    this.initialized = true;
                    console.log('[DissonanceWorkletProcessor] WASM processor initialized');
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
                // Forward messages to WASM processor
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
        console.log('[DissonanceWorkletProcessor] Starting WASM initialization');
        console.log('[DissonanceWorkletProcessor] jsGlueCode length:', jsGlueCode.length);
        
        // Set up the environment for the wasm-bindgen code
        // In AudioWorklet context, we need to provide 'self' as globalThis
        if (typeof self === 'undefined') {
            globalThis.self = globalThis;
        }
        
        // Add polyfills for TextDecoder and TextEncoder if not available
        if (typeof globalThis.TextDecoder === 'undefined') {
            console.log('[DissonanceWorkletProcessor] Adding TextDecoder polyfill');
            globalThis.TextDecoder = class TextDecoder {
                constructor(encoding = 'utf-8') {
                    this.encoding = encoding;
                }
                
                decode(input) {
                    if (!input) return '';
                    
                    // Simple UTF-8 decoder for basic text
                    let result = '';
                    const bytes = new Uint8Array(input);
                    
                    for (let i = 0; i < bytes.length; i++) {
                        const byte = bytes[i];
                        if (byte < 0x80) {
                            // ASCII
                            result += String.fromCharCode(byte);
                        } else if (byte < 0xC0) {
                            // Invalid continuation byte, skip
                            continue;
                        } else if (byte < 0xE0) {
                            // 2-byte sequence
                            if (i + 1 < bytes.length) {
                                const byte2 = bytes[++i];
                                result += String.fromCharCode(((byte & 0x1F) << 6) | (byte2 & 0x3F));
                            }
                        } else if (byte < 0xF0) {
                            // 3-byte sequence
                            if (i + 2 < bytes.length) {
                                const byte2 = bytes[++i];
                                const byte3 = bytes[++i];
                                result += String.fromCharCode(((byte & 0x0F) << 12) | ((byte2 & 0x3F) << 6) | (byte3 & 0x3F));
                            }
                        }
                        // Skip 4-byte sequences for simplicity
                    }
                    
                    return result;
                }
            };
        }
        
        if (typeof globalThis.TextEncoder === 'undefined') {
            console.log('[DissonanceWorkletProcessor] Adding TextEncoder polyfill');
            globalThis.TextEncoder = class TextEncoder {
                encode(input) {
                    if (!input) return new Uint8Array(0);
                    
                    // Simple UTF-8 encoder
                    const result = [];
                    for (let i = 0; i < input.length; i++) {
                        const code = input.charCodeAt(i);
                        if (code < 0x80) {
                            result.push(code);
                        } else if (code < 0x800) {
                            result.push(0xC0 | (code >> 6));
                            result.push(0x80 | (code & 0x3F));
                        } else {
                            result.push(0xE0 | (code >> 12));
                            result.push(0x80 | ((code >> 6) & 0x3F));
                            result.push(0x80 | (code & 0x3F));
                        }
                    }
                    return new Uint8Array(result);
                }
            };
        }
        
        // Provide TextDecoder and TextEncoder polyfills for AudioWorklet context
        if (typeof TextDecoder === 'undefined') {
            globalThis.TextDecoder = class TextDecoder {
                constructor(encoding = 'utf-8') {
                    this.encoding = encoding;
                }
                
                decode(input) {
                    if (!input) return '';
                    // Simple UTF-8 decoder for basic ASCII/UTF-8 text
                    const bytes = new Uint8Array(input);
                    let result = '';
                    let i = 0;
                    while (i < bytes.length) {
                        let byte = bytes[i];
                        if (byte < 0x80) {
                            result += String.fromCharCode(byte);
                            i++;
                        } else if (byte < 0xE0) {
                            result += String.fromCharCode(((byte & 0x1F) << 6) | (bytes[i + 1] & 0x3F));
                            i += 2;
                        } else if (byte < 0xF0) {
                            result += String.fromCharCode(((byte & 0x0F) << 12) | ((bytes[i + 1] & 0x3F) << 6) | (bytes[i + 2] & 0x3F));
                            i += 3;
                        } else {
                            // Skip 4-byte sequences for simplicity
                            i += 4;
                        }
                    }
                    return result;
                }
            };
        }
        
        if (typeof TextEncoder === 'undefined') {
            globalThis.TextEncoder = class TextEncoder {
                encode(input) {
                    // Simple UTF-8 encoder
                    const result = [];
                    for (let i = 0; i < input.length; i++) {
                        const code = input.charCodeAt(i);
                        if (code < 0x80) {
                            result.push(code);
                        } else if (code < 0x800) {
                            result.push(0xC0 | (code >> 6));
                            result.push(0x80 | (code & 0x3F));
                        } else {
                            result.push(0xE0 | (code >> 12));
                            result.push(0x80 | ((code >> 6) & 0x3F));
                            result.push(0x80 | (code & 0x3F));
                        }
                    }
                    return new Uint8Array(result);
                }
            };
        }
        
        console.log('[DissonanceWorkletProcessor] Evaluating JS glue code...');
        
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
            console.log('[DissonanceWorkletProcessor] Successfully captured wasm_bindgen');
        } catch (error) {
            console.error('[DissonanceWorkletProcessor] Failed to capture wasm_bindgen:', error);
            throw new Error(`Failed to evaluate WASM glue code: ${error.message}`);
        }
        
        if (!wasmBindgen || typeof wasmBindgen !== 'function') {
            console.error('[DissonanceWorkletProcessor] wasm_bindgen is not a function:', typeof wasmBindgen);
            throw new Error('Failed to capture valid wasm_bindgen function');
        }
        
        // Initialize the WASM module with the provided bytes
        console.log('[DissonanceWorkletProcessor] Initializing WASM module...');
        await wasmBindgen(wasmBytes);
        
        // Create the processor instance
        console.log('[DissonanceWorkletProcessor] Creating DissonanceProcessor...');
        this.wasmProcessor = new wasmBindgen.DissonanceProcessor();
        this.wasmProcessor.set_port(this.port);
        console.log('[DissonanceWorkletProcessor] WASM initialization complete');
    }
}

registerProcessor('dissonance-processor', DissonanceWorkletProcessor);
