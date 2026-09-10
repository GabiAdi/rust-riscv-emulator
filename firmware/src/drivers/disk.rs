use crate::{
    MMIO_CONSOLE,
    drivers::{console::ConsoleWriter, mmio_driver::MmioDriver},
};
use alloc::vec::Vec;

const CMD_IDLE: u16 = 0;
const CMD_READ: u16 = 1;
const CMD_WRITE: u16 = 2;
const STATUS_IDLE: u16 = 0;
const STATUS_BUSY: u16 = 1;
const STATUS_ERROR: u16 = 2;

pub struct Disk {
    addr: u64,
    sector_size: u32,
}

enum DiskRegister {
    Sector(u64),
    NumSectors(u32),
    DiskCmd(u16),
    DiskStatus(u16),
    BufferAddr(u64),
}

enum DiskRegisterId {
    Sector,
    NumSectors,
    DiskCmd,
    DiskStatus,
    BufferAddr,
}

impl Disk {
    pub unsafe fn new_unchecked(addr: u64, sector_size: u32) -> Disk {
        Disk { addr, sector_size }
    }

    pub fn new(addr: u64, sector_size: u32) -> Option<Disk> {
        let mmio: *mut u64 = (addr + 0x18) as *mut u64;
        let device_id: u64 = unsafe { mmio.read_volatile() };

        if device_id != 0xA000000000000002 {
            return None;
        }

        Some(Disk { addr, sector_size })
    }

    pub fn read_sectors(&self, sector: u64, num_sectors: u32) -> Vec<u8> {
        let size = num_sectors as usize * self.sector_size as usize;
        let mut buf: Vec<u8> = Vec::new();
        buf.resize(size, 0);
        let addr = buf.as_mut_ptr() as u64;

        self.write_register(DiskRegister::Sector(sector));
        self.write_register(DiskRegister::NumSectors(num_sectors));
        self.write_register(DiskRegister::BufferAddr(addr));
        self.write_register(DiskRegister::DiskCmd(CMD_READ));

        loop {
            let DiskRegister::DiskStatus(status) = self.read_register(DiskRegisterId::DiskStatus)
            else {
                unreachable!()
            };

            if status == STATUS_IDLE {
                break;
            }

            core::hint::spin_loop();
        }

        buf
    }

    pub fn write_sectors(&self, sector: u64, num_sectors: u32, buf: Vec<u8>) {
        let size = num_sectors as usize * self.sector_size as usize;

        debug_assert_eq!(size, buf.len());

        let addr = buf.as_ptr() as u64;

        self.write_register(DiskRegister::Sector(sector));
        self.write_register(DiskRegister::NumSectors(num_sectors));
        self.write_register(DiskRegister::BufferAddr(addr));
        self.write_register(DiskRegister::DiskCmd(CMD_WRITE));

        loop {
            let DiskRegister::DiskStatus(status) = self.read_register(DiskRegisterId::DiskStatus)
            else {
                unreachable!()
            };

            if status == STATUS_IDLE {
                break;
            }

            core::hint::spin_loop();
        }
    }

    fn write_register(&self, register: DiskRegister) {
        // let mut console = unsafe { ConsoleWriter::new_unchecked(MMIO_CONSOLE) };
        match register {
            DiskRegister::Sector(sector) => {
                let mmio: *mut u64 = (self.addr) as *mut u64;
                unsafe {
                    mmio.write_volatile(sector);
                }
            }
            DiskRegister::NumSectors(num_sectors) => {
                let mmio: *mut u32 = (self.addr + 0x8) as *mut u32;
                unsafe {
                    mmio.write_volatile(num_sectors);
                }
            }
            DiskRegister::DiskCmd(disk_cmd) => {
                let mmio: *mut u16 = (self.addr + 0xC) as *mut u16;
                unsafe {
                    mmio.write_volatile(disk_cmd);
                }
            }
            DiskRegister::DiskStatus(disk_status) => {
                let mmio: *mut u16 = (self.addr + 0xE) as *mut u16;
                unsafe {
                    mmio.write_volatile(disk_status);
                }
            }
            DiskRegister::BufferAddr(buffer_addr) => {
                let mmio: *mut u64 = (self.addr + 0x10) as *mut u64;
                unsafe {
                    mmio.write_volatile(buffer_addr);
                }
            }
        }
    }

    fn read_register(&self, register: DiskRegisterId) -> DiskRegister {
        match register {
            DiskRegisterId::Sector => {
                let mmio: *mut u64 = (self.addr) as *mut u64;
                unsafe { DiskRegister::Sector(mmio.read_volatile()) }
            }
            DiskRegisterId::NumSectors => {
                let mmio: *mut u32 = (self.addr + 0x8) as *mut u32;
                unsafe { DiskRegister::NumSectors(mmio.read_volatile()) }
            }
            DiskRegisterId::DiskCmd => {
                let mmio: *mut u16 = (self.addr + 0xC) as *mut u16;
                unsafe { DiskRegister::DiskCmd(mmio.read_volatile()) }
            }
            DiskRegisterId::DiskStatus => {
                let mmio: *mut u16 = (self.addr + 0xE) as *mut u16;
                unsafe { DiskRegister::DiskStatus(mmio.read_volatile()) }
            }
            DiskRegisterId::BufferAddr => {
                let mmio: *mut u64 = (self.addr + 0x10) as *mut u64;
                unsafe { DiskRegister::BufferAddr(mmio.read_volatile()) }
            }
        }
    }
}

impl MmioDriver for Disk {
    fn device_id(&self) -> u64 {
        0xA000000000000002
    }
    fn read_device_id(&self) -> u64 {
        let mmio: *mut u64 = (self.addr + 0x18) as *mut u64;
        unsafe { mmio.read_volatile() }
    }
}
// 0x0  u64 sector number
// 0x8  u32 Number of sectors
// 0xC  u16 Disk cmd 0 = idle, 1 = read, 2 = write
// 0xE  u16 Disk status 0 = idle, 1 = busy, 2 = error
// 0x10 u64 Addr of DMA buffer
// 0x18 u64 Device identifier (0xA000000000000002)
