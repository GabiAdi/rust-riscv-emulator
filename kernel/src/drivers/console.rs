use core::fmt;

use alloc::vec::Vec;

use crate::{MMIO_CONSOLE, drivers::mmio_driver::MmioDriver};

pub struct ConsoleWriter {
    mmio_console: *mut u8,
    addr: u64,
}

impl ConsoleWriter {
    pub unsafe fn new_unchecked(addr: u64) -> ConsoleWriter {
        ConsoleWriter {
            addr,
            mmio_console: addr as *mut u8,
        }
    }

    pub fn new(addr: u64) -> Option<ConsoleWriter> {
        let mmio: *mut u64 = addr as *mut u64;
        let device_id: u64 = unsafe { mmio.read_volatile() };

        if device_id != 0xC000000000000003 {
            return None;
        }
        Some(ConsoleWriter {
            addr,
            mmio_console: addr as *mut u8,
        })
    }

    pub unsafe fn print_char(&self, c: u8) {
        unsafe {
            self.mmio_console.write_volatile(c);
        }
    }

    pub fn print_vec_u8(&self, v: &Vec<u8>) {
        for c in v {
            unsafe {
                self.print_char(*c);
            }
        }
    }

    pub fn print_str(&self, s: &str) {
        for byte in s.bytes() {
            unsafe {
                self.print_char(byte);
            }
        }
    }

    pub fn print_u64(&self, mut n: u64) {
        if n == 0 {
            unsafe {
                self.print_char(b'0');
            }
            return;
        }
        let mut buf = [0u8; 20];
        let mut i = 0;
        while n > 0 {
            buf[i] = b'0' + (n % 10) as u8;
            n /= 10;
            i += 1;
        }
        while i > 0 {
            i -= 1;
            unsafe {
                self.print_char(buf[i]);
            }
        }
    }

    pub fn print_fmt(&mut self, fmt: core::fmt::Arguments) -> Result<(), fmt::Error> {
        core::fmt::write(self, fmt)
    }

    pub fn print_fmt_uninit(fmt: core::fmt::Arguments) -> Result<(), fmt::Error> {
        let mut console = unsafe { Self::new_unchecked(MMIO_CONSOLE) };
        core::fmt::write(&mut console, fmt)
    }
}

impl MmioDriver for ConsoleWriter {
    fn device_id(&self) -> u64 {
        0xC000000000000003
    }
    fn read_device_id(&self) -> u64 {
        let mmio: *mut u64 = (self.addr + 8) as *mut u64;
        unsafe { mmio.read_volatile() }
    }
}

impl core::fmt::Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.print_str(s);
        Ok(())
    }
}
