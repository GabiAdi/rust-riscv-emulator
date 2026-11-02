use core::borrow;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::cpu::{Cpu, MemoryWidth};
use crate::dma_device::{self, DmaDevice};
use crate::interrupt_device::{DeviceInterrupt, InterruptDevice};
use crate::memory_device::{DeviceError, DeviceType, MemoryDevice};

pub struct Bus {
    memory_devices: Vec<RefCell<Box<dyn MemoryDevice>>>,
    interrupt_devices: Vec<RefCell<Box<dyn InterruptDevice>>>,
    dma_devices: Vec<Box<RefCell<dyn DmaDevice>>>,
    max_addr: u64,
    mip_shared: Rc<Cell<u64>>,

    interrupts: RefCell<Vec<DeviceInterrupt>>,
    interrupt_controller_base: u64,

    pub halt_cpu: bool,
}

impl Bus {
    pub fn new(
        memory_devices: Vec<RefCell<Box<dyn MemoryDevice>>>,
        interrupt_devices: Vec<RefCell<Box<dyn InterruptDevice>>>,
        dma_devices: Vec<Box<RefCell<dyn DmaDevice>>>,
        mip_shared: Rc<Cell<u64>>,
        ic_base: u64,
    ) -> Bus {
        let mut max_addr: u64 = 0;

        for device in &memory_devices {
            if device.borrow().get_end_addr() > max_addr {
                max_addr = device.borrow().get_end_addr()
            }
        }

        let bus = Bus {
            memory_devices,
            interrupt_devices,
            dma_devices,
            max_addr,
            mip_shared,

            interrupts: RefCell::new(Vec::new()),
            interrupt_controller_base: ic_base,

            halt_cpu: false,
        };

        bus
    }

    pub fn attach_dma_devices(&self, bus: &Rc<Bus>) {
        for device in &self.dma_devices {
            device.borrow_mut().attach_bus(Rc::downgrade(bus));
        }
    }

    pub fn tick_interrupts(&self) {
        for device in &self.interrupt_devices {
            if let Some(int) = device.borrow_mut().trigger_interrupt()
                && !self.interrupts.borrow().contains(&int)
            {
                self.interrupts.borrow_mut().push(int);
            }
        }

        for int in self.interrupts.borrow().iter() {
            let mut mip = self.mip_shared.get();
            mip |= int.bit as u64;
            self.mip_shared.set(mip);
        }

        if self.interrupts.borrow().len() == 0 {
            let mut mip = self.mip_shared.get();
            mip &= !(1 << 11);
            mip &= !(1 << 7);
            self.mip_shared.set(mip);
        }
    }

    pub fn dma_read(&self, addr: u64, buf: &mut [u8]) -> Result<(), DeviceError> {
        let device = self.get_device(addr)?;
        let offset = addr - device.borrow().get_start_addr();
        for (i, byte) in buf.iter_mut().enumerate() {
            *byte = device.borrow().read_byte(offset + i as u64)? as u8;
        }
        Ok(())
    }

    pub fn dma_write(&self, addr: u64, buf: &[u8]) -> Result<(), DeviceError> {
        let device = self.get_device(addr)?;
        let offset = addr - device.borrow().get_start_addr();
        for (i, byte) in buf.iter().enumerate() {
            device
                .borrow_mut()
                .write_byte(offset + i as u64, *byte as u64)?;
        }
        Ok(())
    }

    pub fn read(&self, addr: u64, width: MemoryWidth) -> Result<u64, DeviceError> {
        if (addr == self.interrupt_controller_base || addr == self.interrupt_controller_base + 8)
            && let Some(int) = self.interrupts.borrow().first()
        {
            return Ok(if addr == self.interrupt_controller_base {
                int.device_id
            } else {
                int.start_addr
            });
        }
        let device = self.get_device(addr)?;
        let start_addr = device.borrow().get_start_addr();
        match width {
            MemoryWidth::Byte => device.borrow().read_byte(addr - start_addr),
            MemoryWidth::Hword => device.borrow().read_hword(addr - start_addr),
            MemoryWidth::Word => device.borrow().read_word(addr - start_addr),
            MemoryWidth::Dword => device.borrow().read_dword(addr - start_addr),
        }
    }

    pub fn write(&self, addr: u64, value: u64, width: MemoryWidth) -> Result<(), DeviceError> {
        if addr == self.interrupt_controller_base || addr == self.interrupt_controller_base + 8 {
            if self.interrupts.borrow().len() > 0 {
                self.interrupts.borrow_mut().remove(0);
            }
            return Ok(());
        }
        let device = self.get_device(addr)?;
        let start_addr = device.borrow().get_start_addr();
        match width {
            MemoryWidth::Byte => device.borrow_mut().write_byte(addr - start_addr, value),
            MemoryWidth::Hword => device.borrow_mut().write_hword(addr - start_addr, value),
            MemoryWidth::Word => device.borrow_mut().write_word(addr - start_addr, value),
            MemoryWidth::Dword => device.borrow_mut().write_dword(addr - start_addr, value),
        }
    }

    fn get_device(&self, addr: u64) -> Result<&RefCell<Box<dyn MemoryDevice>>, DeviceError> {
        for device in &self.memory_devices {
            let d = device.borrow();
            if addr >= d.get_start_addr() && addr < d.get_end_addr() {
                // match device.get_device_type() {
                //     DeviceType::KeyboardDevice => {
                //         println!("KeyboardDevice at {:#018x}", addr);
                //     }
                //     _ => {}
                // }
                drop(d);
                return Ok(device);
            }
        }
        Err(DeviceError::DeviceNotFound { addr })
    }
}
