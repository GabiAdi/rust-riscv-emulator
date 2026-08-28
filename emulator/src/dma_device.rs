use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::bus::Bus;

pub trait DmaDevice {
    fn attach_bus(&mut self, bus: Weak<Bus>);
    fn bus(&self) -> Rc<Bus>;
}

impl<T: DmaDevice> DmaDevice for Rc<RefCell<T>> {
    fn attach_bus(&mut self, bus: Weak<Bus>) {
        self.borrow_mut().attach_bus(bus);
    }
    fn bus(&self) -> Rc<Bus> {
        self.borrow().bus()
    }
}
