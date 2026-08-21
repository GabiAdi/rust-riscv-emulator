use crate::interrupt_device::InterruptDevice;
use crate::memory_device::{DeviceError, DeviceType, MemoryDevice};
use std::cell::Cell;
use std::rc::Rc;

pub struct TestDevice {
    base: u64,
    pub pending: bool,
}

impl TestDevice {
    pub fn new(base: u64) -> TestDevice {
        TestDevice {
            base,
            pending: false,
        }
    }
}

impl MemoryDevice for TestDevice {
    fn get_size(&self) -> u64 {
        8
    }
    fn get_start_addr(&self) -> u64 {
        self.base
    }
    fn get_end_addr(&self) -> u64 {
        self.base + 8
    }
    fn read_byte(&self, addr: u64) -> Result<u64, DeviceError> {
        Ok(self.pending as u64)
    }
    fn read_hword(&self, addr: u64) -> Result<u64, DeviceError> {
        Ok(self.pending as u64)
    }
    fn read_word(&self, addr: u64) -> Result<u64, DeviceError> {
        Ok(self.pending as u64)
    }

    fn read_dword(&self, addr: u64) -> Result<u64, DeviceError> {
        Ok(self.pending as u64)
    }

    fn write_byte(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        self.pending = value != 0;
        Ok(())
    }

    fn write_hword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        self.pending = value != 0;
        Ok(())
    }
    fn write_word(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        self.pending = value != 0;
        Ok(())
    }
    fn write_dword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        self.pending = value != 0;
        Ok(())
    }

    fn get_device_type(&self) -> DeviceType {
        DeviceType::TestDevice
    }

    fn clear(&mut self) {
        self.pending = false;
    }
}

impl InterruptDevice for TestDevice {
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
