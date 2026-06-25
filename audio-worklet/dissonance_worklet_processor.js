// AudioWorklet processor for Dissonance Lab
// This JavaScript file properly registers the WASM-based AudioWorkletProcessor
// Tricker to do the AudioWorkletProcessor inheritance in rust

// TODO: do we need to do this in javascript? could we do it in rust instead?

// Add TextDecoder shim for AudioWorklet context
// AudioWorklets run in a restricted environment that doesn't have access to TextDecoder,
// but the console_log crate (and wasm-bindgen string conversion) requires it for logging.
// This provides a minimal implementation that handles basic UTF-8 decoding for log messages.
if (typeof TextDecoder === "undefined") {
  globalThis.TextDecoder = class {
    constructor(encoding = "utf-8") {
      this.encoding = encoding;
    }

    decode(bytes) {
      // Handle undefined/null input
      if (!bytes) {
        return "";
      }

      // Convert to Uint8Array if needed
      if (!(bytes instanceof Uint8Array)) {
        if (bytes.buffer) {
          bytes = new Uint8Array(
            bytes.buffer,
            bytes.byteOffset,
            bytes.byteLength,
          );
        } else {
          return "";
        }
      }

      // Simple UTF-8 decoder for basic ASCII strings
      // This handles the common case for log messages
      let result = "";
      for (let i = 0; i < bytes.length; i++) {
        const byte = bytes[i];
        if (byte < 128) {
          result += String.fromCharCode(byte);
        } else {
          // For non-ASCII, just use replacement character
          result += "�";
        }
      }
      return result;
    }
  };
}

// Add TextEncoder shim for AudioWorklet context.
// Newer wasm-bindgen glue uses TextEncoder, which is not always available in AudioWorkletGlobalScope.
if (typeof TextEncoder === "undefined") {
  globalThis.TextEncoder = class {
    constructor() {
      this.encoding = "utf-8";
    }

    encode(input = "") {
      const bytes = [];
      const text = String(input);

      for (let i = 0; i < text.length; i++) {
        let codePoint = text.charCodeAt(i);

        if (codePoint >= 0xd800 && codePoint <= 0xdbff && i + 1 < text.length) {
          const next = text.charCodeAt(i + 1);
          if (next >= 0xdc00 && next <= 0xdfff) {
            codePoint =
              0x10000 + ((codePoint - 0xd800) << 10) + (next - 0xdc00);
            i += 1;
          }
        }

        if (codePoint < 0x80) {
          bytes.push(codePoint);
        } else if (codePoint < 0x800) {
          bytes.push(0xc0 | (codePoint >> 6));
          bytes.push(0x80 | (codePoint & 0x3f));
        } else if (codePoint < 0x10000) {
          bytes.push(0xe0 | (codePoint >> 12));
          bytes.push(0x80 | ((codePoint >> 6) & 0x3f));
          bytes.push(0x80 | (codePoint & 0x3f));
        } else {
          bytes.push(0xf0 | (codePoint >> 18));
          bytes.push(0x80 | ((codePoint >> 12) & 0x3f));
          bytes.push(0x80 | ((codePoint >> 6) & 0x3f));
          bytes.push(0x80 | (codePoint & 0x3f));
        }
      }

      return new Uint8Array(bytes);
    }

    encodeInto(input, destination) {
      const text = String(input);
      const bytes = this.encode(text);

      if (bytes.length <= destination.length) {
        destination.set(bytes);
        return { read: text.length, written: bytes.length };
      }

      let written = 0;
      let read = 0;
      for (let i = 0; i < text.length; i++) {
        const codePoint = text.codePointAt(i);
        const byteLength =
          codePoint < 0x80
            ? 1
            : codePoint < 0x800
              ? 2
              : codePoint < 0x10000
                ? 3
                : 4;

        if (written + byteLength > destination.length) {
          break;
        }

        written += byteLength;
        read += codePoint > 0xffff ? 2 : 1;
        if (codePoint > 0xffff) {
          i += 1;
        }
      }

      destination.set(bytes.subarray(0, written));
      return { read, written };
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
          this.port.postMessage({ type: "init-complete" });
        })
        .catch((err) => {
          console.error(
            "[DissonanceWorkletProcessor] Failed to initialize WASM processor:",
            err,
          );
          this.port.postMessage({ type: "init-error", error: err.message });
        });
    } else {
      console.error(
        "[DissonanceWorkletProcessor] Missing wasmBytes or jsGlueCode in constructor options",
      );
    }

    // Handle messages from the main thread
    this.port.onmessage = (event) => {
      if (this.initialized && this.wasmProcessor) {
        this.wasmProcessor.handle_message(event.data);
      } else {
        console.warn(
          "[DissonanceWorkletProcessor] Received message before initialization:",
          event.data,
        );
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
      console.error(
        "[DissonanceWorkletProcessor] Failed to capture wasm_bindgen:",
        error,
      );
      throw new Error(`Failed to evaluate WASM glue code: ${error.message}`);
    }

    if (!wasmBindgen || typeof wasmBindgen !== "function") {
      console.error(
        "[DissonanceWorkletProcessor] wasm_bindgen is not a function:",
        typeof wasmBindgen,
      );
      throw new Error("Failed to capture valid wasm_bindgen function");
    }

    // Initialize the WASM module with the provided bytes
    await wasmBindgen(wasmBytes);

    this.wasmProcessor = new wasmBindgen.DissonanceProcessor();
    this.wasmProcessor.set_port(this.port);
  }
}

registerProcessor("dissonance-processor", DissonanceWorkletProcessor);
