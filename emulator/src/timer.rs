use crate::{
    cpu::Interrupt,
    interrupt_device::{DeviceInterrupt, InterruptDevice},
    memory_device::{DeviceError, DeviceType, MemoryDevice},
};
use std::{cell::Cell, rc::Rc, time::Instant};

pub struct Timer {
    start_addr: u64,
    end_addr: u64,
    size: u64,
    count_time: u64,
    counting: bool,
    pending: bool,
    start_time: Option<Instant>,
    device_id: u64,
}

impl Timer {
    pub fn new(start_addr: u64) -> Timer {
        Timer {
            start_addr,
            end_addr: start_addr + 18,
            size: 18,
            count_time: 0,
            counting: false,
            pending: false,
            start_time: None,
            device_id: 0xC000000000000000,
        }
    }
}

impl MemoryDevice for Timer {
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
        if addr == 0 {
            Ok(self.device_id as u8 as u64)
        } else if addr == 8 {
            Ok(self.count_time as u8 as u64)
        } else if addr == 16 {
            Ok(self.counting as u64)
        } else {
            Err(DeviceError::InvalidAlign { addr })
        }
    }
    fn read_hword(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr == 0 {
            Ok(self.device_id as u16 as u64)
        } else if addr == 8 {
            Ok(self.count_time as u16 as u64)
        } else if addr == 16 {
            Ok(self.counting as u64)
        } else {
            Err(DeviceError::InvalidAlign { addr })
        }
    }
    fn read_word(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr == 0 {
            Ok(self.device_id as u32 as u64)
        } else if addr == 8 {
            Ok(self.count_time as u32 as u64)
        } else if addr == 16 {
            Ok(self.counting as u64)
        } else {
            Err(DeviceError::InvalidAlign { addr })
        }
    }
    fn read_dword(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr == 0 {
            Ok(self.device_id)
        } else if addr == 8 {
            Ok(self.count_time)
        } else if addr == 16 {
            Ok(self.counting as u64)
        } else {
            Err(DeviceError::InvalidAlign { addr })
        }
    }
    fn write_byte(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        if addr == 0 {
            return Ok(());
        } else if addr == 8 {
            self.count_time = value as u8 as u64;
        } else if addr == 16 {
            self.counting = value != 0;
            if self.counting {
                self.start_time = Some(Instant::now());
            } else {
                self.start_time = None;
                self.pending = false;
            }
        } else {
            return Err(DeviceError::InvalidAlign { addr });
        }
        Ok(())
    }
    fn write_hword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        if addr == 0 {
            return Ok(());
        } else if addr == 8 {
            self.count_time = value as u16 as u64;
        } else if addr == 16 {
            self.counting = value != 0;
            if self.counting {
                self.start_time = Some(Instant::now());
            } else {
                self.start_time = None;
                self.pending = false;
            }
        } else {
            return Err(DeviceError::InvalidAlign { addr });
        }
        Ok(())
    }
    fn write_word(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        if addr == 0 {
            return Ok(());
        } else if addr == 8 {
            self.count_time = value as u32 as u64;
        } else if addr == 16 {
            self.counting = value != 0;
            if self.counting {
                self.start_time = Some(Instant::now());
            } else {
                self.start_time = None;
                self.pending = false;
            }
        } else {
            return Err(DeviceError::InvalidAlign { addr });
        }
        Ok(())
    }
    fn write_dword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        if addr == 0 {
            return Ok(());
        } else if addr == 8 {
            self.count_time = value;
        } else if addr == 16 {
            self.counting = value != 0;
            if self.counting {
                self.start_time = Some(Instant::now());
            } else {
                self.start_time = None;
                self.pending = false;
            }
        } else {
            return Err(DeviceError::InvalidAlign { addr });
        }
        Ok(())
    }
    fn get_device_type(&self) -> DeviceType {
        DeviceType::Timer
    }
    fn clear(&mut self) {
        self.count_time = 0;
        self.counting = false;
        self.pending = false;
        self.start_time = None;
    }
}

impl InterruptDevice for Timer {
    fn interrupt_bit(&self) -> u32 {
        11
    }
    fn trigger_interrupt(&mut self) -> Option<DeviceInterrupt> {
        if let Some(time) = self.start_time {
            if time.elapsed().as_nanos() > self.count_time as u128 && self.counting {
                self.pending = true;
            }
        }
        if !self.pending {
            return None;
        }
        let bit = 1 << self.interrupt_bit();

        Some(DeviceInterrupt {
            bit: bit,
            device_id: self.device_id,
            start_addr: self.start_addr,
        })
        // let mut mip = mip_shared.get();
        // if self.pending {
        //     mip |= bit;
        // } else {
        //     mip &= !bit;
        // }
        // mip_shared.set(mip);
    }
}
