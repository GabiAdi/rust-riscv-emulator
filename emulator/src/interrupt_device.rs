use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DeviceInterrupt {
    pub bit: u32,
    pub device_id: u64,
    pub start_addr: u64,
}

pub trait InterruptDevice {
    fn interrupt_bit(&self) -> u32;
    fn trigger_interrupt(&mut self) -> Option<DeviceInterrupt>;
}

impl<T: InterruptDevice> InterruptDevice for Rc<RefCell<T>> {
    fn interrupt_bit(&self) -> u32 {
        self.borrow().interrupt_bit()
    }
    fn trigger_interrupt(&mut self) -> Option<DeviceInterrupt> {
        self.borrow_mut().trigger_interrupt()
    }
}
