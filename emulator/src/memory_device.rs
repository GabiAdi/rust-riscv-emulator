use std::{cell::RefCell, rc::Rc};

#[derive(Debug, PartialEq)]
pub enum DeviceType {
    Ram,
    Rom,
    Console,
    KeyboardDevice,
    TestDevice,
}

#[derive(Debug)]
pub enum DeviceError {
    AddrOutOfRange { addr: u64 },
    DeviceNotFound { addr: u64 },
}

pub trait MemoryDevice {
    fn get_size(&self) -> u64;
    fn get_start_addr(&self) -> u64;
    fn get_end_addr(&self) -> u64;

    fn read_byte(&self, addr: u64) -> Result<u64, DeviceError>;
    fn read_hword(&self, addr: u64) -> Result<u64, DeviceError>;
    fn read_word(&self, addr: u64) -> Result<u64, DeviceError>;
    fn read_dword(&self, addr: u64) -> Result<u64, DeviceError>;

    fn write_byte(&mut self, addr: u64, value: u64) -> Result<(), DeviceError>;
    fn write_hword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError>;
    fn write_word(&mut self, addr: u64, value: u64) -> Result<(), DeviceError>;
    fn write_dword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError>;

    fn get_device_type(&self) -> DeviceType;

    fn clear(&mut self);
}

impl<T: MemoryDevice> MemoryDevice for Rc<RefCell<T>> {
    fn get_size(&self) -> u64 {
        self.borrow().get_size()
    }
    fn get_start_addr(&self) -> u64 {
        self.borrow().get_start_addr()
    }
    fn get_end_addr(&self) -> u64 {
        self.borrow().get_end_addr()
    }
    fn read_byte(&self, addr: u64) -> Result<u64, DeviceError> {
        self.borrow().read_byte(addr)
    }
    fn read_hword(&self, addr: u64) -> Result<u64, DeviceError> {
        self.borrow().read_hword(addr)
    }
    fn read_word(&self, addr: u64) -> Result<u64, DeviceError> {
        self.borrow().read_word(addr)
    }
    fn read_dword(&self, addr: u64) -> Result<u64, DeviceError> {
        self.borrow().read_dword(addr)
    }
    fn write_byte(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        self.borrow_mut().write_byte(addr, value)
    }
    fn write_hword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        self.borrow_mut().write_hword(addr, value)
    }
    fn write_word(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        self.borrow_mut().write_word(addr, value)
    }
    fn write_dword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        self.borrow_mut().write_dword(addr, value)
    }
    fn get_device_type(&self) -> DeviceType {
        self.borrow().get_device_type()
    }
    fn clear(&mut self) {
        self.borrow_mut().clear()
    }
}
