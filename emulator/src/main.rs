mod bus;
mod console;
mod cpu;
mod interrupt_device;
mod keyboard_device;
mod memory_device;
mod ram;
mod rom;
mod test_device;

use crossterm::terminal;
use goblin::elf::Elf;
use softfloat_wrapper::{ExceptionFlags, F32, F64, Float, RoundingMode};
use std::cell::RefCell;
use std::io::{Read, Write};
use std::panic;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use std::{fs, io};

use crate::console::Console;
use crate::cpu::CpuError;
use crate::cpu::Register;
use crate::interrupt_device::InterruptDevice;
use crate::keyboard_device::KeyboardDevice;
use crate::memory_device::DeviceError;
use crate::memory_device::MemoryDevice;
use crate::ram::Ram;
use crate::rom::Rom;
use crate::test_device::TestDevice;
use bus::Bus;
use cpu::{Cpu, Csr};

const RAM_SIZE: u64 = 1024 * 1024 * 128;
const RAM_BASE: u64 = 0x80000000;

struct LoadedProgram {
    entry_point: u64,
    tohost_addr: u64,
}

struct TestBinary {
    name: String,
    passed: bool,
    code: u64,
}

fn load_program(path: &str, ram: &mut Ram) -> LoadedProgram {
    let buffer = fs::read(path).expect(path);
    let elf = Elf::parse(&buffer).expect("Failed to parse ELF");

    for ph in &elf.program_headers {
        if ph.p_type == goblin::elf::program_header::PT_LOAD {
            let file_range = ph.p_offset as usize..(ph.p_offset + ph.p_filesz) as usize;
            ram.write_bytes(ph.p_vaddr, &buffer[file_range]);
            if ph.p_memsz > ph.p_filesz {
                ram.zero_fill(ph.p_vaddr + ph.p_filesz, ph.p_memsz - ph.p_filesz);
            }
        }
    }

    let tohost_addr = elf
        .syms
        .iter()
        .find_map(|sym| {
            let name = elf.strtab.get_at(sym.st_name)?;
            (name == "tohost").then_some(sym.st_value)
        })
        .expect("tohost symbol not found");

    LoadedProgram {
        entry_point: elf.entry,
        tohost_addr,
    }
}

fn get_test_binaries(dir: &str, prefix: &str) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .expect("failed to read directory")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| path.extension().is_none())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(prefix))
                .unwrap_or(false)
        })
        .collect()
}

fn handle_error(error: CpuError) {
    match error {
        CpuError::MemoryDeviceError {
            device_error,
            current_instruction,
            pc,
            registers,
        } => {
            match device_error {
                DeviceError::AddrOutOfRange { addr } => {
                    println!("AddrOutOfRange at {:#018x}", addr);
                }
                DeviceError::DeviceNotFound { addr } => {
                    println!("DeviceNotFound at {:#018x}", addr);
                }
            }
            println!("PC: {:#018x}", pc);
            println!("Current instruction: {:#010x}", current_instruction);
            for i in 0..32 {
                let value = registers[i];
                println!(
                    "x{}\t{}:\t{:#018x}\t{}",
                    i,
                    Cpu::get_register_name(i as usize),
                    value,
                    value as i64
                );
            }
        }
        CpuError::TimeOut => {
            println!("Cpu timed out at 10000 cycles");
        }
    }
}

fn test_cpu(prefix: &str) -> Result<bool, CpuError> {
    let test_path = "/home/user/rust-risc-v/riscv-tests/isa/";
    let test_binaries = get_test_binaries(test_path, prefix);
    // let test_binaries = get_test_binaries(test_path, "rv64ui-p-");

    let mut tests: Vec<TestBinary> = Vec::new();

    for binary in &test_binaries {
        let mut memory_devices: Vec<Box<dyn MemoryDevice>> = Vec::new();
        let interrupt_devices: Vec<Box<dyn InterruptDevice>> = Vec::new();
        let mut ram = Ram::new(RAM_BASE, RAM_SIZE);
        let file_name = binary.file_name().unwrap().to_str().unwrap();
        let file_path = String::from(test_path) + file_name;
        let loaded_program = load_program(&file_path, &mut ram);
        memory_devices.push(Box::new(ram));

        let cpu = Rc::new(RefCell::new(Cpu::new()));
        let bus = Rc::new(RefCell::new(Bus::new(
            memory_devices,
            interrupt_devices,
            cpu.borrow().get_shared_mip(),
        )));

        cpu.borrow_mut().attach_bus(&bus);
        bus.borrow_mut().attach_cpu(&cpu);

        cpu.borrow_mut().set_pc(loaded_program.entry_point);
        bus.borrow_mut().tohost_addr = Some(loaded_program.tohost_addr);

        println!("Trying test: {}", file_name);

        // cpu.borrow_mut().print_instruction = true;

        match cpu.borrow_mut().run_until_halt() {
            Ok(()) => {}
            Err(error) => return Err(error),
        };

        if cpu.borrow().get_reg(Register::gp as u8) == 1 {
            tests.push(TestBinary {
                name: file_name.to_string(),
                passed: true,
                code: 1,
            });
        } else {
            tests.push(TestBinary {
                name: file_name.to_string(),
                passed: false,
                code: cpu.borrow().get_reg(Register::gp as u8),
            })
        }
    }

    for test in tests {
        if !test.passed {
            println!(
                "Test: {} failed with gp: {}, code: {}",
                test.name,
                test.code,
                test.code >> 1,
            );
            return Ok(false);
        }
    }
    Ok(true)
}

fn spawn_std_reader() -> Receiver<u8> {
    let (tx, rx): (Sender<u8>, Receiver<u8>) = mpsc::channel();

    thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 1];
        let mut last_byte: Option<u8> = None;
        let mut last_time = Instant::now();

        const DEBOUNCE: Duration = Duration::from_millis(150);

        loop {
            match stdin.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => {
                    let now = Instant::now();
                    let is_repeat =
                        last_byte == Some(buf[0]) && now.duration_since(last_time) < DEBOUNCE;
                    last_byte = Some(buf[0]);
                    last_time == now;

                    if !is_repeat && tx.send(buf[0]).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    rx
}

fn main() {
    // terminal::enable_raw_mode().unwrap();
    // test_cpu("rv64ui-p-ma_data");

    // let mut cpu = Cpu::new();
    // for word in 0u32..0x10000 {
    //     if word & 0b11 == 0b11 {
    //         continue;
    //     }
    //     let decoded = cpu.decode(word as u64);
    //     println!("{:04x} {:?}", word, decoded);
    // }
    //
    // return;

    let mut memory_devices: Vec<Box<dyn MemoryDevice>> = Vec::new();
    let mut interrupt_devices: Vec<Box<dyn InterruptDevice>> = Vec::new();

    // let data = fs::read("/home/user/rust-risc-v/assembly/testc.bin").unwrap();
    let data = fs::read("/home/user/rust-risc-v/assembly/firmware.bin").unwrap();

    // let rom = Rom::new(0, data);

    let cpu = Rc::new(RefCell::new(Cpu::new()));

    let mut ram = Ram::new(0, 0x5000000);
    ram.write_file(0, data);
    let console = Console::new(0xFFFFFFFFFFFFFF00);
    // devices.push(Box::new(rom));
    memory_devices.push(Box::new(ram));
    memory_devices.push(Box::new(console));

    let keyboard_device = Rc::new(RefCell::new(KeyboardDevice::new(0xFFFFFFFFFFFFFF08)));
    memory_devices.push(Box::new(keyboard_device.clone()));
    interrupt_devices.push(Box::new(keyboard_device.clone()));

    let bus = Rc::new(RefCell::new(Bus::new(
        memory_devices,
        interrupt_devices,
        cpu.borrow().get_shared_mip(),
    )));

    cpu.borrow_mut().attach_bus(&bus);
    bus.borrow_mut().attach_cpu(&cpu);

    // cpu.borrow_mut().run_until_halt();

    // let stdin_rx = spawn_std_reader();
    // let mut running = true;
    //
    // while running {
    //     while let Ok(byte) = stdin_rx.try_recv() {
    //         if byte == 0x3 {
    //             running = false;
    //             break;
    //         }
    //         keyboard_device.borrow_mut().push_key(byte);
    //     }
    //
    //     match cpu.borrow_mut().step() {
    //         Ok(()) => {}
    //         Err(error) => {
    //             terminal::disable_raw_mode().unwrap();
    //             handle_error(error);
    //             break;
    //         }
    //     };
    //
    //     if cpu.borrow().halted {
    //         break;
    //     }
    // }

    loop {
        print!(">> ");
        io::stdout().flush().unwrap();

        let mut buffer = String::new();
        let stdin = io::stdin();
        stdin.read_line(&mut buffer);
        let cmd = buffer.trim();

        if cmd == "s" {
            println!("Stepping...");
            cpu.borrow_mut().step();
        } else if cmd == "r" {
            println!("Running...");
            while !cpu.borrow().halted {
                cpu.borrow_mut().step();
            }
            // cpu.borrow_mut().run_until_halt();
            cpu.borrow_mut().halted = false;
        } else if cmd == "reg" {
            for i in 0..32 {
                let value = cpu.borrow().get_reg(i);
                println!(
                    "x{}\t{}:\t{:#018x}\t{}",
                    i,
                    Cpu::get_register_name(i as usize),
                    value,
                    value as i64
                );
            }
        } else if cmd == "pins" {
            println!("{:#010x}", cpu.borrow().get_ins())
        } else if cmd == "pc" {
            println!("{:#018x}", cpu.borrow().get_pc())
        } else if cmd == "ins" {
            println!(
                "{:#010x}",
                bus.borrow_mut().read_word(cpu.borrow().get_pc()).unwrap()
            );
        } else if cmd == "h?" {
            println!(
                "State: {}",
                if cpu.borrow().halted {
                    "halted"
                } else {
                    "unhalted"
                }
            );
        } else if cmd == "h" {
            cpu.borrow_mut().halted = true;
        } else if cmd == "uh" {
            cpu.borrow_mut().halted = false;
        } else if cmd == "q" {
            break;
        }
    }

    // terminal::disable_raw_mode().unwrap();

    // println!();

    // cpu.borrow_mut().print_instruction = true;
    // cpu.borrow_mut().set_pc(2);

    // let instant = Instant::now();

    // match cpu.borrow_mut().run_until_halt() {
    //     Ok(()) => {}
    //     Err(error) => handle_error(error),
    // };

    // let duration = instant.elapsed();
    //
    // println!("Time: {:?}", duration);

    // println!("PC: {}", cpu.borrow().get_pc());
    // for i in 0..32 {
    //     let value = cpu.borrow().get_reg(i);
    //     println!(
    //         "x{}\t{}:\t{:#018x}\t{}",
    //         i,
    //         Cpu::get_register_name(i as usize),
    //         value,
    //         value as i64
    //     );
    // }

    // println!();
    //
    // for i in 0..32 {
    //     let value = cpu.borrow().get_f64_reg(i);
    //     println!("f{}:\t{:#018x}\t{}", i, value.to_bits(), value);
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extensions() {
        const EXTENSIONS: [&str; 6] = [
            "rv64ui-p-",
            "rv64um-p-",
            "rv64ua-p-",
            "rv64uc-p-",
            "rv64uf-p-",
            "rv64ud-p-",
        ];

        let mut passed_all = true;

        for extension in EXTENSIONS {
            match test_cpu(extension) {
                Ok(passed) => {
                    if !passed {
                        passed_all = false;
                    }
                }
                Err(error) => {
                    handle_error(error);
                    assert!(false)
                }
            }
        }

        assert!(passed_all);
    }
}
