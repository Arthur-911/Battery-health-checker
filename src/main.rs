use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::null_mut;
use windows_sys::core::GUID;
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
    SetupDiGetDeviceInterfaceDetailW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT,
    SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Power::{
    GetSystemPowerStatus, BatteryDeviceName, BatteryInformation, BatteryManufactureName,
    BatterySerialNumber, BatteryTemperature, BATTERY_INFORMATION, BATTERY_QUERY_INFORMATION,
    BATTERY_STATUS, BATTERY_WAIT_STATUS, IOCTL_BATTERY_QUERY_INFORMATION, IOCTL_BATTERY_QUERY_STATUS,
    IOCTL_BATTERY_QUERY_TAG, SYSTEM_POWER_STATUS,
};

// Official Windows Battery Device Interface GUID: {72631e54-78a4-11d0-bcf7-00aa00b7b32a}
const GUID_DEVINTERFACE_BATTERY: GUID =
    GUID::from_u128(0x72631e54_78a4_11d0_bcf7_00aa00b7b32a);

const BATTERY_UNKNOWN_RATE: i32 = -2147483648; // 0x80000000 in i32

#[derive(Debug, Default)]
struct DetailedBatteryInfo {
    device_name: String,
    manufacturer: String,
    serial_number: String,
    chemistry: String,
    designed_capacity_mwh: u32,
    full_charge_capacity_mwh: u32,
    cycle_count: Option<u32>,
    current_capacity_mwh: u32,
    voltage_mv: u32,
    rate_mw: i32,
    temperature_celsius: Option<f32>,
    is_charging: bool,
    is_discharging: bool,
    health_percentage: f32,
    wear_percentage: f32,
}

fn query_string_info(
    handle: windows_sys::Win32::Foundation::HANDLE,
    tag: u32,
    level: i32,
) -> Option<String> {
    let mut query = BATTERY_QUERY_INFORMATION {
        BatteryTag: tag,
        InformationLevel: level,
        AtRate: 0,
    };
    let mut buffer = [0u8; 512];
    let mut bytes_returned = 0u32;

    let success = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_INFORMATION,
            &mut query as *mut _ as *mut c_void,
            size_of::<BATTERY_QUERY_INFORMATION>() as u32,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
            &mut bytes_returned,
            null_mut(),
        )
    };

    if success != 0 && bytes_returned > 0 {
        let u16_slice: &[u16] = unsafe {
            std::slice::from_raw_parts(
                buffer.as_ptr() as *const u16,
                (bytes_returned as usize) / 2,
            )
        };
        let trimmed: Vec<u16> = u16_slice.iter().copied().take_while(|&c| c != 0).collect();
        String::from_utf16(&trimmed).ok()
    } else {
        None
    }
}

fn query_battery_at_path(path: &[u16]) -> Option<DetailedBatteryInfo> {
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            null_mut(),
        )
    };

    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        return None;
    }

    let mut info = DetailedBatteryInfo::default();

    // 1. Get Battery Tag
    let mut battery_tag = 0u32;
    let mut bytes_returned = 0u32;
    let mut wait_tag = 0u32;

    let res = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_TAG,
            &mut wait_tag as *mut _ as *mut c_void,
            size_of::<u32>() as u32,
            &mut battery_tag as *mut _ as *mut c_void,
            size_of::<u32>() as u32,
            &mut bytes_returned,
            null_mut(),
        )
    };

    if res == 0 || battery_tag == 0 {
        unsafe { CloseHandle(handle) };
        return None;
    }

    // 2. Query BATTERY_INFORMATION (Design capacity, Full charged capacity, Cycle count)
    let mut query = BATTERY_QUERY_INFORMATION {
        BatteryTag: battery_tag,
        InformationLevel: BatteryInformation,
        AtRate: 0,
    };
    let mut bat_info: BATTERY_INFORMATION = unsafe { std::mem::zeroed() };

    let res = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_INFORMATION,
            &mut query as *mut _ as *mut c_void,
            size_of::<BATTERY_QUERY_INFORMATION>() as u32,
            &mut bat_info as *mut _ as *mut c_void,
            size_of::<BATTERY_INFORMATION>() as u32,
            &mut bytes_returned,
            null_mut(),
        )
    };

    if res != 0 {
        info.designed_capacity_mwh = bat_info.DesignedCapacity;
        info.full_charge_capacity_mwh = bat_info.FullChargedCapacity;
        if bat_info.CycleCount > 0 && bat_info.CycleCount != u32::MAX {
            info.cycle_count = Some(bat_info.CycleCount);
        }
        let chem_bytes = bat_info.Chemistry;
        info.chemistry = String::from_utf8_lossy(&chem_bytes)
            .trim_matches(char::from(0))
            .to_string();

        if info.designed_capacity_mwh > 0 {
            info.health_percentage = (info.full_charge_capacity_mwh as f32
                / info.designed_capacity_mwh as f32)
                * 100.0;
            info.wear_percentage = 100.0 - info.health_percentage;
            if info.wear_percentage < 0.0 {
                info.wear_percentage = 0.0;
            }
        }
    }

    // 3. Query Strings: Device name, manufacturer, serial number
    info.device_name = query_string_info(handle, battery_tag, BatteryDeviceName)
        .unwrap_or_else(|| "Internal Battery".to_string());
    info.manufacturer = query_string_info(handle, battery_tag, BatteryManufactureName)
        .unwrap_or_else(|| "Unknown".to_string());
    info.serial_number = query_string_info(handle, battery_tag, BatterySerialNumber)
        .unwrap_or_else(|| "Unknown".to_string());

    // 4. Query Temperature
    let mut temp_query = BATTERY_QUERY_INFORMATION {
        BatteryTag: battery_tag,
        InformationLevel: BatteryTemperature,
        AtRate: 0,
    };
    let mut temp_val = 0u32;
    let res = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_INFORMATION,
            &mut temp_query as *mut _ as *mut c_void,
            size_of::<BATTERY_QUERY_INFORMATION>() as u32,
            &mut temp_val as *mut _ as *mut c_void,
            size_of::<u32>() as u32,
            &mut bytes_returned,
            null_mut(),
        )
    };
    if res != 0 && temp_val > 0 {
        let temp_c = (temp_val as f32 / 10.0) - 273.15;
        if temp_c > -50.0 && temp_c < 120.0 {
            info.temperature_celsius = Some(temp_c);
        }
    }

    // 5. Query Live Status (Current Capacity, Voltage, Rate)
    let mut wait_status = BATTERY_WAIT_STATUS {
        BatteryTag: battery_tag,
        Timeout: 0,
        PowerState: 0,
        LowCapacity: 0,
        HighCapacity: 0,
    };
    let mut status: BATTERY_STATUS = unsafe { std::mem::zeroed() };

    let res = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_STATUS,
            &mut wait_status as *mut _ as *mut c_void,
            size_of::<BATTERY_WAIT_STATUS>() as u32,
            &mut status as *mut _ as *mut c_void,
            size_of::<BATTERY_STATUS>() as u32,
            &mut bytes_returned,
            null_mut(),
        )
    };

    if res != 0 {
        info.current_capacity_mwh = status.Capacity;
        info.voltage_mv = status.Voltage;
        info.rate_mw = status.Rate;
        info.is_discharging = (status.PowerState & 0x02) != 0;
        info.is_charging = (status.PowerState & 0x04) != 0;
    }

    unsafe { CloseHandle(handle) };
    Some(info)
}

fn scan_all_batteries() -> Vec<DetailedBatteryInfo> {
    let mut results = Vec::new();

    let dev_info = unsafe {
        SetupDiGetClassDevsW(
            &GUID_DEVINTERFACE_BATTERY,
            null_mut(),
            null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    };

    if dev_info == -1 || dev_info == 0 {
        return results;
    }

    let mut index = 0u32;
    loop {
        let mut iface_data: SP_DEVICE_INTERFACE_DATA = unsafe { std::mem::zeroed() };
        iface_data.cbSize = size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;

        let res = unsafe {
            SetupDiEnumDeviceInterfaces(
                dev_info,
                null_mut(),
                &GUID_DEVINTERFACE_BATTERY,
                index,
                &mut iface_data,
            )
        };

        if res == 0 {
            break;
        }

        let mut required_size = 0u32;
        unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                dev_info,
                &iface_data,
                null_mut(),
                0,
                &mut required_size,
                null_mut(),
            )
        };

        if required_size > 0 {
            let mut detail_buffer: Vec<u8> = vec![0; required_size as usize];
            let detail_ptr =
                detail_buffer.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;

            let cb_size = if size_of::<usize>() == 8 { 8 } else { 6 };
            unsafe {
                (*detail_ptr).cbSize = cb_size;
            }

            let detail_res = unsafe {
                SetupDiGetDeviceInterfaceDetailW(
                    dev_info,
                    &iface_data,
                    detail_ptr,
                    required_size,
                    null_mut(),
                    null_mut(),
                )
            };

            if detail_res != 0 {
                let struct_offset = std::mem::offset_of!(SP_DEVICE_INTERFACE_DETAIL_DATA_W, DevicePath);
                let u16_slice: &[u16] = unsafe {
                    let ptr = (detail_buffer.as_ptr().add(struct_offset)) as *const u16;
                    std::slice::from_raw_parts(ptr, (required_size as usize - struct_offset) / 2)
                };
                let mut path_u16: Vec<u16> =
                    u16_slice.iter().copied().take_while(|&c| c != 0).collect();
                path_u16.push(0);

                if let Some(battery_info) = query_battery_at_path(&path_u16) {
                    results.push(battery_info);
                }
            }
        }

        index += 1;
    }

    unsafe {
        SetupDiDestroyDeviceInfoList(dev_info);
    }

    results
}

fn progress_bar(percentage: f32, width: usize) -> String {
    let clamped = percentage.clamp(0.0, 100.0);
    let filled = ((clamped / 100.0) * width as f32).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "■".repeat(filled), "░".repeat(empty))
}

fn print_health_badge(health_pct: f32) -> (&'static str, &'static str) {
    if health_pct >= 90.0 {
        ("EXCELLENT", "Battery is in near-factory condition with minimal degradation.")
    } else if health_pct >= 80.0 {
        ("GOOD", "Normal degradation. Reliable runtime remains.")
    } else if health_pct >= 65.0 {
        ("FAIR", "Noticeable degradation. Runtime is reduced by ~25-35%.")
    } else if health_pct >= 50.0 {
        ("DEGRADED", "Significant capacity loss. Replacement recommended soon.")
    } else {
        ("CRITICAL", "Severe wear level. Battery will drain rapidly or shut off.")
    }
}

fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║                 LAPTOP BATTERY HEALTH DIAGNOSTIC                 ║");
    println!("║                     Native Rust Win32 Driver                     ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    // 1. High-level Windows Power Status
    let mut power_status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    let has_power_status = unsafe { GetSystemPowerStatus(&mut power_status) } != 0;

    if has_power_status {
        let ac_status = match power_status.ACLineStatus {
            0 => "🔋 Battery Power (Unplugged)",
            1 => "⚡ AC Adapter (Plugged In)",
            _ => "Unknown",
        };
        let pct = power_status.BatteryLifePercent;
        println!("\n┌── [ Real-Time Power Status ]");
        println!("│  Power Source      : {}", ac_status);
        if pct != 255 {
            let bar = progress_bar(pct as f32, 20);
            println!("│  Current Charge    : {:>3}%  {}", pct, bar);
        }
        if power_status.BatteryLifeTime != u32::MAX {
            let hours = power_status.BatteryLifeTime / 3600;
            let minutes = (power_status.BatteryLifeTime % 3600) / 60;
            println!("│  Est. Runtime Left : {} hours, {} minutes", hours, minutes);
        }
        println!("└─────────────────────────────────────────────────────────────────");
    }

    // 2. Low-level Hardware Query
    let batteries = scan_all_batteries();

    if batteries.is_empty() {
        println!("\n⚠️  No ACPI-compliant battery found.");
        println!("   (If you are running on a Desktop PC or VM, no battery exists.)\n");
        return;
    }

    for (i, bat) in batteries.iter().enumerate() {
        let (rating, description) = print_health_badge(bat.health_percentage);

        println!("\n┌── [ Battery #{} Hardware Profile ]", i + 1);
        println!("│  Device Model      : {}", bat.device_name);
        println!("│  Manufacturer      : {}", bat.manufacturer);
        println!("│  Serial Number     : {}", bat.serial_number);
        println!("│  Chemistry Type    : {}", bat.chemistry);
        println!("├─────────────────────────────────────────────────────────────────");
        println!("│  [ Capacity & Longevity ]");
        println!("│  Original Design   : {:>6} mWh", bat.designed_capacity_mwh);
        println!("│  Full Charge Cap.  : {:>6} mWh", bat.full_charge_capacity_mwh);

        let health_bar = progress_bar(bat.health_percentage, 20);
        println!(
            "│  Battery Health    : {:>5.1}%  {} (Wear: {:.1}%)",
            bat.health_percentage, health_bar, bat.wear_percentage
        );

        match bat.cycle_count {
            Some(cycles) => println!("│  Charge Cycles     : {} cycles", cycles),
            None => println!("│  Charge Cycles     : Not reported by ACPI firmware"),
        }

        println!("├─────────────────────────────────────────────────────────────────");
        println!("│  [ Health Rating ]");
        println!("│  Status Grade      : [{}]", rating);
        println!("│  Assessment        : {}", description);
        println!("├─────────────────────────────────────────────────────────────────");
        println!("│  [ Electrical Diagnostics ]");
        println!("│  Current Remaining : {:>6} mWh", bat.current_capacity_mwh);
        println!("│  Terminal Voltage  : {:.3} V", bat.voltage_mv as f32 / 1000.0);

        if bat.rate_mw == BATTERY_UNKNOWN_RATE || bat.rate_mw == 0 {
            println!("│  Power Flow Rate   : Idle / Fully Charged");
        } else if bat.rate_mw > 0 {
            println!("│  Charge Rate       : +{} mW (Actively Charging)", bat.rate_mw);
        } else {
            println!("│  Discharge Rate    : {} mW (Discharging)", bat.rate_mw);
        }

        if let Some(temp) = bat.temperature_celsius {
            println!("│  Temperature       : {:.1} °C", temp);
        }
        println!("└─────────────────────────────────────────────────────────────────");
    }

    println!("\n✔ Diagnostics finished successfully.\n");
}
