# Remedy Serial Communication for Web

**Date**: 2026-01-30  
**Feature**: Web-based Serial Interface for Remedy Hardware

## Overview

This feature enables direct serial communication with Remedy hardware (X-Ray generator control) from the web browser interface using the Web Serial API and WebAssembly.

## Architecture

1.  **Pure Rust Protocol Logic (`src/acquisition/remedy.rs`)**:
    *   `RemedyProtocol`: Handles parsing, checksum calculation, and state management.
    *   `RemedyEvent`: Enums for logs, data updates, and errors.
    *   `SystemState`: Struct representing the current hardware state (KV, mA, mS, Status, etc.).

2.  **WASM Bindings (`RemedyWasm`)**:
    *   Exposes `process_input` to receive raw bytes from JS.
    *   Exposes `build_command` to construct packets for JS to send.
    *   Uses `serde-wasm-bindgen` to serialize events and state for JS consumption.

3.  **Web Interface (`static/remedy.html`)**:
    *   Uses **Web Serial API** to connect to the physical serial port.
    *   Reads streams of bytes and passes them to the WASM module.
    *   Updates the DOM based on events returned by WASM.
    *   Sends commands (PR/XR/KV/etc.) constructed by WASM back to the serial port.

## Usage

1.  **Build WASM**:
    ```bash
    wasm-pack build --target web
    ```

2.  **Serve**:
    Serve the project root using a web server (e.g., Python `http.server` or `live-server`).
    ```bash
    # From project root
    python -m http.server 8000
    ```

3.  **Run**:
    *   Open `http://localhost:8000/static/remedy.html` in a browser (Chrome/Edge recommended for Web Serial support).
    *   Click **Connect Serial Port**, select the COM port for the Remedy device.
    *   Use the interface to Monitor status and Send commands.

## Key Implementation Details

*   **Borrow Checker Resolution**: The Rust WASM interface creates owned copies of data buffers to avoid mutable/immutable borrow conflicts when handling protocol replies.
*   **Serialization**: `serde` is used to convert Rust enums and structs to JavaScript objects, enabling rich structured data transfer between WASM and JS.
*   **No Polling**: The system is event-driven; JS reads from the serial port stream and pushes data to Rust immediately.

## Verified Features

*   Connection/Disconnection handling.
*   Packet parsing (ST, PR, XR, EL, ER, EW commands).
*   Checksum verification.
*   Automatic Reply (ACK/NAK/RE) generation.
*   State parsing and UI updates.
