# Enki 3D SDF Raymarching Showcase

A real-time, procedural 3D raymarching sdf with lighting, ambient occlusion, and mouse driven orbital camera, written entirely in standard Rust and compiled just-in-time onto GPU silicon using [Enki](https://github.com/enkiruntime/enki).

---

## Quick Start

```bash
git clone https://github.com/enkiruntime/enki_sdf.git
cd enki_sdf
cargo run
```

---

## Controls

- **Mouse Drag:** Rotate camera orbit around the liquid chrome cluster.
- **`SPACE`:** Dynamically toggle execution between **GPU silicon (Enki)** and **multi-threaded CPU cores (Rayon)** in real time. Observe the instantaneous performance differential on the window title bar!
- **`ESC`:** Terminate cleanly.
- No shaders, just pure rust.
---

## Requirements

- Stable Rust `1.80+`.
- A GPU driver supporting Vulkan 1.3 (64-bit Buffer Device Addresses).
- Linux or Windows.

---

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
{}
# enki_sdf
