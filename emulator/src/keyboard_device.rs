use crate::interrupt_device::InterruptDevice;
use crate::memory_device::{DeviceError, DeviceType, MemoryDevice};
use std::cell::Cell;
use std::rc::Rc;

pub struct KeyboardDevice {
    start_addr: u64,
    end_addr: u64,
    size: u64,
    device_id: u64,
    last_key: Option<u8>,
    pending: bool,
}

impl KeyboardDevice {
    pub fn new(start_addr: u64) -> KeyboardDevice {
        KeyboardDevice {
            start_addr,
            end_addr: start_addr + 0x10,
            size: 0x10,
            // bits 0 - 60 identifier
            // bit 61 - DMA device
            // bit 62 - interrupt device
            // bit 63 - memory device (always set)
            device_id: 0xC000000000000001,
            last_key: None,
            pending: false,
        }
    }

    pub fn push_key(&mut self, key: u8) {
        self.last_key = Some(key);
        self.pending = true;
    }
}

impl MemoryDevice for KeyboardDevice {
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
        Ok(self.last_key.unwrap_or(0) as u64)
    }
    fn read_hword(&self, addr: u64) -> Result<u64, DeviceError> {
        Ok(self.last_key.unwrap_or(0) as u64)
    }
    fn read_word(&self, addr: u64) -> Result<u64, DeviceError> {
        Ok(self.last_key.unwrap_or(0) as u64)
    }

    fn read_dword(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr == 0x0 {
            return Ok(self.device_id);
        }
        Ok(self.last_key.unwrap_or(0) as u64)
    }

    fn write_byte(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        self.pending = false;
        Ok(())
    }

    fn write_hword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        self.pending = false;
        Ok(())
    }
    fn write_word(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        self.pending = false;
        Ok(())
    }
    fn write_dword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        self.pending = false;
        Ok(())
    }

    fn get_device_type(&self) -> DeviceType {
        DeviceType::KeyboardDevice
    }

    fn clear(&mut self) {
        self.pending = false;
        self.last_key = None;
    }
}

impl InterruptDevice for KeyboardDevice {
    fn interrupt_bit(&self) -> u32 {
        11
    }

    fn trigger_interrupt(&mut self, mip_shared: &Rc<Cell<u64>>) {
        let bit = 1 << self.interrupt_bit();
        let mut mip = mip_shared.get();
        if self.pending {
            mip |= bit;
        } else {
            mip &= !bit;
        }
        mip_shared.set(mip);
    }
}
