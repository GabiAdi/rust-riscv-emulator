use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::cpu::{Cpu, MemoryWidth};
use crate::interrupt_device::InterruptDevice;
use crate::memory_device::{DeviceError, DeviceType, MemoryDevice};

pub struct Bus {
    memory_devices: Vec<Box<dyn MemoryDevice>>,
    interrupt_devices: Vec<Box<dyn InterruptDevice>>,
    cpu: Weak<RefCell<Cpu>>,
    max_addr: u64,
    mip_shared: Rc<Cell<u64>>,

    pub halt_cpu: bool,
    pub tohost_addr: Option<u64>,
}

impl Bus {
    pub fn new(
        memory_devices: Vec<Box<dyn MemoryDevice>>,
        interrupt_devices: Vec<Box<dyn InterruptDevice>>,
        mip_shared: Rc<Cell<u64>>,
    ) -> Bus {
        let mut max_addr: u64 = 0;

        for device in &memory_devices {
            if device.get_end_addr() > max_addr {
                max_addr = device.get_end_addr()
            }
        }

        Bus {
            memory_devices,
            interrupt_devices,
            max_addr,
            cpu: Weak::new(),
            mip_shared,

            halt_cpu: false,
            tohost_addr: None,
        }
    }

    pub fn attach_cpu(&mut self, cpu: &Rc<RefCell<Cpu>>) {
        self.cpu = Rc::downgrade(cpu);
    }

    pub fn tick_interrupts(&mut self) {
        for device in &mut self.interrupt_devices {
            device.trigger_interrupt(&self.mip_shared);
        }
    }

    pub fn read(&self, addr: u64, width: MemoryWidth) -> Result<u64, DeviceError> {
        match width {
            MemoryWidth::Byte => self.read_byte(addr),
            MemoryWidth::Hword => self.read_hword(addr),
            MemoryWidth::Word => self.read_word(addr),
            MemoryWidth::Dword => self.read_dword(addr),
        }
    }

    pub fn write(&mut self, addr: u64, value: u64, width: MemoryWidth) -> Result<(), DeviceError> {
        if addr >= 0x1000 && addr <= 0x2000 {
            println!("Wrote: {:#018x} to {:#018x}", value, addr);
        }
        match self.tohost_addr {
            Some(tohost_addr) => {
                if tohost_addr == addr {
                    self.halt_cpu = true;
                }
            }
            None => {}
        }
        match width {
            MemoryWidth::Byte => self.write_byte(addr, value),
            MemoryWidth::Hword => self.write_hword(addr, value),
            MemoryWidth::Word => self.write_word(addr, value),
            MemoryWidth::Dword => self.write_dword(addr, value),
        }
    }

    pub fn read_byte(&self, addr: u64) -> Result<u64, DeviceError> {
        match self.get_device(addr) {
            Ok(device) => device.read_byte(addr - device.get_start_addr()),
            Err(error) => Err(error),
        }
    }

    pub fn read_hword(&self, addr: u64) -> Result<u64, DeviceError> {
        match self.get_device(addr) {
            Ok(device) => device.read_hword(addr - device.get_start_addr()),
            Err(error) => Err(error),
        }
    }

    pub fn read_word(&self, addr: u64) -> Result<u64, DeviceError> {
        match self.get_device(addr) {
            Ok(device) => device.read_word(addr - device.get_start_addr()),
            Err(error) => Err(error),
        }
    }

    pub fn read_dword(&self, addr: u64) -> Result<u64, DeviceError> {
        match self.get_device(addr) {
            Ok(device) => device.read_dword(addr - device.get_start_addr()),
            Err(error) => Err(error),
        }
    }

    pub fn write_byte(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        match self.get_device_mut(addr) {
            Ok(device) => device.write_byte(addr - device.get_start_addr(), value),
            Err(error) => Err(error),
        }
    }

    pub fn write_hword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        match self.get_device_mut(addr) {
            Ok(device) => device.write_hword(addr - device.get_start_addr(), value),
            Err(error) => Err(error),
        }
    }

    pub fn write_word(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        match self.get_device_mut(addr) {
            Ok(device) => device.write_word(addr - device.get_start_addr(), value),
            Err(error) => Err(error),
        }
    }

    pub fn write_dword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        match self.get_device_mut(addr) {
            Ok(device) => device.write_dword(addr - device.get_start_addr(), value),
            Err(error) => Err(error),
        }
    }

    fn get_device(&self, addr: u64) -> Result<&Box<dyn MemoryDevice>, DeviceError> {
        for device in &self.memory_devices {
            if addr >= device.get_start_addr() && addr < device.get_end_addr() {
                // match device.get_device_type() {
                //     DeviceType::KeyboardDevice => {
                //         println!("KeyboardDevice at {:#018x}", addr);
                //     }
                //     _ => {}
                // }
                return Ok(device);
            }
        }
        Err(DeviceError::DeviceNotFound { addr })
    }

    fn get_device_mut(&mut self, addr: u64) -> Result<&mut Box<dyn MemoryDevice>, DeviceError> {
        for device in &mut self.memory_devices {
            if addr >= device.get_start_addr() && addr < device.get_end_addr() {
                return Ok(device);
            }
        }
        Err(DeviceError::DeviceNotFound { addr })
    }
}
