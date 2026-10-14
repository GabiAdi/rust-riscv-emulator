use std::{
    cell::RefCell,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    rc::{Rc, Weak},
};

use crate::{
    bus::Bus,
    dma_device::DmaDevice,
    memory_device::{DeviceError, DeviceType, MemoryDevice},
};

pub const SECTOR_SIZE: u64 = 512;

pub struct Disk {
    start_addr: u64,
    end_addr: u64,
    size: u64,

    data: Vec<u8>,

    device_id: u64,    // 0x0 Device identifier (0xA000000000000002)
    sector: u64,       // 0x8 sector number
    sector_count: u32, // 0x10 Number of sectors
    disk_cmd: u16,     // 0x14 0 = idle, 1 = read, 2 = write
    disk_status: u16,  // 0x16 0 = idle, 1 = busy, 2 = error
    disk_bufptr: u64,  // 0x18 Addr of DMA buffer
    bus: Option<Weak<Bus>>,

    file: File,
}

impl Disk {
    pub fn new(start_addr: u64, path: &str) -> std::io::Result<Disk> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;

        let mut disk = Disk {
            start_addr,
            end_addr: start_addr + 0x20,
            size: 0x20,

            data: vec![0u8; 0x20],

            // bits 0 - 60 identifier
            // bit 61 - DMA device
            // bit 62 - interrupt device
            // bit 63 - memory device (always set)
            device_id: 0xA000000000000002,

            sector: 0,
            sector_count: 0,
            disk_cmd: 0,
            disk_status: 0,
            disk_bufptr: 0,
            bus: None,

            file,
        };

        disk.data[0x00..0x08].copy_from_slice(&disk.device_id.to_le_bytes());

        Ok(disk)
    }

    fn operation(&mut self) {
        if self.disk_status == 2 {
            // ERROR (wait for error to be cleared)
            return;
        }
        match self.disk_cmd {
            0 => {
                // IDLE
                self.disk_status = 0; // IDLE
            }
            1 => {
                // READ disk into memory
                self.disk_status = 1;
                let size: usize = self.sector_count as usize * SECTOR_SIZE as usize;
                let mut buf: Vec<u8> = vec![0u8; size];
                match self.read_sectors(self.sector, self.sector_count, &mut buf) {
                    Ok(()) => {}
                    Err(error) => {
                        println!("{:?}", error);
                        self.disk_status = 2;
                        self.update_data();
                        return;
                    }
                }
                match self.bus().dma_write(self.disk_bufptr, &buf) {
                    Ok(()) => {}
                    Err(error) => {
                        println!("{:?}", error);
                        self.disk_status = 2;
                        self.update_data();
                        return;
                    }
                }
                self.disk_status = 0; // IDLE
            }
            2 => {
                // WRITE memory into disk
                self.disk_status = 1;
                let size: usize = self.sector_count as usize * SECTOR_SIZE as usize;
                let mut buf: Vec<u8> = vec![0u8; size];
                match self.bus().dma_read(self.disk_bufptr, &mut buf) {
                    Ok(()) => {}
                    Err(error) => {
                        println!("{:?}", error);
                        self.disk_status = 2;
                        self.update_data();
                        return;
                    }
                }
                match self.write_sectors(self.sector, self.sector_count, &buf) {
                    Ok(()) => {}
                    Err(error) => {
                        println!("{:?}", error);
                        self.disk_status = 2;
                        self.update_data();
                        return;
                    }
                }
                self.disk_status = 0; // IDLE
            }
            _ => unreachable!(),
        }
        self.disk_cmd = 0; // IDLE
        self.update_data();
    }

    fn update_regs(&mut self) {
        self.sector = u64::from_le_bytes(self.data[0x08..0x10].try_into().unwrap());
        self.sector_count = u32::from_le_bytes(self.data[0x10..0x14].try_into().unwrap());
        self.disk_cmd = u16::from_le_bytes(self.data[0x14..0x16].try_into().unwrap());
        self.disk_status = u16::from_le_bytes(self.data[0x16..0x18].try_into().unwrap());
        self.disk_bufptr = u64::from_le_bytes(self.data[0x18..0x20].try_into().unwrap());
    }

    fn update_data(&mut self) {
        self.data[0x08..0x10].copy_from_slice(&self.sector.to_le_bytes());
        self.data[0x10..0x14].copy_from_slice(&self.sector_count.to_le_bytes());
        self.data[0x14..0x16].copy_from_slice(&self.disk_cmd.to_le_bytes());
        self.data[0x16..0x18].copy_from_slice(&self.disk_status.to_le_bytes());
        self.data[0x18..0x20].copy_from_slice(&self.disk_bufptr.to_le_bytes());
    }

    fn read_sectors(
        &mut self,
        start_sector: u64,
        sector_count: u32,
        buf: &mut [u8],
    ) -> std::io::Result<()> {
        debug_assert_eq!(buf.len(), sector_count as usize * SECTOR_SIZE as usize);
        self.file
            .seek(SeekFrom::Start(start_sector * SECTOR_SIZE))?;
        self.file.read_exact(buf)?;
        Ok(())
    }

    fn write_sectors(
        &mut self,
        start_sector: u64,
        sector_count: u32,
        buf: &[u8],
    ) -> std::io::Result<()> {
        debug_assert_eq!(buf.len(), sector_count as usize * SECTOR_SIZE as usize);
        self.file
            .seek(SeekFrom::Start(start_sector * SECTOR_SIZE))?;
        self.file.write_all(buf)?;
        self.file.sync_data()?;
        Ok(())
    }
}

impl MemoryDevice for Disk {
    fn get_size(&self) -> u64 {
        self.size
    }
    fn get_start_addr(&self) -> u64 {
        self.start_addr
    }
    fn get_end_addr(&self) -> u64 {
        self.end_addr
    }
    fn read_byte(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr >= self.size {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        Ok(self.data[addr as usize] as u64)
    }

    fn read_hword(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr >= self.size + 1 {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        let offset = addr as usize;
        Ok(u16::from_le_bytes(self.data[offset..offset + 2].try_into().unwrap()) as u64)
    }

    fn read_word(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr >= self.size + 3 {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        let offset = addr as usize;
        Ok(u32::from_le_bytes(self.data[offset..offset + 4].try_into().unwrap()) as u64)
    }

    fn read_dword(&self, addr: u64) -> Result<u64, DeviceError> {
        if addr >= self.size + 7 {
            return Err(DeviceError::AddrOutOfRange { addr });
        }
        let offset = addr as usize;
        Ok(u64::from_le_bytes(self.data[offset..offset + 8].try_into().unwrap()) as u64)
    }

    fn write_byte(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        if addr >= self.size {
            return Err(DeviceError::AddrOutOfRange { addr });
        }

        let offset = addr as usize;
        self.data[offset] = value as u8;
        self.update_regs();
        self.operation();

        Ok(())
    }

    fn write_hword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        if addr + 1 >= self.size {
            return Err(DeviceError::AddrOutOfRange { addr });
        }

        let offset = addr as usize;
        let bytes = (value as u16).to_le_bytes();
        self.data[offset..offset + 2].copy_from_slice(&bytes);
        self.update_regs();
        self.operation();

        Ok(())
    }

    fn write_word(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        if addr + 3 >= self.size {
            return Err(DeviceError::AddrOutOfRange { addr });
        }

        let offset = addr as usize;
        let bytes = (value as u32).to_le_bytes();
        self.data[offset..offset + 4].copy_from_slice(&bytes);
        self.update_regs();
        self.operation();

        Ok(())
    }

    fn write_dword(&mut self, addr: u64, value: u64) -> Result<(), DeviceError> {
        if addr + 7 >= self.size {
            return Err(DeviceError::AddrOutOfRange { addr });
        }

        let offset = addr as usize;
        let bytes = (value as u64).to_le_bytes();
        self.data[offset..offset + 8].copy_from_slice(&bytes);
        self.update_regs();
        self.operation();

        Ok(())
    }

    fn get_device_type(&self) -> DeviceType {
        DeviceType::Disk
    }

    fn clear(&mut self) {
        self.data.fill(0);
        self.update_regs();
    }
}

impl DmaDevice for Disk {
    fn attach_bus(&mut self, bus: Weak<Bus>) {
        self.bus = Some(bus);
    }
    // fn bus_ref(&self) -> &Weak<Bus> {
    //     match &self.bus {
    //         Some(b) => b,
    //         None => panic!("No bus"),
    //     }
    // }
    fn bus(&self) -> Rc<Bus> {
        match &self.bus {
            Some(b) => b.upgrade().expect("Bus dropped before device"),
            None => panic!("No bus"),
        }
    }
}
