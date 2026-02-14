# NeoNet

NeoNet is a Rust-based tool that starts a web server powered by Actix Web.

By default, the server listens on:

```
0.0.0.0:8080
```

---

## 🚀 Features

- Starts an HTTP server
- Built with Actix Web
- Designed for modular API development

---

## 📦 Clone the repository

```bash
git clone https://github.com/NeoNet-app/NeoNet.git
cd NeoNet
```

---

## 🔧 Build the project

Make sure you have Rust installed:

```bash
rustup update
```

Then build the project:

```bash
cargo build
```

Or build in release mode:

```bash
cargo build --release
```

---

## ▶️ Run the server

```bash
cargo run
```

The server will start on:

```
http://localhost:8080
```

---

## 🛠 Custom Build

You can customize the build using features or environment variables if implemented.

Example:

```bash
RUST_LOG=info cargo run
```

