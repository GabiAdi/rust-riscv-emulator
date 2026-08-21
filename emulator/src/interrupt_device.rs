use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub trait InterruptDevice {
    fn interrupt_bit(&self) -> u32;
    fn trigger_interrupt(&mut self, mip_shared: &Rc<Cell<u64>>);
}

impl<T: InterruptDevice> InterruptDevice for Rc<RefCell<T>> {
    fn interrupt_bit(&self) -> u32 {
        self.borrow().interrupt_bit()
    }
    fn trigger_interrupt(&mut self, mip_shared: &Rc<Cell<u64>>) {
        self.borrow_mut().trigger_interrupt(mip_shared);
    }
}
