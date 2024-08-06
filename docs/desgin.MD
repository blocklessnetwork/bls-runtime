### Features of blockless runtime.

The runtime is depeneded on wasm implements. so it have follow features of wasm.

- Fast. built on the optimizing code generator to quickly generate high-quality
  machine code either at runtime or ahead-of-time. Wasmtime's runtime is also
  optimized for cases such as efficient instantiation, low-overhead transitions
  between the embedder and wasm, and scalability of concurrent instances.
