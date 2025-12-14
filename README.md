## 🦀 Carapace: A Minimal Rust STUN Server

**Carapace** (the protective shell of a crab or turtle) is a minimalist project focused on implementing the STUN (Session Traversal Utilities for NAT) protocol in Rust to **penetrate the hard outer layer of NATs.**

### 📌 Project Overview

This project is primarily a **learning exercise** designed to help developers understand how a STUN server works. **Carapace** enables clients hidden behind a NAT to successfully discover their **publicly visible IP address and port**.

We focus exclusively on the core implementation required for **Basic NAT Traversal**, deliberately omitting complex features like long-term authentication or advanced NAT type testing.

This serves as an excellent, hands-on environment to master fundamental concepts in networking, including UDP socket programming, NAT operation, and low-level protocol parsing.

### 🛠️ Getting Started

#### Prerequisites

* Rust Toolchain (latest stable version)
* Cargo (Rust's package manager)

#### Running the Server

Start the server from the project directory. It defaults to listening on `0.0.0.0:3478` (the standard STUN port).

```bash
cargo run
```
