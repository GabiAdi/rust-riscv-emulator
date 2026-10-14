use std::io::{self, Write};

use crate::memory_device::{DeviceError, DeviceType, MemoryDevice};

pub struct Console {
    character: Option<char>,
    start_addr: u64,
    end_addr: u64,
    size: u64,
    device_id: u64,
}

impl Console {
    pub fn new(start_addr: u64) -> Console {
        Console {
            character: None,
            start_addr,
            end_addr: start_addr + 0x10,
            size: 0x10,
            device_id: 0xC000000000000003,
        }
    }
}

impl MemoryDevice for Console {
    fn get_start_addr(&self) -> u64 {
        self.start_addr
    }

    fn get_end_addr(&self) -> u64 {
        self.end_addr
    }

    fn get_size(&self) -> u64 {
        self.size
    }

    fn read_byte(&self, addr: u64) -> Result<u64, DeviceError> {
        Ok(match self.character {
            Some(character) => (character as u8) as u64,
            None => 0,
        })
    }

    fn read_hword(&self, addr: u64) -> Result<u64, DeviceError> {
        Ok(match self.character {
            Some(character) => (character as u16) as u64,
            None => 0,
        })
    }

    fn read_word(&self, addr: u64) -> Result<u64, DeviceError> {
        Ok(match self.character {
            Some(character) => (character as u32) as u64,
            None => 0,
        })
    }

    fn read_dword(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr == 0x0 {
            return Ok(self.device_id);
        }
        Ok(match self.character {
            Some(character) => character as u64,
            None => 0,
        })
    }

    fn write_byte(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        self.character = Some(value as u8 as char);
        match value as u8 {
            b'\n' => print!("\r\n"),
            b'\r' => print!("\r\n"),
            c => print!("{}", c as char),
        }
        io::stdout().flush().unwrap();
        Ok(())
    }

    fn write_hword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        self.character = Some(value as u8 as char);
        match value as u8 {
            b'\n' => print!("\r\n"),
            b'\r' => print!("\r\n"),
            c => print!("{}", c as char),
        }
        io::stdout().flush().unwrap();
        Ok(())
    }

    fn write_word(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        self.character = Some(value as u8 as char);
        match value as u8 {
            b'\n' => print!("\r\n"),
            b'\r' => print!("\r\n"),
            c => print!("{}", c as char),
        }
        io::stdout().flush().unwrap();
        Ok(())
    }

    fn write_dword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        self.character = Some(value as u8 as char);
        match value as u8 {
            b'\n' => print!("\r\n"),
            b'\r' => print!("\r\n"),
            c => print!("{}", c as char),
        }
        io::stdout().flush().unwrap();
        Ok(())
    }

    fn get_device_type(&self) -> DeviceType {
        DeviceType::Console
    }

    fn clear(&mut self) {
        self.character = None
    }
}
