use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::bus::Bus;

pub trait DmaDevice {
    fn attach_bus(&mut self, bus: Weak<Bus>);
    fn bus_ref(&self) -> &Weak<Bus>;

    fn bus(&self) -> Rc<Bus> {
        self.bus_ref().upgrade().expect("Bus dropped before device")
    }
}
