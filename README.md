# 🛡️ Vajra-Sentinel
> **A High-Performance, Multi-Threaded EV Diagnostic System built in Rust.**

![License](https://img.shields.io/badge/license-MIT-blue.svg) ![Language](https://img.shields.io/badge/language-Rust%20🦀-orange.svg) ![Platform](https://img.shields.io/badge/platform-Linux%20|%20macOS%20|%20Windows-lightgrey.svg)

## 📖 Overview
**Vajra-Sentinel** is a hardware-in-the-loop (HIL) simulation engine designed to mimic the nervous system of a modern Electric Vehicle (EV). 

Unlike standard loggers, this system implements **Edge AI** capabilities directly within the driver layer. It simulates independent ECUs (Electronic Control Units) communicating over a virtual CAN Bus, detecting critical safety anomalies like **Thermal Runaway** and **Sensor Blindness** in real-time without relying on external cloud processing.

This project demonstrates the power of **Rust** in safety-critical automotive applications, leveraging "Fearless Concurrency" to manage shared state without garbage collection latency.

## ⚡ Key Features
* **🚀 Multi-Threaded Architecture:** Simulates 4+ independent hardware sensors running on separate system threads.
* **🧠 Edge AI Anomaly Detection:** Implements **Z-Score Statistical Analysis** (Standard Deviation) to detect outliers in voltage and sensor confidence scores.
* **🛡️ Thread Safety:** Utilizes `Arc<Mutex<T>>` patterns to ensure safe data sharing between worker threads and the UI thread.
* **💾 Persistent Blackbox:** Logs all critical Diagnostic Trouble Codes (DTCs) to an embedded **SQLite** database for post-incident analysis.
* **🖥️ Real-Time Dashboard:** Features a high-performance TUI (Terminal User Interface) built with `ratatui` for live system monitoring (60 FPS).

## 🏗️ System Architecture
The system follows a producer-consumer model where independent ECU threads generate data, process it via internal driver logic, and push updates to a thread-safe shared state.

*(See `architecture.mermaid` for visual diagram)*

| Component | Role | Tech Stack |
| :--- | :--- | :--- |
| **BMS ECU** | Monitors battery cell voltage & thermal runaway | `rand`, `std::thread` |
| **ADAS Computer** | Monitors radar/camera confidence levels | `rand` |
| **Shared State** | Thread-safe memory buffer for UI data | `std::sync::Mutex`, `std::sync::Arc` |
| **Blackbox** | Persistent storage for fault logs | `rusqlite` (SQLite) |
| **Dashboard** | Visualizes live data and logs | `ratatui`, `crossterm` |

## 🛠️ Installation & Setup

### Prerequisites
* **Rust Toolchain:** Ensure you have Rust installed (`cargo --version`).
* **Terminal:** A terminal that supports ANSI color codes (VS Code Terminal, iTerm2, Windows Terminal).

### 1. Clone the Repository
```bash
git clone https://github.com/SIDR1921/rusty-adas.git
cd rusty-adas
```
