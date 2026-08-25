use std::{cell::RefCell, rc::Weak};

use crate::{
    bus::Bus,
    dma_device::DmaDevice,
    memory_device::{DeviceError, DeviceType, MemoryDevice},
};

pub struct Disk {
    start_addr: u64,
    end_addr: u64,
    size: u64,

    data: Vec<u8>,

    sector: u64,       // 0x0
    sector_count: u32, // 0x8 Number of sectors
    disk_cmd: u16,     // 0xC 0 = idle, 1 = read, 2 = write
    disk_status: u16,  // 0xE 0 = idle, 1 = busy, 2 = done, 3 = error
    disk_bufptr: u64,  // 0x10 Addr of DMA buffer

    bus: Option<Weak<Bus>>,
}

impl Disk {
    pub fn new(start_addr: u64) -> Disk {
        Disk {
            start_addr,
            end_addr: start_addr + 0x18,
            size: 0x18,

            data: Vec::with_capacity(0x18),

            sector: 0,
            sector_count: 0,
            disk_cmd: 0,
            disk_status: 0,
            disk_bufptr: 0,

            bus: None,
        }
    }

    fn update_regs(&mut self) {
        self.sector = u64::from_le_bytes(self.data[0x00..0x08].try_into().unwrap());
        self.sector_count = u32::from_le_bytes(self.data[0x08..0x0C].try_into().unwrap());
        self.disk_cmd = u16::from_le_bytes(self.data[0x0C..0x0E].try_into().unwrap());
        self.disk_status = u16::from_le_bytes(self.data[0x0E..0x10].try_into().unwrap());
        self.disk_bufptr = u64::from_le_bytes(self.data[0x10..0x18].try_into().unwrap());
    }

    fn disk_io(&mut self) {}
}

impl MemoryDevice for Disk {
    fn get_size(&self) -> u64 {
        self.size
    }
    fn get_start_addr(&self) -> u64 {
        self.start_addr
    }
    fn get_end_addr(&self) -> u64 {
        self.end_addr
    }
    fn read_byte(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr >= self.size {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        Ok(self.data[addr as usize] as u64)
    }

    fn read_hword(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr >= self.size + 1 {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        let offset = addr as usize;
        Ok(u16::from_le_bytes(self.data[offset..offset + 2].try_into().unwrap()) as u64)
    }

    fn read_word(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr >= self.size + 3 {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        let offset = addr as usize;
        Ok(u32::from_le_bytes(self.data[offset..offset + 4].try_into().unwrap()) as u64)
    }

    fn read_dword(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr >= self.size + 7 {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        let offset = addr as usize;
        Ok(u64::from_le_bytes(self.data[offset..offset + 8].try_into().unwrap()) as u64)
    }

    fn write_byte(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        if addr >= self.size {
            return Err(DeviceError::AddrOutOfRange { addr });
        }

        let offset = addr as usize;
        self.data[offset] = value as u8;
        self.update_regs();

        Ok(())
    }

    fn write_hword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        if addr >= self.size + 1 {
            return Err(DeviceError::AddrOutOfRange { addr });
        }

        let offset = addr as usize;
        let bytes = (value as u16).to_le_bytes();
        self.data[offset..offset + 2].copy_from_slice(&bytes);
        self.update_regs();

        Ok(())
    }

    fn write_word(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        if addr >= self.size + 3 {
            return Err(DeviceError::AddrOutOfRange { addr });
        }

        let offset = addr as usize;
        let bytes = (value as u32).to_le_bytes();
        self.data[offset..offset + 4].copy_from_slice(&bytes);
        self.update_regs();

        Ok(())
    }

    fn write_dword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        if addr >= self.size + 7 {
            return Err(DeviceError::AddrOutOfRange { addr });
        }

        let offset = addr as usize;
        let bytes = (value as u64).to_le_bytes();
        self.data[offset..offset + 8].copy_from_slice(&bytes);
        self.update_regs();

        Ok(())
    }

    fn get_device_type(&self) -> DeviceType {
        DeviceType::Disk
    }

    fn clear(&mut self) {
        self.data.fill(0);
        self.update_regs();
    }
}

impl DmaDevice for Disk {
    fn attach_bus(&mut self, bus: Weak<Bus>) {
        self.bus = Some(bus);
    }
    fn bus_ref(&self) -> &Weak<Bus> {
        match &self.bus {
            Some(b) => b,
            None => panic!("No bus"),
        }
    }
}
