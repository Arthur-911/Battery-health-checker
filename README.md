# 🔋 BatteryHealth (Pure Rust)

A lightweight, zero-dependency Windows battery diagnostic tool written in pure Rust. It communicates directly with ACPI battery drivers via the Win32 Device Driver API (`SetupDi` and `IOCTL_BATTERY_*`).

## ⚡ Key Highlights
* **Direct Hardware FFI:** No subprocesses, no HTML parsing, no WMI overhead.
* **Instant Startup:** Executes in milliseconds.
* **Tiny Binary Footprint:** Compiled release binary is only **~170 KB**.
* **Zero Background Drain:** Uses practically 0% CPU and < 5MB RAM.

---

## 🚀 Running the Tool

### Option 1: Run with Cargo
```powershell
cargo run --release
```

### Option 2: Run the Standalone Binary
```powershell
.\target\release\batteryhealth.exe
```

---

## 📊 Metrics Extracted
* **Live Power Source & Percentage:** Live AC / battery state and percentage gauge.
* **Factory Design Capacity vs Full Charge Capacity:** Exact mWh degradation measurement.
* **Battery Health & Wear Percentage:** Visual health status bar and percentage rating.
* **Health Assessment Grade:** Plain-English rating (`EXCELLENT`, `GOOD`, `FAIR`, `DEGRADED`, `CRITICAL`).
* **Electrical Diagnostics:** Terminal voltage (V) and charge/discharge flow rate (mW).
* **Hardware Profile:** Manufacturer, Model/Device Name, Serial Number, and Chemistry.
