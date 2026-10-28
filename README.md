# Battery Health Checker

A lightweight diagnostic tool for Windows laptops, built in pure Rust.

Operating systems track detailed battery health and degradation data, but Windows typically hides these metrics behind command-line scripts or dense HTML reports that are difficult to interpret. Battery Health Checker interfaces directly with low-level ACPI drivers to deliver a clear, human-readable assessment of your laptop's battery condition, capacity loss, and electrical performance.

---

## Overview

Whether you are troubleshooting reduced runtime on your everyday machine or evaluating a second-hand laptop before purchase, this utility gives you instant visibility into your battery's actual physical health.

### Key Features

* **Direct Hardware Queries:** Communicates directly with the Windows ACPI battery driver (`cmbatt.sys`) via Win32 device control APIs (`DeviceIoControl`). It does not spawn background processes or scrape temporary files.
* **Portable and Self-Contained:** Compiles to a single, dependency-free binary under 200 KB. You can place it on a USB flash drive and run it on any Windows laptop without installation or administrator privileges.
* **Plain-Language Diagnostics:** Translates raw milliwatt-hour (mWh) measurements into intuitive health ratings (`EXCELLENT`, `GOOD`, `FAIR`, `DEGRADED`, `CRITICAL`) and wear percentages.
* **Real-Time Electrical Diagnostics:** Displays current charge status, terminal voltage, and live charge/discharge flow rates.
* **Multi-Battery Detection:** Automatically identifies and evaluates secondary or external batteries on supported hardware (such as Lenovo ThinkPad models).

---

## Example Output

```text
==================================================================
                 LAPTOP BATTERY HEALTH DIAGNOSTIC                 
                     Native Rust Win32 Driver                     
==================================================================

[ Real-Time Power Status ]
  Power Source      : AC Adapter (Plugged In)
  Current Charge    : 100%  [■■■■■■■■■■■■■■■■■■■■]

[ Battery #1 Hardware Profile ]
  Device Model      : PABAS0241231
  Manufacturer      : Hewlett-Packard 
  Serial Number     : 41167
  Chemistry Type    : LION
------------------------------------------------------------------
  [ Capacity & Longevity ]
  Original Design   :  52500 mWh
  Full Charge Cap.  :  36240 mWh
  Battery Health    :  69.0%  [■■■■■■■■■■■■■■░░░░░░] (Wear: 31.0%)
  Charge Cycles     : Not reported by ACPI firmware
------------------------------------------------------------------
  [ Health Rating ]
  Status Grade      : [FAIR]
  Assessment        : Noticeable degradation. Runtime is reduced by ~25-35%.
------------------------------------------------------------------
  [ Electrical Diagnostics ]
  Current Remaining :  36240 mWh
  Terminal Voltage  : 12.655 V
  Power Flow Rate   : Idle / Fully Charged
------------------------------------------------------------------

Diagnostic completed successfully.
```

---

## Quick Start

### Download Pre-built Binary (No Setup Needed)

You can download the standalone executable directly from the [Releases page](https://github.com/Arthur-911/Battery-health-checker/releases/latest).

Place `batteryhealth.exe` anywhere (or copy it to a USB flash drive) and execute it directly from PowerShell or Command Prompt. No runtime installation is required.

### Prerequisites (For Building from Source)

* Windows 10 or Windows 11
* Rust toolchain (1.70 or newer) if building from source

### Installing Globally (Run from Any Terminal Directory)

To run the utility anywhere on your system simply by typing `batteryhealth` or `battery`:

```powershell
cargo install --path .
```

Once installed, open any new Command Prompt or PowerShell window and type:

```powershell
batteryhealth
```

### Running with Cargo

Clone the repository and run the project directly:

```powershell
git clone https://github.com/Arthur-911/Battery-health-checker.git
cd Battery-health-checker
cargo run --release
```

### Building the Standalone Executable

To compile a production-ready, self-contained binary:

```powershell
cargo build --release
```

The compiled executable will be located at:

```powershell
.\target\release\batteryhealth.exe
```

You can copy this file to any USB drive or folder and run it independently on any modern Windows laptop.

---

## Metrics Reference

| Metric | Description |
| :--- | :--- |
| **Designed Capacity** | The factory-rated energy storage of the battery when new, measured in milliwatt-hours (mWh). |
| **Full Charge Capacity** | The maximum energy the battery can store in its current state. As chemical cells age, this value decreases. |
| **Battery Health** | The current capacity expressed as a percentage of the original factory specification. |
| **Wear Level** | The percentage of capacity permanently lost to chemical degradation. |
| **Terminal Voltage** | Real-time voltage output across the battery terminals. |
| **Power Flow Rate** | Active electrical intake or draw (positive during charging, negative during discharge). |

---

## System Compatibility

* **Operating System:** Windows 10 and Windows 11 (x64 and x86 architectures).
* **Hardware:** Compatible with all ACPI-compliant laptops, including models from Dell, HP, Lenovo, ASUS, Acer, MSI, Razer, Microsoft Surface, and Samsung.
* **Desktop / VM Behavior:** When run on a desktop workstation or virtual environment without an ACPI battery interface, the tool safely exits with an informative notification rather than failing.

---

## Technical Details

The tool relies on the official Microsoft `windows-sys` crate to interface directly with the kernel-level battery device class:

1. Enumerates devices registered under `GUID_DEVINTERFACE_BATTERY` (`{72631e54-78a4-11d0-bcf7-00aa00b7b32a}`).
2. Opens a device handle using `CreateFileW`.
3. Issues `IOCTL_BATTERY_QUERY_TAG` to obtain the active battery tag.
4. Requests `BATTERY_INFORMATION` and `BATTERY_STATUS` structures via `DeviceIoControl`.
5. Safely processes wide UTF-16 device strings and computes degradation metrics before releasing system resources.

---

## License

This project is licensed under the MIT License. Feel free to use, modify, and distribute it.
