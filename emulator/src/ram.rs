use crate::memory_device::{DeviceError, DeviceType, MemoryDevice};

pub struct Ram {
    data: Vec<u8>,
    start_addr: u64,
    end_addr: u64,
    size: u64,
}

impl Ram {
    pub fn new(start_addr: u64, size: u64) -> Ram {
        Ram {
            data: vec![0u8; size as usize],
            start_addr,
            end_addr: start_addr + size,
            size,
        }
    }

    pub fn write_file(&mut self, addr: u64, file: Vec<u8>) {
        let size = file.len();
        for i in 0..(size as u64) {
            self.data[(i + addr - self.start_addr) as usize] = file[i as usize];
        }
    }

    pub fn write_bytes(&mut self, addr: u64, bytes: &[u8]) {
        let size = bytes.len();
        for i in 0..(size as u64) {
            self.data[(i + addr - self.start_addr) as usize] = bytes[i as usize];
        }
    }

    pub fn zero_fill(&mut self, addr: u64, size: u64) {
        for i in 0..size {
            self.write_byte(addr + i - self.start_addr, 0);
        }
    }
}

impl MemoryDevice for Ram {
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
        if addr as usize >= self.data.len() {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        return Ok(self.data[addr as usize] as u64);
    }

    fn read_hword(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr as usize + 1 >= self.data.len() {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        let offset = addr as usize;
        Ok(u16::from_le_bytes(self.data[offset..offset + 2].try_into().unwrap()) as u64)
    }

    fn read_word(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr as usize + 3 >= self.data.len() {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        let offset = addr as usize;
        Ok(u32::from_le_bytes(self.data[offset..offset + 4].try_into().unwrap()) as u64)
    }

    fn read_dword(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr as usize + 7 >= self.data.len() {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        let offset = addr as usize;
        Ok(u64::from_le_bytes(self.data[offset..offset + 8].try_into().unwrap()) as u64)
    }

    fn write_byte(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        if addr as usize >= self.data.len() {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        self.data[addr as usize] = value as u8;
        Ok(())
    }

    fn write_hword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        if addr as usize + 1 >= self.data.len() {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        let offset = addr as usize;
        let bytes = (value as u16).to_le_bytes();
        self.data[offset..offset + 2].copy_from_slice(&bytes);
        Ok(())
    }

    fn write_word(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        if addr as usize + 3 >= self.data.len() {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        let offset = addr as usize;
        let bytes = (value as u32).to_le_bytes();
        self.data[offset..offset + 4].copy_from_slice(&bytes);
        Ok(())
    }

    fn write_dword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        if addr as usize + 7 >= self.data.len() {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        let offset = addr as usize;
        let bytes = (value as u64).to_le_bytes();
        self.data[offset..offset + 8].copy_from_slice(&bytes);
        Ok(())
    }

    fn get_device_type(&self) -> DeviceType {
        DeviceType::Ram
    }

    fn clear(&mut self) {
        self.data.fill(0);
    }
}
