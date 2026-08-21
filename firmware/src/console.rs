pub const MMIO_CONSOLE: *mut u8 = 0xFFFFFFFFFFFFFF00 as *mut u8;
pub const MMIO_KEYBOARD: *mut u8 = 0xFFFFFFFFFFFFFF08 as *mut u8;

pub struct ConsoleWriter;

impl core::fmt::Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        print_str(s);
        Ok(())
    }
}

pub fn print_char(c: u8) {
    unsafe {
        MMIO_CONSOLE.write_volatile(c);
    }
}

pub fn print_str(s: &str) {
    for byte in s.bytes() {
        print_char(byte);
    }
}

pub fn print_u64(mut n: u64) {
    if n == 0 {
        print_char(b'0');
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
        print_char(buf[i]);
    }
}
