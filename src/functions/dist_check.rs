
use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::Storage::FileSystem::*,
    Win32::System::Ioctl::*,
    Win32::System::IO::DeviceIoControl,
};


use colored::*;
use std::mem;

// This function prints the results of checking the disk/SSD health on a Windows system using IOCTL to retrieve storage properties.
// This is called from commands_menu.rs
pub fn print_results_of_check_disk_health() -> windows::core::Result<()>  {
    unsafe {
        println!("{}", "Starting Disk Health Check...".bright_green().bold());
        // Path to the physical drive
        let handle =  
                CreateFileW(
                w!("\\\\.\\PhysicalDrive0"),
                GENERIC_READ.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )?
        ;

        println!("{}", "\n=== Storage Device Information ===\n".bright_cyan().bold());

        // 1. Query StorageDeviceProperty (Device Info)
        check_query_function_support(
            query_device_property(handle),
            "⚠ Device Property not supported on this drive",
        );

        // 2. Query StorageAdapterProperty (Controller Info)
        check_query_function_support(
            query_adapter_property(handle),
            "⚠ Adapter Property not supported on this drive",
        );

        // 3. Query StorageDeviceIdProperty (Device ID)
        check_query_function_support(
            query_device_id_property(handle),
            "⚠ Device ID Property not supported on this drive",
        );

        // 4. (Optional) Query Physical Element Status (Health Monitoring)
        // This may not be supported on all drives/systems
        check_query_function_support(
            query_physical_element_status(handle),
            "⚠ Physical Element Status not supported on this drive",
        );

        let _ = CloseHandle(handle);
    }
    Ok(())
}


fn check_query_function_support(query_function: windows::core::Result<()>,  message_error: &str) {
    // This function can be used to check if a query function is supported
    // Currently not implemented

    match query_function {
        Ok(_) => {},
        Err(_e) => {
            println!("{}", message_error.bright_yellow());
            println!("{}", format!("  (This is normal for some drives)").dimmed());
        }
    }
}


// Query basic device information (model, vendor, type, etc.)
unsafe fn query_device_property(handle: HANDLE) -> windows::core::Result<()> {
    // Create input query structure
    let query = STORAGE_PROPERTY_QUERY {
        PropertyId: StorageDeviceProperty,
        QueryType: PropertyStandardQuery,
        AdditionalParameters: [0u8; 1],
    };

    let output: Vec<u8> = unsafe { call_ioctl_storage_query_property(handle, &query)? };


    // Parse the output as STORAGE_DEVICE_DESCRIPTOR
    let descriptor = unsafe { &*(output.as_ptr() as *const STORAGE_DEVICE_DESCRIPTOR) };

    println!("{}", "► Device Properties:".bright_yellow());
    
    // Device Type
    print!("  Device Type: ");
    match descriptor.DeviceType {
        0 => println!("Direct Access (HDD/SSD)"),
        1 => println!("Sequential Access (Tape)"),
        5 => println!("CD/DVD-ROM"),
        _ => println!("Unknown ({})", descriptor.DeviceType),
    }

    // Bus Type
    print!("  Bus Type: ");
    let bus_type_nvme = STORAGE_BUS_TYPE(17);
    let bus_type_sata = STORAGE_BUS_TYPE(11);
    let bus_type_usb = STORAGE_BUS_TYPE(7);
    let bus_type_scsi = STORAGE_BUS_TYPE(1);
    
    match descriptor.BusType {
        t if t == bus_type_nvme => println!("{}", "NVMe".bright_green()),
        t if t == bus_type_sata => println!("{}", "SATA".bright_blue()),
        t if t == bus_type_usb => println!("USB"),
        t if t == bus_type_scsi => println!("SCSI"),
        _ => println!("Unknown ({:?})", descriptor.BusType),
    }

    println!("  Removable: {}", descriptor.RemovableMedia.as_bool());
    println!("  Command Queueing: {}", descriptor.CommandQueueing.as_bool());

    // Extract strings from descriptor
    if descriptor.VendorIdOffset > 0 && descriptor.VendorIdOffset < output.len() as u32 {
        let vendor = get_string_from_offset(&output, descriptor.VendorIdOffset as usize);
        println!("  Vendor: {}", vendor.bright_cyan());
    }

    if descriptor.ProductIdOffset > 0 && descriptor.ProductIdOffset < output.len() as u32 {
        let product = get_string_from_offset(&output, descriptor.ProductIdOffset as usize);
        println!("  Product: {}", product.bright_cyan());
    }

    if descriptor.ProductRevisionOffset > 0 && descriptor.ProductRevisionOffset < output.len() as u32 {
        let revision = get_string_from_offset(&output, descriptor.ProductRevisionOffset as usize);
        println!("  Revision: {}", revision);
    }

    if descriptor.SerialNumberOffset > 0 && descriptor.SerialNumberOffset < output.len() as u32 {
        let serial = get_string_from_offset(&output, descriptor.SerialNumberOffset as usize);
        println!("  Serial Number: {}", serial.bright_magenta());
    }

    println!();
    Ok(())
}

// Query adapter/controller information
unsafe fn query_adapter_property(handle: HANDLE) -> windows::core::Result<()> {
    let query = STORAGE_PROPERTY_QUERY {
        PropertyId: StorageAdapterProperty,
        QueryType: PropertyStandardQuery,
        AdditionalParameters: [0u8; 1],
    };

    let output: Vec<u8> = unsafe { call_ioctl_storage_query_property(handle, &query)? };


    let descriptor = unsafe { &*(output.as_ptr() as *const STORAGE_ADAPTER_DESCRIPTOR) };

    println!("{}", "► Adapter Properties:".bright_yellow());
    println!("  Max Transfer Length: {} bytes", descriptor.MaximumTransferLength);
    println!("  Max Physical Pages: {}", descriptor.MaximumPhysicalPages);
    println!("  Alignment Mask: 0x{:X}", descriptor.AlignmentMask);
    println!("  Adapter Version: {}.{}", 
        descriptor.BusMajorVersion,
        descriptor.BusMinorVersion
    );
    println!("  Command Queueing: {}", descriptor.CommandQueueing.as_bool());
    println!("  Accelerated Transfer: {}", descriptor.AcceleratedTransfer.as_bool());
    println!("  Adapter Scan Down: {}", descriptor.AdapterScansDown.as_bool());
    println!("  Bus Type: {:?}", descriptor.BusType);
    println!("  Srb Type: {:?}", descriptor.SrbType);
    println!("  Address type: {:?}", descriptor.AddressType);

    println!("  Size of Descriptor: {} bytes", descriptor.Size);
    println!("  Version: {}", descriptor.Version);

    println!();
    Ok(())
}

// Query device unique identifiers
unsafe fn query_device_id_property(handle: HANDLE) -> windows::core::Result<()> {
    let query = STORAGE_PROPERTY_QUERY {
        PropertyId: StorageDeviceIdProperty,
        QueryType: PropertyStandardQuery,
        AdditionalParameters: [0u8; 1],
    };

    let output = unsafe { call_ioctl_storage_query_property(handle, &query)? };

    let descriptor = unsafe { &*(output.as_ptr() as *const STORAGE_DEVICE_ID_DESCRIPTOR) };

    println!("{}", "► Device ID Properties:".bright_yellow());
    println!("  Number of Identifiers: {}", descriptor.NumberOfIdentifiers);
    
    // Parse identifiers (advanced - shows unique device IDs like WWN, EUI-64)
    if descriptor.NumberOfIdentifiers > 0 {
        println!("  {} Device identifiers available", descriptor.NumberOfIdentifiers);
    }

    
    
    println!();
    Ok(())
}

// Helper function to extract null-terminated strings from byte arrays
fn get_string_from_offset(buffer: &[u8], offset: usize) -> String {
    if offset >= buffer.len() {
        return String::new();
    }

    let slice = &buffer[offset..];
    let end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
    String::from_utf8_lossy(&slice[..end]).trim().to_string()
}

// Function for STORAGE_PROPERTY_QUERY (Device/Adapter/ID properties)
unsafe fn call_ioctl_storage_query_property(
        handle: HANDLE,
        query: &STORAGE_PROPERTY_QUERY,
    ) -> windows::core::Result<Vec<u8>> {
        let mut output = [0u8; 4096];
        let mut bytes_returned: u32 = 0;
        
        unsafe {
            DeviceIoControl(
                handle,
                IOCTL_STORAGE_QUERY_PROPERTY,  // This IOCTL
                Some(query as *const _ as *const _),
                mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                Some(output.as_mut_ptr() as *mut _),
                output.len() as u32,
                Some(&mut bytes_returned),
                None,
            )?;
        }

        let data = output[..bytes_returned as usize].to_vec();
        Ok(data)
    }

// Separate function for PHYSICAL_ELEMENT_STATUS (Health monitoring)
unsafe fn call_ioctl_physical_element_status(
        handle: HANDLE,
        request: &PHYSICAL_ELEMENT_STATUS_REQUEST,
    ) -> windows::core::Result<Vec<u8>> {
        let mut output = [0u8; 4096];
        let mut bytes_returned: u32 = 0;
        
        unsafe {
            DeviceIoControl(
                handle,
                IOCTL_STORAGE_GET_PHYSICAL_ELEMENT_STATUS,  // Different IOCTL!
                Some(request as *const _ as *const _),
                mem::size_of::<PHYSICAL_ELEMENT_STATUS_REQUEST>() as u32,
                Some(output.as_mut_ptr() as *mut _),
                output.len() as u32,
                Some(&mut bytes_returned),
                None,
            )?;
        }

        let data = output[..bytes_returned as usize].to_vec();
        Ok(data)
    }

//using the IOCTL_STORAGE_GET_PHYSICAL_ELEMENT_STATUS to get the physical status of the disk
// This function is not used currently but can be implemented for more detailed health checks
unsafe fn query_physical_element_status(handle: HANDLE) -> windows::core::Result<()> {
    let request = PHYSICAL_ELEMENT_STATUS_REQUEST {
        Version: std::mem::size_of::<PHYSICAL_ELEMENT_STATUS_REQUEST>() as u32,
        Size: std::mem::size_of::<PHYSICAL_ELEMENT_STATUS_REQUEST>() as u32,
        StartingElement: 0,  // Start from first element
        Filter: 0,           // 0 = all elements, 1 = only restored elements
        ReportType: 0,       // 0 = physical element status
        Reserved: [0u8; 2],
    };
    
    // Use the dedicated function for physical element status
    let output = unsafe { call_ioctl_physical_element_status(handle, &request)? };
    
    // Parse the output
    let descriptor = unsafe { &*(output.as_ptr() as *const PHYSICAL_ELEMENT_STATUS_DESCRIPTOR) };
    
    println!("{}", "► Physical Element Status:".bright_yellow());
    println!("  Version: {}", descriptor.Version);
    println!("  Size: {}", descriptor.Size);
    println!("  Number of Elements: {}", descriptor.ElementIdentifier);
    // println!("  Health Type: {}", descriptor.PhysicalElementType);
    println!("  Health Status: ");
    match descriptor.PhysicalElementHealth {
        0 => println!("    Healthy"),
        1 => println!("    Warning"),
        2 => println!("    Critical"),
        _ => println!("    Unknown ({})", descriptor.PhysicalElementHealth),
    }

    Ok(())
}

