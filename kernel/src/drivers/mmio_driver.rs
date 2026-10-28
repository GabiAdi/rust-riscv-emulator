pub trait MmioDriver {
    fn device_id(&self) -> u64;
    fn read_device_id(&self) -> u64;
}
