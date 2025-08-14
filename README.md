# Kepler2-WGPU

Kepler2-WGPU is a Rust-based project designed for processing and visualizing **DICOM** (Digital Imaging and Communications in Medicine) images using **WebGPU**. This project provides powerful tools for handling DICOM files, generating **CT volumes**, and rendering them both in **native** and **web environments**. It leverages modern technologies like **WebGPU** for high-performance rendering, and supports fast **asynchronous** I/O operations to optimize the workflow.

## Key Features (Plan)

- **Cross-Platform Support**: Runs in both **native** desktop and **web** environments, enabling flexibility for various platforms.
- **Fast I/O**: Utilizes asynchronous libraries to achieve high-speed input and output, reducing processing time.
- **Full DICOM Structure Support**: Handles the DICOM file structure, including metadata and pixel data for seamless integration.
- **RTPlan Support**: Includes features for processing and visualizing **Radiation Therapy Plans** (RTPlan), with support for **dose** visualization, **planning information**, and more.
- **Customizable**: Designed to be easily extended and customized, allowing users to modify the code to suit specific needs and workflows.
- **Optimized for Performance**: By leveraging the **WebGPU** API, the project ensures high-performance rendering and efficient memory handling for large datasets.
- **Flexible Rendering**: Visualizes CT volumes and other medical images using **WebGPU**'s advanced graphics capabilities.


## Building and Running

### Prerequisites

- Rust and Cargo
- Node.js and npm

### Building the Project

To build the project, run the following command:

```sh
cargo build --release
```

### Running the Project

To run the project, use the following command:

```sh
cargo run
```

### Running in WebAssembly

To build and run the project for WebAssembly, follow these steps:

1. Install the `wasm-pack` tool:

    ```sh
    cargo install wasm-pack
    ```

2. Build the project for WebAssembly:

    ```sh
    wasm-pack build --target web --no-opt
    ```

3. Serve the static files using a web server. For example, you can use `http-server`:

    ```sh
    npx live-server ./static
    ```

## Usage

1. Open the web interface by navigating to `http://localhost:8080` (or the appropriate port if using a different web server).
2. Upload DICOM files using the file input.
3. The application will parse the DICOM files and display the data in a tree structure.
4. You can interact with the tree to view studies, series, and images.

## License

This project is licensed under the MIT License. 

## Contributing

Contributions are welcome! Please open an issue or submit a pull request on GitHub.

## Acknowledgements

This project uses the following libraries and tools:

- [wgpu](https://github.com/gfx-rs/wgpu)
- [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen)
- [tokio](https://github.com/tokio-rs/tokio)
