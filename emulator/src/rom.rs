use crate::memory_device::{DeviceError, DeviceType, MemoryDevice};

pub struct Rom {
    data: Vec<u8>,
    start_addr: u64,
    end_addr: u64,
    size: u64,
}

impl Rom {
    pub fn new(start_addr: u64, file: Vec<u8>) -> Rom {
        let size = file.len().clone() as u64;
        Rom {
            data: file,
            start_addr,
            end_addr: start_addr + size,
            size: size,
        }
    }
}

impl MemoryDevice for Rom {
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
        if addr >= self.data.len() as u64 {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        Ok(self.data[addr as usize] as u64)
    }

    fn read_hword(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr as usize >= self.data.len() {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        let offset = addr as usize;
        Ok(u16::from_le_bytes(self.data[offset..offset + 2].try_into().unwrap()) as u64)
    }

    fn read_word(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr as usize >= self.data.len() {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        let offset = addr as usize;
        Ok(u32::from_le_bytes(self.data[offset..offset + 4].try_into().unwrap()) as u64)
    }

    fn read_dword(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr as usize >= self.data.len() {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        let offset = addr as usize;
        Ok(u64::from_le_bytes(self.data[offset..offset + 8].try_into().unwrap()) as u64)
    }

    fn write_byte(&mut self, _addr: u64, _value: u64) -> Result<(), DeviceError> {
        Ok(())
    }

    fn write_hword(&mut self, _addr: u64, _value: u64) -> Result<(), DeviceError> {
        Ok(())
    }

    fn write_word(&mut self, _addr: u64, _value: u64) -> Result<(), DeviceError> {
        Ok(())
    }

    fn write_dword(&mut self, _addr: u64, _value: u64) -> Result<(), DeviceError> {
        Ok(())
    }

    fn get_device_type(&self) -> DeviceType {
        DeviceType::Rom
    }

    fn clear(&mut self) {}
}
