use crate::bus::Bus;
use crate::cpu::FloatConversion::{WordSigned, WordUnsigned};
use crate::cpu::MemoryWidth::{Dword, Word};
use crate::memory_device::DeviceError;
use core::panic;
use softfloat_wrapper::{ExceptionFlags, F32, F64, Float, RoundingMode};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Cpu {
    pub print_instruction: bool,
    pub halted: bool,
    pub halt_on_ebreak: bool,
    bus: Option<Rc<RefCell<Bus>>>,
    registers: [u64; 32],
    float_registers: [u64; 32],
    csrs: Csrs,
    reservation: Option<u64>,
    pc: u64,
    instruction: u64,
    priv_mode: u8,
    compressed: bool,
    jump: bool,
}

pub enum CpuError {
    MemoryDeviceError {
        device_error: DeviceError,
        current_instruction: u64,
        pc: u64,
        registers: [u64; 32],
    },
    TimeOut,
}

const REGISTER_NAME: [&str; 32] = [
    "zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2", "s0", "s1", "a0", "a1", "a2", "a3", "a4",
    "a5", "a6", "a7", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11", "t3", "t4",
    "t5", "t6",
];

pub enum Register {
    zero = 0,
    ra = 1,
    sp = 2,
    gp = 3,
    tp = 4,
    t0 = 5,
    t1 = 6,
    t2 = 7,
    s0 = 8,
    s1 = 9,
    a0 = 10,
    a1 = 11,
    a2 = 12,
    a3 = 13,
    a4 = 14,
    a5 = 15,
    a6 = 16,
    a7 = 17,
    s2 = 18,
    s3 = 19,
    s4 = 20,
    s5 = 21,
    s6 = 22,
    s7 = 23,
    s8 = 24,
    s9 = 25,
    s10 = 26,
    s11 = 27,
    t3 = 28,
    t4 = 29,
    t5 = 30,
    t6 = 31,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MemoryWidth {
    Byte = 1,
    Hword = 2,
    Word = 4,
    Dword = 8,
}

#[repr(u64)]
#[derive(Clone, Copy)]
pub enum Exception {
    InstructionAddrMisaligned = 0,
    InstructionAccessFault = 1,
    IllegalInstruction = 2,
    Breakpoint = 3,
    LoadAddrMisaligned = 4,
    LoadAccessFault = 5,
    StoreAddrMisaligned = 6,
    StoreAccessFault = 7,
    EcallFromU = 8,
    EcallFromS = 9,
    EcallFromM = 11,
    InstructionPageFault = 12,
    LoadPageFault = 13,
    StorePageFault = 15,
}

#[repr(u64)]
#[derive(Clone, Copy)]
pub enum Interrupt {
    SupervisorSoftware = 1,
    MachineSoftware = 3,
    SuperVisorTimer = 5,
    MachineTimer = 7,
    SupervisorExternal = 9,
    MachineExternal = 11,
}

pub struct Trap {
    pub cause: u64,
    pub is_interrupt: bool,
    pub tval: u64,
}

enum AtomicOperation {
    Swap,
    Add,
    Xor,
    And,
    Or,
    Min,
    Max,
    Minu,
    Maxu,
}

enum CsrOperation {
    Swap,
    Set,
    Clear,
}

#[derive(PartialEq, Eq)]
enum FloatConversion {
    WordSigned,
    WordUnsigned,
    DwordSigned,
    DwordUnsigned,
    Single,
    Double,
}

enum FloatOperation {
    Fmadd,
    Fmsub,
    Fnmsub,
    Fnmadd,
    Fadd,
    Fsub,
    Fmul,
    Fdiv,
    Fsqrt,
    Fsgnj,
    Fsgnjn,
    Fsgnjx,
    Fmin,
    Fmax,
    Feq,
    Flt,
    Fle,
    Fclass,
    Fcvt {
        from: FloatConversion,
        to: FloatConversion,
    },
    Fmv {
        from: FloatConversion,
        to: FloatConversion,
    },
}

#[derive(Debug)]
pub enum DecodedInstruction {
    R {
        opcode: u8,
        rd: u8,
        funct3: u8,
        rs1: u8,
        rs2: u8,
        funct7: u8,
    },
    R4 {
        opcode: u8,
        rd: u8,
        rm: u8,
        rs1: u8,
        rs2: u8,
        funct2: u8,
        rs3: u8,
    },
    I {
        opcode: u8,
        rd: u8,
        funct3: u8,
        rs1: u8,
        imm: i32,
    },
    S {
        opcode: u8,
        funct3: u8,
        rs1: u8,
        rs2: u8,
        imm: i32,
    },
    B {
        opcode: u8,
        funct3: u8,
        rs1: u8,
        rs2: u8,
        imm: i32,
    },
    U {
        opcode: u8,
        rd: u8,
        imm: i32,
    },
    J {
        opcode: u8,
        rd: u8,
        imm: i32,
    },
    Atomic {
        opcode: u8,
        rd: u8,
        funct3: u8,
        rs1: u8,
        rs2: u8,
        rl: u8,
        aq: u8,
        funct5: u8,
    },
    Noop,
    Unkown,
}

enum InstructionResult {
    WriteToReg {
        rd: u8,
        result: u64,
    },
    JumpAndLink {
        rd: u8,
        result: i64,
    },
    JumpAndLinkReg {
        rd: u8,
        result: u64,
    },
    WriteMemToReg {
        rd: u8,
        addr: u64,
        width: MemoryWidth,
        extension: Extension,
    },
    WriteToMem {
        addr: u64,
        result: u64,
        width: MemoryWidth,
    },
    WriteMemToFloatReg {
        rd: u8,
        addr: u64,
        width: MemoryWidth,
    },
    Branch {
        result: i64,
    },
    AtomicLoad {
        rd: u8,
        addr: u64,
        width: MemoryWidth,
    },
    AtomicStore {
        rd: u8,
        addr: u64,
        result: u64,
        width: MemoryWidth,
    },
    AtomicOp {
        rd: u8,
        addr: u64,
        result: u64,
        width: MemoryWidth,
        atomic_op: AtomicOperation,
    },
    CsrOp {
        rd: u8,
        csr: u16,
        result: u64,
        csr_op: CsrOperation,
    },
    FloatOp {
        rd: u8,
        rs1: u8,
        rs2: u8,
        rs3: Option<u8>,
        rm: u8,
        float_op: FloatOperation,
        width: MemoryWidth,
    },
    Fence,
    Ecall,
    Ebreak,
    Mret,
    Noop,
    Unkown,
}

enum Extension {
    ZeroExtend,
    SignExtend,
    NoExtend,
}

#[repr(u16)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Csr {
    Fflags = 0x001,
    Frm = 0x002,
    Fcsr = 0x003,
    Satp = 0x180,
    Mstatus = 0x300,
    Misa = 0x301,
    Medeleg = 0x302,
    Mideleg = 0x303,
    Mie = 0x304,
    Mtvec = 0x305,
    Mscratch = 0x340,
    Mepc = 0x341,
    Mcause = 0x342,
    Mtval = 0x343,
    Mip = 0x344,
    Pmpcfg0 = 0x3A0,
    Pmpaddr0 = 0x3B0,
    Mnstatus = 0x744,
    Mcycle = 0xB00,
    Mvendorid = 0xF11,
    Marchid = 0xF12,
    Mimpid = 0xF13,
    Mhartid = 0xF14,
}

const FORCE_READ_ONLY: &[u16] = &[Csr::Mip as u16, Csr::Mcycle as u16];

pub struct Csrs {
    regs: [u64; 4096],
    pub mip_shared: Rc<Cell<u64>>,
}

impl Csrs {
    pub fn new(mip_shared: Rc<Cell<u64>>) -> Csrs {
        Csrs {
            regs: [0; 4096],
            mip_shared,
        }
    }

    pub fn sync_mip(&mut self) {
        self.regs[Csr::Mip as usize] = self.mip_shared.get();
    }

    fn is_read_only(csr: u16) -> bool {
        (csr >> 10) & 0b11 == 0b11 || FORCE_READ_ONLY.contains(&csr)
    }

    pub fn get_csr(&self, csr: u16) -> u64 {
        if csr as usize > self.regs.len() {
            return 0;
        }
        match csr {
            x if x == Csr::Fflags as u16 => self.regs[Csr::Fcsr as usize] & 0x1F,
            x if x == Csr::Frm as u16 => (self.regs[Csr::Fcsr as usize] >> 5) & 0x7,
            _ => self.regs[csr as usize],
        }
    }

    pub fn set_csr(&mut self, csr: u16, value: u64) {
        if Self::is_read_only(csr) {
            return;
        }

        // println!("Wrote to csr: {:#05x}", csr);

        match csr {
            x if x == Csr::Fflags as u16 => {
                let fcsr = self.regs[Csr::Fcsr as usize];
                self.regs[Csr::Fcsr as usize] = (fcsr & !0x1F) | (value & 0x1F);
            }
            x if x == Csr::Frm as u16 => {
                let fcsr = self.regs[Csr::Fcsr as usize];
                self.regs[Csr::Fcsr as usize] = (fcsr & !0xE0) | ((value & 0x7) << 5);
            }
            x if x == Csr::Fcsr as u16 => {
                self.regs[Csr::Fcsr as usize] = value & 0xFF;
            }
            _ => self.regs[csr as usize] = value,
        }
    }

    pub fn read(&self, csr: Csr) -> u64 {
        self.get_csr(csr as u16)
    }

    pub fn write(&mut self, csr: Csr, value: u64) {
        self.set_csr(csr as u16, value);
    }
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            print_instruction: false,
            halted: false,
            halt_on_ebreak: true,
            bus: None,
            registers: [0; 32],
            float_registers: [0; 32],
            csrs: Csrs::new(Rc::new(Cell::new(0))),
            reservation: None,
            pc: 0,
            instruction: 0,
            priv_mode: 3,
            jump: false,
            compressed: false,
        }
    }

    pub fn get_shared_mip(&self) -> Rc<Cell<u64>> {
        self.csrs.mip_shared.clone()
    }

    pub fn get_register_name(val: usize) -> String {
        match REGISTER_NAME.get(val).copied() {
            Some(s) => s.to_string(),
            None => "".to_string(),
        }
    }

    pub fn attach_bus(&mut self, bus: &Rc<RefCell<Bus>>) {
        self.bus = Some(Rc::clone(bus));
    }

    pub fn halt(&mut self) {
        self.halted = true;
    }

    pub fn unhalt(&mut self) {
        self.halted = false;
    }

    pub fn run_until_halt(&mut self) -> Result<(), CpuError> {
        while !self.halted {
            match self.step() {
                Ok(()) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub fn step(&mut self) -> Result<(), CpuError> {
        // Pre-step
        self.csrs.sync_mip();

        if let Some(bus) = &self.bus {
            bus.borrow_mut().tick_interrupts();
        }

        if self.halted {
            return Ok(());
        }

        if let Some(trap) = self.check_interrupts() {
            self.take_trap(trap);

            self.csrs.regs[Csr::Mcycle as usize] += 1;
            return Ok(());
        }

        if let Some(bus) = &self.bus {
            if bus.borrow().halt_cpu {
                self.halted = true;
                return Ok(());
            }

            let lo = match bus.borrow().read_hword(self.pc) {
                Ok(h) => h as u64,
                Err(error) => {
                    return Err(CpuError::MemoryDeviceError {
                        device_error: error,
                        current_instruction: self.instruction,
                        pc: self.pc,
                        registers: self.registers,
                    });
                }
            };

            self.instruction = if lo & 0b11 == 0b11 {
                let hi = match bus.borrow().read_hword(self.pc.wrapping_add(2)) {
                    Ok(h) => h as u64,
                    Err(error) => {
                        return Err(CpuError::MemoryDeviceError {
                            device_error: error,
                            current_instruction: self.instruction,
                            pc: self.pc,
                            registers: self.registers,
                        });
                    }
                };
                (hi << 16) | lo
            } else {
                lo
            };

            // self.instruction = match bus.borrow().read_word(self.pc) {
            //     Ok(word) => word,
            //     Err(error) => {
            //         return Err(CpuError::MemoryDeviceError {
            //             device_error: error,
            //             current_instruction: self.instruction,
            //             pc: self.pc,
            //             registers: self.registers,
            //         });
            //     }
            // };

            let decoded_instruction = self.decode(self.instruction);
            let instruction_result = self.execute(decoded_instruction);

            if self.print_instruction {
                match instruction_result {
                    InstructionResult::Unkown => {
                        println!(
                            "Unkown:\t{:#010x}\tat:\t{:#018x}",
                            self.instruction,
                            self.get_pc()
                        );
                    }
                    _ => {
                        // println!(
                        //     "Decoded:\t{:#010x}\tat\t{:#018x}",
                        //     self.instruction,
                        //     self.get_pc()
                        // );
                    }
                }
            }

            match self.writeback(instruction_result) {
                Ok(trap_option) => match trap_option {
                    // Some(trap) => {}
                    Some(trap) => self.take_trap(trap),
                    None => {}
                },
                Err(error) => {
                    return Err(CpuError::MemoryDeviceError {
                        device_error: error,
                        current_instruction: self.instruction,
                        pc: self.pc,
                        registers: self.registers,
                    });
                }
            };

            if !self.jump {
                if self.compressed {
                    self.pc = self.pc.wrapping_add(2);
                    self.compressed = false;
                } else {
                    self.pc = self.pc.wrapping_add(4);
                }
            } else {
                self.compressed = false;
                self.jump = false;
            }

            self.csrs.regs[Csr::Mcycle as usize] += 1;

            // if self.csrs.read(Csr::Mcycle) == 10000 {
            //     self.halted = true;
            //     return Err(CpuError::TimeOut);
            // }
        }
        Ok(())
    }

    pub fn set_pc(&mut self, addr: u64) {
        self.pc = addr
    }

    pub fn get_ins(&self) -> u64 {
        self.instruction
    }

    pub fn get_pc(&self) -> u64 {
        self.pc
    }

    pub fn get_csr(&self, csr: u16) -> Option<u64> {
        if csr as usize > self.csrs.regs.len() {
            return None;
        }
        Some(self.csrs.get_csr(csr))
    }

    fn check_interrupts(&mut self) -> Option<Trap> {
        let status = self.csrs.read(Csr::Mstatus);
        let global_enable = (status >> 3) & 1; // MIE
        if global_enable == 0 {
            return None;
        }

        let pending = self.csrs.read(Csr::Mip) & self.csrs.read(Csr::Mie);
        // if pending != 0 {
        //     eprint!(
        //         "check_interrupts: mstatus={:#x} MIE={}",
        //         status, global_enable
        //     );
        // }
        if pending == 0 {
            return None;
        }

        const PRIORITY: [u32; 6] = [11, 3, 7, 9, 1, 5]; // MEI, MSI, MTI, SEI, SSI, STI
        for bit in PRIORITY {
            if pending & (1 << bit) != 0 {
                return Some(Trap {
                    cause: bit as u64,
                    is_interrupt: true,
                    tval: 0,
                });
            }
        }
        None
    }

    fn take_trap(&mut self, trap: Trap) {
        let cause = trap.cause | ((trap.is_interrupt as u64) << 63);

        // println!(
        //     "Pc: {:#018x}, instruction: {:#010x}, ra: {:#018x}",
        //     self.pc,
        //     self.instruction,
        //     self.get_reg(Register::ra as u8)
        // );
        // println!("Cause: {}, Tval: {}", trap.cause, trap.tval);

        self.csrs.write(Csr::Mepc, self.pc);
        self.csrs.write(Csr::Mcause, cause);
        self.csrs.write(Csr::Mtval, trap.tval);

        let mut status = self.csrs.read(Csr::Mstatus);
        let mie = (status >> 3) & 1;

        status = (status & !(1 << 7)) | (mie << 7);
        status &= !(1 << 3);
        status = (status & !(0b11 << 11)) | ((self.priv_mode as u64) << 11);

        self.csrs.write(Csr::Mstatus, status);

        let readback = self.csrs.read(Csr::Mstatus);
        // eprintln!(
        //     "take_trap: wrote mstatus={:#x}, readback={:#x}",
        //     status, readback
        // );

        self.priv_mode = 3;

        let mtvec = self.csrs.read(Csr::Mtvec);
        let base = mtvec & !0b11;
        let mode = mtvec & 0b11;

        self.pc = if trap.is_interrupt && mode == 1 {
            base + 4 * trap.cause
        } else {
            base
        };

        self.jump = true;
    }

    fn set_csr(&mut self, csr: u16, value: u64) {
        self.csrs.set_csr(csr, value);
    }

    fn extend_byte(value: u8, extension: Extension) -> u64 {
        match extension {
            Extension::SignExtend => value as i8 as i64 as u64,
            Extension::NoExtend | Extension::ZeroExtend => value as u64,
        }
    }

    fn extend_hword(value: u16, extension: Extension) -> u64 {
        match extension {
            Extension::SignExtend => value as i16 as i64 as u64,
            Extension::NoExtend | Extension::ZeroExtend => value as u64,
        }
    }

    fn extend_word(value: u32, extension: Extension) -> u64 {
        match extension {
            Extension::SignExtend => value as i32 as i64 as u64,
            Extension::NoExtend | Extension::ZeroExtend => value as u64,
        }
    }

    fn atomic_operation_word(old_val: u32, rs2_val: u32, atomic_op: AtomicOperation) -> u32 {
        match atomic_op {
            AtomicOperation::Swap => rs2_val,
            AtomicOperation::Add => (old_val as i32).wrapping_add(rs2_val as i32) as u32,
            AtomicOperation::Xor => old_val ^ rs2_val,
            AtomicOperation::And => old_val & rs2_val,
            AtomicOperation::Or => old_val | rs2_val,
            AtomicOperation::Min => {
                if old_val as i32 > rs2_val as i32 {
                    rs2_val
                } else {
                    old_val
                }
            }
            AtomicOperation::Max => {
                if old_val as i32 > rs2_val as i32 {
                    old_val
                } else {
                    rs2_val
                }
            }
            AtomicOperation::Minu => {
                if old_val > rs2_val {
                    rs2_val
                } else {
                    old_val
                }
            }
            AtomicOperation::Maxu => {
                if old_val > rs2_val {
                    old_val
                } else {
                    rs2_val
                }
            }
        }
    }

    fn atomic_operation_dword(old_val: u64, rs2_val: u64, atomic_op: AtomicOperation) -> u64 {
        match atomic_op {
            AtomicOperation::Swap => rs2_val,
            AtomicOperation::Add => (old_val as i64).wrapping_add(rs2_val as i64) as u64,
            AtomicOperation::Xor => old_val ^ rs2_val,
            AtomicOperation::And => old_val & rs2_val,
            AtomicOperation::Or => old_val | rs2_val,
            AtomicOperation::Min => {
                if old_val as i64 > rs2_val as i64 {
                    rs2_val
                } else {
                    old_val
                }
            }
            AtomicOperation::Max => {
                if old_val as i64 > rs2_val as i64 {
                    old_val
                } else {
                    rs2_val
                }
            }
            AtomicOperation::Minu => {
                if old_val > rs2_val {
                    rs2_val
                } else {
                    old_val
                }
            }
            AtomicOperation::Maxu => {
                if old_val > rs2_val {
                    old_val
                } else {
                    rs2_val
                }
            }
        }
    }

    fn decode_rm(rm: u8, dyn_rm: u8) -> RoundingMode {
        let effective = if rm == 0x7 { dyn_rm } else { rm };
        match effective {
            0x0 => RoundingMode::TiesToEven,
            0x1 => RoundingMode::TowardZero,
            0x2 => RoundingMode::TowardNegative,
            0x3 => RoundingMode::TowardPositive,
            0x4 => RoundingMode::TiesToAway,
            _ => panic!("Illegal rounding mode"),
        }
    }

    fn float_conversion_to_str(fc: FloatConversion) -> String {
        match fc {
            FloatConversion::Single => String::from("Single"),
            FloatConversion::Double => String::from("Double"),
            FloatConversion::WordSigned => String::from("WordSigned"),
            FloatConversion::WordUnsigned => String::from("WordUnsigned"),
            FloatConversion::DwordSigned => String::from("DwordSigned"),
            FloatConversion::DwordUnsigned => String::from("DwordUnsigned"),
        }
    }

    fn writeback(
        &mut self,
        instruction_result: InstructionResult,
    ) -> Result<Option<Trap>, DeviceError> {
        match instruction_result {
            InstructionResult::WriteToReg { rd, result } => {
                self.set_reg(rd, result);
            }
            InstructionResult::JumpAndLink { rd, result } => {
                let target_pc = (self.pc as i64).wrapping_add(result) as u64;
                if target_pc % 2 != 0 {
                    return Ok(Some(Trap {
                        cause: Exception::InstructionAddrMisaligned as u64,
                        is_interrupt: false,
                        tval: target_pc,
                    }));
                }
                if self.compressed {
                    self.set_reg(rd, self.pc.wrapping_add(2));
                } else {
                    self.set_reg(rd, self.pc.wrapping_add(4));
                }
                self.pc = target_pc;
                self.jump = true;
            }
            InstructionResult::JumpAndLinkReg { rd, result } => {
                let target_pc = result;
                if target_pc % 2 != 0 {
                    return Ok(Some(Trap {
                        cause: Exception::InstructionAddrMisaligned as u64,
                        is_interrupt: false,
                        tval: target_pc,
                    }));
                }
                if self.compressed {
                    self.set_reg(rd, self.pc.wrapping_add(2));
                } else {
                    self.set_reg(rd, self.pc.wrapping_add(4));
                }
                self.pc = target_pc;
                self.jump = true;
            }
            InstructionResult::WriteMemToReg {
                rd,
                addr,
                width,
                extension,
            } => {
                let data = match self.bus.as_ref().unwrap().borrow().read(addr, width) {
                    Ok(data) => data,
                    Err(error) => return Err(error),
                };
                match width {
                    MemoryWidth::Byte => self.set_reg(rd, Self::extend_byte(data as u8, extension)),
                    MemoryWidth::Hword => {
                        self.set_reg(rd, Self::extend_hword(data as u16, extension))
                    }
                    MemoryWidth::Word => {
                        self.set_reg(rd, Self::extend_word(data as u32, extension))
                    }
                    MemoryWidth::Dword => self.set_reg(rd, data),
                }
            }
            InstructionResult::WriteToMem {
                addr,
                result,
                width,
            } => {
                match self
                    .bus
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .write(addr, result, width)
                {
                    Ok(()) => {}
                    Err(error) => return Err(error),
                };
            }
            InstructionResult::WriteMemToFloatReg { rd, addr, width } => {
                if addr % width as u64 != 0 {
                    return Ok(Some(Trap {
                        cause: Exception::LoadAddrMisaligned as u64,
                        is_interrupt: false,
                        tval: addr,
                    }));
                }
                let data = match self.bus.as_ref().unwrap().borrow().read(addr, width) {
                    Ok(data) => data,
                    Err(error) => return Err(error),
                };
                match width {
                    MemoryWidth::Word => self.set_f32_reg(rd, f32::from_bits(data as u32)),
                    MemoryWidth::Dword => self.set_f64_reg(rd, f64::from_bits(data)),
                    _ => panic!("Invalid float load width"),
                }
            }
            InstructionResult::Branch { result } => {
                let target_pc = (self.pc as i64).wrapping_add(result) as u64;
                if target_pc % 2 != 0 {
                    return Ok(Some(Trap {
                        cause: Exception::InstructionAddrMisaligned as u64,
                        is_interrupt: false,
                        tval: target_pc,
                    }));
                }
                self.pc = target_pc;
                self.jump = true;
            }
            InstructionResult::AtomicLoad { rd, addr, width } => {
                let data = match self.bus.as_ref().unwrap().borrow().read(addr, width) {
                    Ok(data) => data,
                    Err(error) => return Err(error),
                };
                self.reservation = Some(addr);
                match width {
                    MemoryWidth::Word => {
                        self.set_reg(rd, Self::extend_word(data as u32, Extension::SignExtend));
                    }
                    MemoryWidth::Dword => {
                        self.set_reg(rd, data);
                    }
                    _ => panic!("Invalid atomic width"),
                }
            }
            InstructionResult::AtomicStore {
                rd,
                addr,
                result,
                width,
            } => {
                if self.reservation == Some(addr) {
                    match self
                        .bus
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .write(addr, result, width)
                    {
                        Ok(()) => {}
                        Err(error) => return Err(error),
                    };
                    self.set_reg(rd, 0);
                } else {
                    self.set_reg(rd, 1);
                }
                self.reservation = None;
            }
            InstructionResult::AtomicOp {
                rd,
                addr,
                result,
                width,
                atomic_op,
            } => {
                let data = match self.bus.as_ref().unwrap().borrow().read(addr, width) {
                    Ok(data) => data,
                    Err(error) => return Err(error),
                };
                let new_data: u64;
                match width {
                    MemoryWidth::Word => {
                        new_data =
                            Self::atomic_operation_word(data as u32, result as u32, atomic_op)
                                as u64;
                        self.set_reg(rd, Self::extend_word(data as u32, Extension::SignExtend));
                    }
                    MemoryWidth::Dword => {
                        new_data = Self::atomic_operation_dword(data, result, atomic_op);
                        self.set_reg(rd, data);
                    }
                    _ => panic!("Invalid atomic width"),
                }
                match self
                    .bus
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .write(addr, new_data, width)
                {
                    Ok(()) => {}
                    Err(error) => return Err(error),
                };
            }
            InstructionResult::CsrOp {
                rd,
                csr,
                result,
                csr_op,
            } => match csr_op {
                CsrOperation::Swap => match self.get_csr(csr) {
                    Some(csr_val) => {
                        self.set_reg(rd, csr_val);
                        self.set_csr(csr, result);
                    }
                    None => return Ok(None),
                },
                CsrOperation::Set => match self.get_csr(csr) {
                    Some(csr_val) => {
                        self.set_reg(rd, csr_val);
                        self.set_csr(csr, csr_val | result);
                    }
                    None => return Ok(None),
                },
                CsrOperation::Clear => match self.get_csr(csr) {
                    Some(csr_val) => {
                        self.set_reg(rd, csr_val);
                        self.set_csr(csr, csr_val & !result);
                    }
                    None => return Ok(None),
                },
            },
            InstructionResult::FloatOp {
                rd,
                rs1,
                rs2,
                rs3,
                rm,
                float_op,
                width,
            } => {
                let mut set_float = true;
                let mut int_result: u64 = 0;
                if width == MemoryWidth::Word {
                    let mut op1 = F32::from_f32(self.get_f32_reg(rs1));
                    let op2 = F32::from_f32(self.get_f32_reg(rs2));
                    let op3 = match rs3 {
                        Some(register) => Some(F32::from_f32(self.get_f32_reg(register))),
                        None => None,
                    };

                    let rnd = Self::decode_rm(
                        rm,
                        Self::extract_bits(self.csrs.read(Csr::Fcsr), 5, 7) as u8,
                    );

                    let mut flags = ExceptionFlags::default();
                    flags.set();

                    let result: F32 = match float_op {
                        FloatOperation::Fmadd => op1.fused_mul_add(op2, op3.unwrap(), rnd),
                        FloatOperation::Fmsub => op1.fused_mul_add(op2, op3.unwrap().neg(), rnd),
                        FloatOperation::Fnmsub => op1.neg().fused_mul_add(op2, op3.unwrap(), rnd),
                        FloatOperation::Fnmadd => {
                            op1.neg().fused_mul_add(op2, op3.unwrap().neg(), rnd)
                        }
                        FloatOperation::Fadd => op1.add(op2, rnd),
                        FloatOperation::Fsub => op1.sub(op2, rnd),
                        FloatOperation::Fmul => op1.mul(op2, rnd),
                        FloatOperation::Fdiv => op1.div(op2, rnd),
                        FloatOperation::Fsqrt => op1.sqrt(rnd),
                        FloatOperation::Fsgnj => {
                            op1.set_sign(op2.sign());
                            op1
                        }
                        FloatOperation::Fsgnjn => {
                            op1.set_sign(1 - op2.sign());
                            op1
                        }
                        FloatOperation::Fsgnjx => {
                            op1.set_sign(op1.sign() ^ op2.sign());
                            op1
                        }
                        FloatOperation::Fmin => {
                            if op1.is_signaling_nan() || op2.is_signaling_nan() {
                                self.csrs
                                    .write(Csr::Fcsr, self.csrs.read(Csr::Fcsr) | 0b10000);
                            }

                            if op1.is_nan() && op2.is_nan() {
                                F32::quiet_nan()
                            } else if op1.is_nan() {
                                op2
                            } else if op2.is_nan() {
                                op1
                            } else if op1.is_zero() && op2.is_zero() {
                                if op1.is_negative() { op1 } else { op2 }
                            } else if op1.lt_quiet(op2) {
                                op1
                            } else {
                                op2
                            }
                        }
                        FloatOperation::Fmax => {
                            if op1.is_signaling_nan() || op2.is_signaling_nan() {
                                self.csrs
                                    .write(Csr::Fcsr, self.csrs.read(Csr::Fcsr) | 0b10000);
                            }

                            if op1.is_nan() && op2.is_nan() {
                                F32::quiet_nan()
                            } else if op1.is_nan() {
                                op2
                            } else if op2.is_nan() {
                                op1
                            } else if op1.is_zero() && op2.is_zero() {
                                if op1.is_negative() { op2 } else { op1 }
                            } else if op1.lt_quiet(op2) {
                                op2
                            } else {
                                op1
                            }
                        }
                        FloatOperation::Feq => {
                            set_float = false;
                            int_result = op1.eq(op2) as u64;
                            op1
                        }
                        FloatOperation::Flt => {
                            set_float = false;
                            int_result = op1.lt(op2) as u64;
                            op1
                        }
                        FloatOperation::Fle => {
                            set_float = false;
                            int_result = op1.le(op2) as u64;
                            op1
                        }
                        FloatOperation::Fclass => {
                            set_float = false;

                            if op1.is_negative_infinity() {
                                int_result |= 1 << 0;
                            }
                            if op1.is_negative_normal() {
                                int_result |= 1 << 1;
                            }
                            if op1.is_negative_subnormal() {
                                int_result |= 1 << 2;
                            }
                            if op1.is_negative_zero() {
                                int_result |= 1 << 3;
                            }
                            if op1.is_positive_zero() {
                                int_result |= 1 << 4;
                            }
                            if op1.is_positive_subnormal() {
                                int_result |= 1 << 5;
                            }
                            if op1.is_positive_normal() {
                                int_result |= 1 << 6;
                            }
                            if op1.is_positive_infinity() {
                                int_result |= 1 << 7;
                            }
                            if op1.is_signaling_nan() {
                                int_result |= 1 << 8;
                            }
                            if op1.is_nan() && !op1.is_signaling_nan() {
                                int_result |= 1 << 9;
                            } // quiet NaN
                            op1
                        }
                        FloatOperation::Fcvt { from, to } => {
                            if from == FloatConversion::Single && to == FloatConversion::WordSigned
                            {
                                set_float = false;
                                int_result = op1.to_i32(rnd, true) as i64 as u64;
                                op1
                            } else if from == FloatConversion::Single
                                && to == FloatConversion::WordUnsigned
                            {
                                set_float = false;
                                int_result = op1.to_u32(rnd, true) as i32 as i64 as u64;
                                op1
                            } else if from == FloatConversion::Single
                                && to == FloatConversion::DwordSigned
                            {
                                set_float = false;
                                int_result = op1.to_i64(rnd, true) as u64;
                                op1
                            } else if from == FloatConversion::Single
                                && to == FloatConversion::DwordUnsigned
                            {
                                set_float = false;
                                int_result = op1.to_u64(rnd, true);
                                op1
                            } else if from == FloatConversion::WordSigned
                                && to == FloatConversion::Single
                            {
                                F32::from_i32(self.get_reg(rs1) as i32, rnd)
                            } else if from == FloatConversion::WordUnsigned
                                && to == FloatConversion::Single
                            {
                                F32::from_u32(self.get_reg(rs1) as u32, rnd)
                            } else if from == FloatConversion::DwordSigned
                                && to == FloatConversion::Single
                            {
                                F32::from_i64(self.get_reg(rs1) as i64, rnd)
                            } else if from == FloatConversion::DwordUnsigned
                                && to == FloatConversion::Single
                            {
                                F32::from_u64(self.get_reg(rs1) as u64, rnd)
                            } else if from == FloatConversion::Double
                                && to == FloatConversion::Single
                            {
                                F32::from_f64(self.get_f64_reg(rs1))
                            } else {
                                panic!(
                                    "Invalid 32 bit fcvt operation from: {}, to: {}",
                                    Self::float_conversion_to_str(from),
                                    Self::float_conversion_to_str(to)
                                )
                            }
                        }
                        FloatOperation::Fmv { from, to } => {
                            if from == FloatConversion::Double && to == FloatConversion::WordSigned
                            {
                                set_float = false;
                                int_result = self.get_f32_reg_bits(rs1) as i32 as i64 as u64;
                                op1
                            } else if from == FloatConversion::WordUnsigned
                                && to == FloatConversion::Double
                            {
                                F32::from_bits(self.get_reg(rs1) as u32)
                            } else {
                                panic!("Invalid 32 bit fmv operation");
                            }
                        }
                    };

                    flags.get();
                    self.update_fflags(flags);

                    if set_float {
                        self.set_f32_reg(rd, f32::from_bits(result.to_bits()));
                    } else {
                        self.set_reg(rd, int_result);
                    }
                } else if width == MemoryWidth::Dword {
                    let mut op1 = F64::from_f64(self.get_f64_reg(rs1));
                    let op2 = F64::from_f64(self.get_f64_reg(rs2));
                    let op3 = match rs3 {
                        Some(register) => Some(F64::from_f64(self.get_f64_reg(register))),
                        None => None,
                    };

                    let rnd = Self::decode_rm(
                        rm,
                        Self::extract_bits(self.csrs.read(Csr::Fcsr), 5, 7) as u8,
                    );

                    let mut flags = ExceptionFlags::default();
                    flags.set();

                    let result: F64 = match float_op {
                        FloatOperation::Fmadd => op1.fused_mul_add(op2, op3.unwrap(), rnd),
                        FloatOperation::Fmsub => op1.fused_mul_add(op2, op3.unwrap().neg(), rnd),
                        FloatOperation::Fnmsub => op1.neg().fused_mul_add(op2, op3.unwrap(), rnd),
                        FloatOperation::Fnmadd => {
                            op1.neg().fused_mul_add(op2, op3.unwrap().neg(), rnd)
                        }
                        FloatOperation::Fadd => op1.add(op2, rnd),
                        FloatOperation::Fsub => op1.sub(op2, rnd),
                        FloatOperation::Fmul => op1.mul(op2, rnd),
                        FloatOperation::Fdiv => op1.div(op2, rnd),
                        FloatOperation::Fsqrt => op1.sqrt(rnd),
                        FloatOperation::Fsgnj => {
                            op1.set_sign(op2.sign());
                            op1
                        }
                        FloatOperation::Fsgnjn => {
                            op1.set_sign(1 - op2.sign());
                            op1
                        }
                        FloatOperation::Fsgnjx => {
                            op1.set_sign(op1.sign() ^ op2.sign());
                            op1
                        }
                        FloatOperation::Fmin => {
                            if op1.is_signaling_nan() || op2.is_signaling_nan() {
                                self.csrs
                                    .write(Csr::Fcsr, self.csrs.read(Csr::Fcsr) | 0b10000);
                            }

                            if op1.is_nan() && op2.is_nan() {
                                F64::quiet_nan()
                            } else if op1.is_nan() {
                                op2
                            } else if op2.is_nan() {
                                op1
                            } else if op1.is_zero() && op2.is_zero() {
                                if op1.is_negative() { op1 } else { op2 }
                            } else if op1.lt_quiet(op2) {
                                op1
                            } else {
                                op2
                            }
                        }
                        FloatOperation::Fmax => {
                            if op1.is_signaling_nan() || op2.is_signaling_nan() {
                                self.csrs
                                    .write(Csr::Fcsr, self.csrs.read(Csr::Fcsr) | 0b10000);
                            }

                            if op1.is_nan() && op2.is_nan() {
                                F64::quiet_nan()
                            } else if op1.is_nan() {
                                op2
                            } else if op2.is_nan() {
                                op1
                            } else if op1.is_zero() && op2.is_zero() {
                                if op1.is_negative() { op2 } else { op1 }
                            } else if op1.lt_quiet(op2) {
                                op2
                            } else {
                                op1
                            }
                        }
                        FloatOperation::Feq => {
                            set_float = false;
                            int_result = op1.eq(op2) as u64;
                            op1
                        }
                        FloatOperation::Flt => {
                            set_float = false;
                            int_result = op1.lt(op2) as u64;
                            op1
                        }
                        FloatOperation::Fle => {
                            set_float = false;
                            int_result = op1.le(op2) as u64;
                            op1
                        }
                        FloatOperation::Fclass => {
                            set_float = false;

                            if op1.is_negative_infinity() {
                                int_result |= 1 << 0;
                            }
                            if op1.is_negative_normal() {
                                int_result |= 1 << 1;
                            }
                            if op1.is_negative_subnormal() {
                                int_result |= 1 << 2;
                            }
                            if op1.is_negative_zero() {
                                int_result |= 1 << 3;
                            }
                            if op1.is_positive_zero() {
                                int_result |= 1 << 4;
                            }
                            if op1.is_positive_subnormal() {
                                int_result |= 1 << 5;
                            }
                            if op1.is_positive_normal() {
                                int_result |= 1 << 6;
                            }
                            if op1.is_positive_infinity() {
                                int_result |= 1 << 7;
                            }
                            if op1.is_signaling_nan() {
                                int_result |= 1 << 8;
                            }
                            if op1.is_nan() && !op1.is_signaling_nan() {
                                int_result |= 1 << 9;
                            } // quiet NaN
                            op1
                        }
                        FloatOperation::Fcvt { from, to } => {
                            if from == FloatConversion::Double && to == FloatConversion::WordSigned
                            {
                                set_float = false;
                                int_result = op1.to_i32(rnd, true) as i64 as u64;
                                op1
                            } else if from == FloatConversion::Double
                                && to == FloatConversion::WordUnsigned
                            {
                                set_float = false;
                                int_result = op1.to_u32(rnd, true) as i32 as i64 as u64;
                                op1
                            } else if from == FloatConversion::Double
                                && to == FloatConversion::DwordSigned
                            {
                                set_float = false;
                                int_result = op1.to_i64(rnd, true) as u64;
                                op1
                            } else if from == FloatConversion::Double
                                && to == FloatConversion::DwordUnsigned
                            {
                                set_float = false;
                                int_result = op1.to_u64(rnd, true);
                                op1
                            } else if from == FloatConversion::WordSigned
                                && to == FloatConversion::Double
                            {
                                F64::from_i32(self.get_reg(rs1) as i32, rnd)
                            } else if from == FloatConversion::WordUnsigned
                                && to == FloatConversion::Double
                            {
                                F64::from_u32(self.get_reg(rs1) as u32, rnd)
                            } else if from == FloatConversion::DwordSigned
                                && to == FloatConversion::Double
                            {
                                F64::from_i64(self.get_reg(rs1) as i64, rnd)
                            } else if from == FloatConversion::DwordUnsigned
                                && to == FloatConversion::Double
                            {
                                F64::from_u64(self.get_reg(rs1) as u64, rnd)
                            } else if from == FloatConversion::Single
                                && to == FloatConversion::Double
                            {
                                F64::from_f32(self.get_f32_reg(rs1))
                            } else {
                                panic!("Invalid 64 bit fcvt operation")
                            }
                        }
                        FloatOperation::Fmv { from, to } => {
                            if from == FloatConversion::Double && to == FloatConversion::DwordSigned
                            {
                                set_float = false;
                                int_result = self.get_f64_reg(rs1).to_bits();
                                op1
                            } else if from == FloatConversion::DwordUnsigned
                                && to == FloatConversion::Double
                            {
                                F64::from_bits(self.get_reg(rs1))
                            } else {
                                panic!("Invalid 32 bit fmv operation");
                            }
                        }
                    };

                    flags.get();
                    self.update_fflags(flags);

                    if set_float {
                        self.set_f64_reg(rd, f64::from_bits(result.to_bits()));
                    } else {
                        self.set_reg(rd, int_result);
                    }
                } else {
                    panic!("Invalid float operation memory width");
                }
            }
            InstructionResult::Fence => {}
            InstructionResult::Mret => {
                let status = self.csrs.read(Csr::Mstatus);
                let mpie = (status >> 7) & 1;
                let mpp = (status >> 11) & 0b11;

                let mut new_status = (status & !(1 << 3)) | (mpie << 3);
                new_status |= 1 << 7;
                new_status &= !(0b11 << 11);

                // println!("Returning to: {:#018x}", self.csrs.read(Csr::Mepc));

                self.csrs.write(Csr::Mstatus, new_status);
                // self.priv_mode = mpp as u8;
                self.priv_mode = 3;
                self.pc = self.csrs.read(Csr::Mepc);
                self.jump = true;
            }
            InstructionResult::Ecall => {
                if self.get_reg(Register::a7 as u8) == 93 {
                    self.halted = true;
                }
                let cause = match self.priv_mode {
                    0 => Exception::EcallFromU,
                    1 => Exception::EcallFromS,
                    3 => Exception::EcallFromM,
                    _ => unreachable!(),
                };
                return Ok(Some(Trap {
                    cause: cause as u64,
                    is_interrupt: false,
                    tval: 0,
                }));
            }
            InstructionResult::Ebreak => {
                self.halted = self.halt_on_ebreak;
                // return Ok(Some(Trap {
                //     cause: Exception::Breakpoint as u64,
                //     is_interrupt: false,
                //     tval: self.pc,
                // }));
            }
            InstructionResult::Noop => {}
            InstructionResult::Unkown => {
                return Ok(Some(Trap {
                    cause: Exception::IllegalInstruction as u64,
                    is_interrupt: false,
                    tval: self.instruction,
                }));
            }
        }
        Ok(None)
    }

    fn execute(&self, instruction: DecodedInstruction) -> InstructionResult {
        match instruction {
            DecodedInstruction::U { opcode, rd, imm } => {
                match opcode {
                    0x17 => {
                        // AUIPC
                        InstructionResult::WriteToReg {
                            rd,
                            result: (imm as u64).wrapping_add(self.pc),
                        }
                    }
                    0x37 => InstructionResult::WriteToReg {
                        // LUI
                        rd,
                        result: imm as u64,
                    },
                    _ => InstructionResult::Unkown,
                }
            }
            DecodedInstruction::J { opcode, rd, imm } => match opcode {
                // JAL
                0x6F => InstructionResult::JumpAndLink {
                    rd,
                    result: imm as i64,
                },
                _ => InstructionResult::Unkown,
            },
            DecodedInstruction::I {
                opcode,
                rd,
                funct3,
                rs1,
                imm,
            } => match opcode {
                0x03 => {
                    // LOAD
                    match funct3 {
                        0x0 => {
                            // LB
                            InstructionResult::WriteMemToReg {
                                rd,
                                addr: (self.get_reg(rs1) as i64).wrapping_add(imm as i64) as u64,
                                width: MemoryWidth::Byte,
                                extension: Extension::SignExtend,
                            }
                        }
                        0x1 => {
                            // LH
                            InstructionResult::WriteMemToReg {
                                rd,
                                addr: (self.get_reg(rs1) as i64).wrapping_add(imm as i64) as u64,
                                width: MemoryWidth::Hword,
                                extension: Extension::SignExtend,
                            }
                        }
                        0x2 => {
                            // LW
                            InstructionResult::WriteMemToReg {
                                rd,
                                addr: (self.get_reg(rs1) as i64).wrapping_add(imm as i64) as u64,
                                width: MemoryWidth::Word,
                                extension: Extension::SignExtend,
                            }
                        }
                        0x3 => {
                            // LD
                            InstructionResult::WriteMemToReg {
                                rd,
                                addr: (self.get_reg(rs1) as i64).wrapping_add(imm as i64) as u64,
                                width: MemoryWidth::Dword,
                                extension: Extension::NoExtend,
                            }
                        }
                        0x4 => {
                            // LBU
                            InstructionResult::WriteMemToReg {
                                rd,
                                addr: (self.get_reg(rs1) as i64).wrapping_add(imm as i64) as u64,
                                width: MemoryWidth::Byte,
                                extension: Extension::ZeroExtend,
                            }
                        }
                        0x5 => {
                            // LHU
                            InstructionResult::WriteMemToReg {
                                rd,
                                addr: (self.get_reg(rs1) as i64).wrapping_add(imm as i64) as u64,
                                width: MemoryWidth::Hword,
                                extension: Extension::ZeroExtend,
                            }
                        }
                        0x6 => {
                            // LWU
                            InstructionResult::WriteMemToReg {
                                rd,
                                addr: (self.get_reg(rs1) as i64).wrapping_add(imm as i64) as u64,
                                width: MemoryWidth::Word,
                                extension: Extension::ZeroExtend,
                            }
                        }
                        _ => InstructionResult::Unkown,
                    }
                }
                0x07 => {
                    // FLW and FLD
                    InstructionResult::WriteMemToFloatReg {
                        rd,
                        addr: (self.get_reg(rs1) as i64).wrapping_add(imm as i64) as u64,
                        width: if funct3 == 0x2 {
                            MemoryWidth::Word
                        } else {
                            MemoryWidth::Dword
                        },
                    }
                }
                0x13 => {
                    let funct7 = ((imm >> 6) & 0x3F) as u8;
                    // OP-IMM
                    match funct3 {
                        0x0 => {
                            InstructionResult::WriteToReg {
                                // ADDI
                                rd,
                                result: (self.get_reg(rs1) as i64).wrapping_add(imm as i64) as u64,
                            }
                        }
                        0x1 => {
                            // SLLI
                            if funct7 != 0x00 {
                                return InstructionResult::Unkown;
                            }
                            InstructionResult::WriteToReg {
                                rd,
                                result: self.get_reg(rs1).wrapping_shl((imm & 0x3F) as u32),
                            }
                        }
                        0x2 => InstructionResult::WriteToReg {
                            // SLTI
                            rd,
                            result: if (self.get_reg(rs1) as i64) < (imm as i64) {
                                1
                            } else {
                                0
                            },
                        },
                        0x3 => InstructionResult::WriteToReg {
                            // SLTIU
                            rd,
                            result: if (self.get_reg(rs1) as u64) < (imm as u64) {
                                1
                            } else {
                                0
                            },
                        },
                        0x4 => InstructionResult::WriteToReg {
                            // XORI
                            rd,
                            result: self.get_reg(rs1) ^ (imm as u64),
                        },
                        0x5 => {
                            if funct7 == 0x00 {
                                // SRLI
                                InstructionResult::WriteToReg {
                                    rd,
                                    result: self.get_reg(rs1).wrapping_shr((imm & 0x3F) as u32),
                                }
                            } else if funct7 == 0x10 {
                                // SRAI
                                InstructionResult::WriteToReg {
                                    rd,
                                    result: ((self.get_reg(rs1) as i64)
                                        .wrapping_shr((imm & 0x3F) as u32))
                                        as u64,
                                }
                            } else {
                                InstructionResult::Unkown
                            }
                        }
                        0x6 => InstructionResult::WriteToReg {
                            // ORI
                            rd,
                            result: self.get_reg(rs1) | (imm as u64),
                        },
                        0x7 => InstructionResult::WriteToReg {
                            // ANDI
                            rd,
                            result: self.get_reg(rs1) & (imm as u64),
                        },
                        _ => InstructionResult::Unkown,
                    }
                }
                0x1B => {
                    // OP-IMM-32
                    let funct7 = ((imm >> 5) & 0x7F) as u8;
                    match funct3 {
                        0x0 => {
                            // ADDIW
                            InstructionResult::WriteToReg {
                                rd,
                                result: Self::extend_word(
                                    (self.get_reg(rs1) as i32).wrapping_add(imm as i32) as u32,
                                    Extension::SignExtend,
                                ),
                            }
                        }
                        0x1 => {
                            // SLLIW
                            if funct7 != 0x0 {
                                return InstructionResult::Unkown;
                            }
                            InstructionResult::WriteToReg {
                                rd,
                                result: Self::extend_word(
                                    (self.get_reg(rs1) as u32)
                                        .wrapping_shl((imm as u32 & 0x1F) as u32),
                                    Extension::SignExtend,
                                ),
                            }
                        }
                        0x5 => {
                            if funct7 == 0x00 {
                                // SRLIW
                                InstructionResult::WriteToReg {
                                    rd,
                                    result: Self::extend_word(
                                        (self.get_reg(rs1) as u32)
                                            .wrapping_shr((imm & 0x1F) as u32),
                                        Extension::SignExtend,
                                    ),
                                }
                            } else if funct7 == 0x20 {
                                // SRAIW
                                InstructionResult::WriteToReg {
                                    rd,
                                    result: Self::extend_word(
                                        ((self.get_reg(rs1) as i32)
                                            .wrapping_shr((imm & 0x1F) as u32))
                                            as u32,
                                        Extension::SignExtend,
                                    ),
                                }
                            } else {
                                InstructionResult::Unkown
                            }
                        }
                        _ => InstructionResult::Unkown,
                    }
                }
                0x67 => {
                    // JALR
                    if funct3 != 0x0 {
                        return InstructionResult::Unkown;
                    }
                    InstructionResult::JumpAndLinkReg {
                        rd,
                        result: ((self.get_reg(rs1) as i64).wrapping_add(imm as i64) as u64) & !1,
                    }
                }
                0x0F => {
                    // MISC-MEM (FENCE, FENCE.I)
                    InstructionResult::Fence
                }
                0x73 => {
                    // SYSTEM
                    match funct3 {
                        0x0 => match imm {
                            0x000 => InstructionResult::Ecall,
                            0x001 => InstructionResult::Ebreak,
                            0x302 => InstructionResult::Mret,
                            _ => InstructionResult::Unkown,
                        },
                        0x1 | 0x2 | 0x3 | 0x5 | 0x6 | 0x7 => {
                            InstructionResult::CsrOp {
                                rd,
                                csr: (imm as u16 & 0xFFF),
                                result: match funct3 {
                                    0x1 | 0x2 | 0x3 => self.get_reg(rs1),
                                    _ => rs1 as u64,
                                },
                                csr_op: match funct3 {
                                    0x1 | 0x5 => CsrOperation::Swap,  // CSRRW and CSRRWI
                                    0x2 | 0x6 => CsrOperation::Set,   // CSRRS and CSRRSI
                                    0x3 | 0x7 => CsrOperation::Clear, // CSRRC and CSRRCI
                                    _ => panic!("Unkown valud CSR operation"),
                                },
                            }
                        }
                        _ => InstructionResult::Unkown,
                    }
                }
                _ => InstructionResult::Unkown,
            },

            DecodedInstruction::S {
                opcode,
                funct3,
                rs1,
                rs2,
                imm,
            } => match opcode {
                0x23 => {
                    match funct3 {
                        0x0 => {
                            // SB
                            InstructionResult::WriteToMem {
                                addr: (self.get_reg(rs1) as i64).wrapping_add(imm as i64) as u64,
                                result: self.get_reg(rs2),
                                width: MemoryWidth::Byte,
                            }
                        }
                        0x1 => {
                            // SH
                            InstructionResult::WriteToMem {
                                addr: (self.get_reg(rs1) as i64).wrapping_add(imm as i64) as u64,
                                result: self.get_reg(rs2),
                                width: MemoryWidth::Hword,
                            }
                        }
                        0x2 => {
                            // SW
                            InstructionResult::WriteToMem {
                                addr: (self.get_reg(rs1) as i64).wrapping_add(imm as i64) as u64,
                                result: self.get_reg(rs2),
                                width: MemoryWidth::Word,
                            }
                        }
                        0x3 => {
                            // SD
                            InstructionResult::WriteToMem {
                                addr: (self.get_reg(rs1) as i64).wrapping_add(imm as i64) as u64,
                                result: self.get_reg(rs2),
                                width: MemoryWidth::Dword,
                            }
                        }
                        _ => InstructionResult::Unkown,
                    }
                }
                0x27 => {
                    if funct3 == 0x2 {
                        // FSW
                        InstructionResult::WriteToMem {
                            addr: (self.get_reg(rs1) as i64).wrapping_add(imm as i64) as u64,
                            result: self.get_f32_reg_bits(rs2) as u64,
                            width: MemoryWidth::Word,
                        }
                    } else if funct3 == 0x3 {
                        // FSD
                        InstructionResult::WriteToMem {
                            addr: (self.get_reg(rs1) as i64).wrapping_add(imm as i64) as u64,
                            result: self.get_f64_reg(rs2).to_bits() as u64,
                            width: MemoryWidth::Dword,
                        }
                    } else {
                        InstructionResult::Unkown
                    }
                }
                _ => InstructionResult::Unkown,
            },

            DecodedInstruction::B {
                opcode,
                funct3,
                rs1,
                rs2,
                imm,
            } => {
                match opcode {
                    0x63 => {
                        match funct3 {
                            0x0 => {
                                // BEQ
                                if self.get_reg(rs1) == self.get_reg(rs2) {
                                    InstructionResult::Branch { result: imm as i64 }
                                } else {
                                    InstructionResult::Noop
                                }
                            }
                            0x1 => {
                                // BNE
                                if self.get_reg(rs1) != self.get_reg(rs2) {
                                    InstructionResult::Branch { result: imm as i64 }
                                } else {
                                    InstructionResult::Noop
                                }
                            }
                            0x4 => {
                                // BLT
                                if (self.get_reg(rs1) as i64) < (self.get_reg(rs2) as i64) {
                                    InstructionResult::Branch { result: imm as i64 }
                                } else {
                                    InstructionResult::Noop
                                }
                            }
                            0x5 => {
                                // BGE
                                if (self.get_reg(rs1) as i64) >= (self.get_reg(rs2) as i64) {
                                    InstructionResult::Branch { result: imm as i64 }
                                } else {
                                    InstructionResult::Noop
                                }
                            }
                            0x6 => {
                                // BLTU
                                if (self.get_reg(rs1) as u64) < (self.get_reg(rs2) as u64) {
                                    InstructionResult::Branch { result: imm as i64 }
                                } else {
                                    InstructionResult::Noop
                                }
                            }
                            0x7 => {
                                // BGEU
                                if (self.get_reg(rs1) as u64) >= (self.get_reg(rs2) as u64) {
                                    InstructionResult::Branch { result: imm as i64 }
                                } else {
                                    InstructionResult::Noop
                                }
                            }
                            _ => InstructionResult::Unkown,
                        }
                    }
                    _ => InstructionResult::Unkown,
                }
            }
            DecodedInstruction::R {
                opcode,
                rd,
                funct3,
                rs1,
                rs2,
                funct7,
            } => {
                match opcode {
                    0x33 => {
                        // OP
                        match funct3 {
                            0x0 => {
                                if funct7 == 0x0 {
                                    // ADD
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: self.get_reg(rs1).wrapping_add(self.get_reg(rs2)),
                                    };
                                } else if funct7 == 0x1 {
                                    // MUL
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: self.get_reg(rs1).wrapping_mul(self.get_reg(rs2)),
                                    };
                                } else if funct7 == 0x20 {
                                    // SUB
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: self.get_reg(rs1).wrapping_sub(self.get_reg(rs2)),
                                    };
                                }
                                return InstructionResult::Unkown;
                            }
                            0x1 => {
                                if funct7 == 0x0 {
                                    // SLL
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: self
                                            .get_reg(rs1)
                                            .wrapping_shl(self.get_reg(rs2) as u32),
                                    };
                                } else if funct7 == 0x1 {
                                    // MULH
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: ((self.get_reg(rs1) as i64 as i128
                                            * self.get_reg(rs2) as i64 as i128)
                                            .wrapping_shr(64))
                                            as u64,
                                    };
                                }
                                return InstructionResult::Unkown;
                            }
                            0x2 => {
                                if funct7 == 0x0 {
                                    // SLT
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: if (self.get_reg(rs1) as i64)
                                            < (self.get_reg(rs2) as i64)
                                        {
                                            1
                                        } else {
                                            0
                                        },
                                    };
                                } else if funct7 == 0x1 {
                                    // MULHSU
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: ((self.get_reg(rs1) as i64 as i128
                                            * self.get_reg(rs2) as u128 as i128)
                                            .wrapping_shr(64))
                                            as u64,
                                    };
                                }
                                return InstructionResult::Unkown;
                            }
                            0x3 => {
                                if funct7 == 0x0 {
                                    // SLTU
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: if (self.get_reg(rs1) as u64)
                                            < (self.get_reg(rs2) as u64)
                                        {
                                            1
                                        } else {
                                            0
                                        },
                                    };
                                } else if funct7 == 0x1 {
                                    // MULHU
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: ((self.get_reg(rs1) as u128
                                            * self.get_reg(rs2) as u128)
                                            .wrapping_shr(64))
                                            as u64,
                                    };
                                }
                                return InstructionResult::Unkown;
                            }
                            0x4 => {
                                if funct7 == 0x0 {
                                    // XOR
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: self.get_reg(rs1) ^ self.get_reg(rs2),
                                    };
                                } else if funct7 == 0x1 {
                                    // DIV
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: if self.get_reg(rs2) == 0 {
                                            u64::MAX
                                        } else if self.get_reg(rs1) as i64 == i64::MIN
                                            && self.get_reg(rs2) as i64 == -1
                                        {
                                            self.get_reg(rs1)
                                        } else {
                                            ((self.get_reg(rs1) as i64)
                                                .wrapping_div(self.get_reg(rs2) as i64))
                                                as u64
                                        },
                                    };
                                }
                                return InstructionResult::Unkown;
                            }
                            0x5 => {
                                if funct7 == 0x00 {
                                    // SRL
                                    InstructionResult::WriteToReg {
                                        rd,
                                        result: self
                                            .get_reg(rs1)
                                            .wrapping_shr(self.get_reg(rs2) as u32 & 0x3F),
                                    }
                                } else if funct7 == 0x1 {
                                    // DIVU
                                    InstructionResult::WriteToReg {
                                        rd,
                                        result: if self.get_reg(rs2) == 0 {
                                            u64::MAX
                                        } else {
                                            self.get_reg(rs1).wrapping_div(self.get_reg(rs2))
                                        },
                                    }
                                } else if funct7 == 0x20 {
                                    // SRA
                                    InstructionResult::WriteToReg {
                                        rd,
                                        result: ((self.get_reg(rs1) as i64)
                                            .wrapping_shr(self.get_reg(rs2) as u32 & 0x3F))
                                            as u64,
                                    }
                                } else {
                                    InstructionResult::Unkown
                                }
                            }
                            0x6 => {
                                if funct7 == 0x0 {
                                    // OR
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: self.get_reg(rs1) | self.get_reg(rs2),
                                    };
                                } else if funct7 == 0x1 {
                                    // REM
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: if self.get_reg(rs2) == 0 {
                                            self.get_reg(rs1)
                                        } else if self.get_reg(rs1) as i64 == i64::MIN
                                            && self.get_reg(rs2) as i64 == -1
                                        {
                                            0
                                        } else {
                                            ((self.get_reg(rs1) as i64)
                                                .wrapping_rem(self.get_reg(rs2) as i64))
                                                as u64
                                        },
                                    };
                                }
                                return InstructionResult::Unkown;
                            }
                            0x7 => {
                                if funct7 == 0x0 {
                                    // AND
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: self.get_reg(rs1) & self.get_reg(rs2),
                                    };
                                } else if funct7 == 0x1 {
                                    // REMU
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: if self.get_reg(rs2) == 0 {
                                            self.get_reg(rs1)
                                        } else {
                                            self.get_reg(rs1).wrapping_rem(self.get_reg(rs2))
                                        },
                                    };
                                }
                                return InstructionResult::Unkown;
                            }
                            _ => InstructionResult::Unkown,
                        }
                    }
                    0x3B => {
                        // OP-32
                        match funct3 {
                            0x0 => {
                                if funct7 == 0x0 {
                                    // ADDW
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: Self::extend_word(
                                            (self.get_reg(rs1) as i32)
                                                .wrapping_add(self.get_reg(rs2) as i32)
                                                as u32,
                                            Extension::SignExtend,
                                        ),
                                    };
                                } else if funct7 == 0x1 {
                                    // MULW
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: Self::extend_word(
                                            (self.get_reg(rs1) as u32)
                                                .wrapping_mul(self.get_reg(rs2) as u32),
                                            Extension::SignExtend,
                                        ),
                                    };
                                } else if funct7 == 0x20 {
                                    // SUBW
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: Self::extend_word(
                                            (self.get_reg(rs1) as i32)
                                                .wrapping_sub(self.get_reg(rs2) as i32)
                                                as u32,
                                            Extension::SignExtend,
                                        ),
                                    };
                                }
                                return InstructionResult::Unkown;
                            }
                            0x1 => {
                                if funct7 == 0x0 {
                                    // SLLW
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: Self::extend_word(
                                            (self.get_reg(rs1) as u32).wrapping_shl(
                                                (self.get_reg(rs2) as u32 & 0x1F) as u32,
                                            ),
                                            Extension::SignExtend,
                                        ),
                                    };
                                }
                                return InstructionResult::Unkown;
                            }
                            0x4 => {
                                if funct7 == 0x1 {
                                    // DIVW
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: Self::extend_word(
                                            if self.get_reg(rs2) == 0 {
                                                u32::MAX
                                            } else if self.get_reg(rs1) as i32 == i32::MIN
                                                && self.get_reg(rs2) as i32 == -1
                                            {
                                                self.get_reg(rs1) as u32
                                            } else {
                                                ((self.get_reg(rs1) as i32)
                                                    .wrapping_div(self.get_reg(rs2) as i32))
                                                    as u32
                                            },
                                            Extension::SignExtend,
                                        ),
                                    };
                                }
                                return InstructionResult::Unkown;
                            }
                            0x5 => {
                                if funct7 == 0x00 {
                                    // SRLW
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: Self::extend_word(
                                            (self.get_reg(rs1) as u32)
                                                .wrapping_shr((self.get_reg(rs2) & 0x1F) as u32),
                                            Extension::SignExtend,
                                        ),
                                    };
                                } else if funct7 == 0x1 {
                                    // DIVUW
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: Self::extend_word(
                                            if self.get_reg(rs2) == 0 {
                                                u32::MAX
                                            } else {
                                                (self.get_reg(rs1) as u32)
                                                    .wrapping_div(self.get_reg(rs2) as u32)
                                            },
                                            Extension::SignExtend,
                                        ),
                                    };
                                } else if funct7 == 0x20 {
                                    // SRAW
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: Self::extend_word(
                                            ((self.get_reg(rs1) as i32)
                                                .wrapping_shr((self.get_reg(rs2) & 0x1F) as u32))
                                                as u32,
                                            Extension::SignExtend,
                                        ),
                                    };
                                }
                                return InstructionResult::Unkown;
                            }
                            0x6 => {
                                if funct7 == 0x1 {
                                    // REMW
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: Self::extend_word(
                                            if self.get_reg(rs2) == 0 {
                                                self.get_reg(rs1) as u32
                                            } else if self.get_reg(rs1) as i32 == i32::MIN
                                                && self.get_reg(rs2) as i32 == -1
                                            {
                                                0
                                            } else {
                                                ((self.get_reg(rs1) as i32)
                                                    .wrapping_rem(self.get_reg(rs2) as i32))
                                                    as u32
                                            },
                                            Extension::SignExtend,
                                        ),
                                    };
                                }
                                return InstructionResult::Unkown;
                            }
                            0x7 => {
                                if funct7 == 0x1 {
                                    // REMUW
                                    return InstructionResult::WriteToReg {
                                        rd,
                                        result: Self::extend_word(
                                            if self.get_reg(rs2) == 0 {
                                                self.get_reg(rs1) as u32
                                            } else {
                                                (self.get_reg(rs1) as u32)
                                                    .wrapping_rem(self.get_reg(rs2) as u32)
                                            },
                                            Extension::SignExtend,
                                        ),
                                    };
                                }
                                return InstructionResult::Unkown;
                            }
                            _ => InstructionResult::Unkown,
                        }
                    }
                    0x53 => {
                        // OP-FP
                        const SUPPORTED_FLOAT_OPS: [u8; 14] = [
                            0x0, 0x4, 0x8, 0xC, 0x2C, 0x10, 0x14, 0x50, 0x70, 0x60, 0x68, 0x20,
                            0x70, 0x78,
                        ];
                        if !SUPPORTED_FLOAT_OPS.contains(&(funct7 & 0xFE)) {
                            return InstructionResult::Unkown;
                        }
                        InstructionResult::FloatOp {
                            rd,
                            rs1,
                            rs2,
                            rs3: None,
                            rm: funct3,
                            float_op: match funct7 & 0xFE {
                                0x00 => FloatOperation::Fadd,
                                0x04 => FloatOperation::Fsub,
                                0x08 => FloatOperation::Fmul,
                                0x0C => FloatOperation::Fdiv,
                                0x2C => FloatOperation::Fsqrt,
                                0x10 => match funct3 {
                                    0x0 => FloatOperation::Fsgnj,
                                    0x1 => FloatOperation::Fsgnjn,
                                    0x2 => FloatOperation::Fsgnjx,
                                    _ => panic!(
                                        "Unkown float operation funct7: {}, funct3: {}",
                                        funct7, funct3
                                    ),
                                },
                                0x14 => match funct3 {
                                    0x0 => FloatOperation::Fmin,
                                    0x1 => FloatOperation::Fmax,
                                    _ => panic!(
                                        "Unkown float operation funct7: {}, funct3: {}",
                                        funct7, funct3
                                    ),
                                },
                                0x50 => match funct3 {
                                    0x0 => FloatOperation::Fle,
                                    0x1 => FloatOperation::Flt,
                                    0x2 => FloatOperation::Feq,
                                    _ => panic!(
                                        "Unkown float operation funct7: {}, funct3: {}",
                                        funct7, funct3
                                    ),
                                },
                                0x70 => match funct3 {
                                    0x0 => FloatOperation::Fmv {
                                        from: FloatConversion::Double,
                                        to: if funct7 & 0x1 == 0 {
                                            FloatConversion::WordSigned
                                        } else {
                                            FloatConversion::DwordSigned
                                        },
                                    },
                                    0x1 => FloatOperation::Fclass,
                                    _ => panic!(
                                        "Unkown float operation funct7: {}, funct3: {}",
                                        funct7, funct3
                                    ),
                                },
                                0x60 => match rs2 {
                                    0x0 => FloatOperation::Fcvt {
                                        from: if funct7 & 0x1 == 0 {
                                            FloatConversion::Single
                                        } else {
                                            FloatConversion::Double
                                        },
                                        to: WordSigned,
                                    },
                                    0x1 => FloatOperation::Fcvt {
                                        from: if funct7 & 0x1 == 0 {
                                            FloatConversion::Single
                                        } else {
                                            FloatConversion::Double
                                        },
                                        to: WordUnsigned,
                                    },
                                    0x2 => FloatOperation::Fcvt {
                                        from: if funct7 & 0x1 == 0 {
                                            FloatConversion::Single
                                        } else {
                                            FloatConversion::Double
                                        },
                                        to: FloatConversion::DwordSigned,
                                    },
                                    0x3 => FloatOperation::Fcvt {
                                        from: if funct7 & 0x1 == 0 {
                                            FloatConversion::Single
                                        } else {
                                            FloatConversion::Double
                                        },
                                        to: FloatConversion::DwordUnsigned,
                                    },
                                    _ => panic!(
                                        "Unkown float operation funct7: {}, funct3: {}",
                                        funct7, funct3
                                    ),
                                },
                                0x68 => match rs2 {
                                    0x0 => FloatOperation::Fcvt {
                                        from: WordSigned,
                                        to: if funct7 & 0x1 == 0 {
                                            FloatConversion::Single
                                        } else {
                                            FloatConversion::Double
                                        },
                                    },
                                    0x1 => FloatOperation::Fcvt {
                                        from: WordUnsigned,
                                        to: if funct7 & 0x1 == 0 {
                                            FloatConversion::Single
                                        } else {
                                            FloatConversion::Double
                                        },
                                    },
                                    0x2 => FloatOperation::Fcvt {
                                        from: FloatConversion::DwordSigned,
                                        to: if funct7 & 0x1 == 0 {
                                            FloatConversion::Single
                                        } else {
                                            FloatConversion::Double
                                        },
                                    },
                                    0x3 => FloatOperation::Fcvt {
                                        from: FloatConversion::DwordUnsigned,
                                        to: if funct7 & 0x1 == 0 {
                                            FloatConversion::Single
                                        } else {
                                            FloatConversion::Double
                                        },
                                    },
                                    _ => panic!(
                                        "Unkown float operation funct7: {}, rs2: {}",
                                        funct7, rs2
                                    ),
                                },
                                0x20 => match rs2 {
                                    0x0 => FloatOperation::Fcvt {
                                        from: FloatConversion::Single,
                                        to: FloatConversion::Double,
                                    },
                                    0x1 => FloatOperation::Fcvt {
                                        from: FloatConversion::Double,
                                        to: FloatConversion::Single,
                                    },
                                    _ => panic!(
                                        "Unkown float operation funct7: {}, rs2: {}",
                                        funct7, rs2
                                    ),
                                },
                                0x78 => FloatOperation::Fmv {
                                    from: if funct7 & 0x1 == 0 {
                                        WordUnsigned
                                    } else {
                                        FloatConversion::DwordUnsigned
                                    },
                                    to: FloatConversion::Double,
                                },
                                _ => panic!("Unkown float operation: {}", funct7),
                            },
                            width: if funct7 & 0x1 == 0 {
                                MemoryWidth::Word
                            } else {
                                MemoryWidth::Dword
                            },
                        }
                    }
                    _ => InstructionResult::Unkown,
                }
            }
            DecodedInstruction::R4 {
                opcode,
                rd,
                rm,
                rs1,
                rs2,
                funct2,
                rs3,
            } => {
                match opcode {
                    0x43 | 0x47 | 0x4B | 0x4F => {
                        // Fmadd
                        InstructionResult::FloatOp {
                            rd,
                            rs1,
                            rs2,
                            rs3: Some(rs3),
                            rm,
                            float_op: match opcode {
                                0x43 => FloatOperation::Fmadd,
                                0x47 => FloatOperation::Fmsub,
                                0x4B => FloatOperation::Fnmsub,
                                0x4F => FloatOperation::Fnmadd,
                                _ => panic!("Invalid fused multiply operation: {}", opcode),
                            },
                            width: if funct2 == 0 {
                                MemoryWidth::Word
                            } else {
                                MemoryWidth::Dword
                            },
                        }
                    }
                    _ => InstructionResult::Unkown,
                }
            }
            DecodedInstruction::Atomic {
                opcode,
                rd,
                funct3,
                rs1,
                rs2,
                rl,
                aq,
                funct5,
            } => {
                const SUPPORTED_ATOMICS: [u8; 11] =
                    [0x0, 0x1, 0x2, 0x3, 0x4, 0x8, 0xC, 0x10, 0x14, 0x18, 0x1C];

                if opcode != 0x2F
                    || !matches!(funct3, 0x2 | 0x3)
                    || !SUPPORTED_ATOMICS.contains(&funct5)
                {
                    return InstructionResult::Unkown;
                }
                if funct5 == 0x2 {
                    // LR
                    return InstructionResult::AtomicLoad {
                        rd,
                        addr: self.get_reg(rs1),
                        width: if funct3 == 0x2 {
                            MemoryWidth::Word
                        } else {
                            MemoryWidth::Dword
                        },
                    };
                }
                if funct5 == 0x3 {
                    // SC
                    return InstructionResult::AtomicStore {
                        rd,
                        addr: self.get_reg(rs1),
                        result: self.get_reg(rs2),
                        width: if funct3 == 0x2 {
                            MemoryWidth::Word
                        } else {
                            MemoryWidth::Dword
                        },
                    };
                }
                return InstructionResult::AtomicOp {
                    rd,
                    addr: self.get_reg(rs1),
                    result: self.get_reg(rs2),
                    width: if funct3 == 0x2 {
                        MemoryWidth::Word
                    } else {
                        MemoryWidth::Dword
                    },
                    atomic_op: match funct5 {
                        0x0 => AtomicOperation::Add,
                        0x1 => AtomicOperation::Swap,
                        0x4 => AtomicOperation::Xor,
                        0x8 => AtomicOperation::Or,
                        0xC => AtomicOperation::And,
                        0x10 => AtomicOperation::Min,
                        0x14 => AtomicOperation::Max,
                        0x18 => AtomicOperation::Minu,
                        0x1C => AtomicOperation::Maxu,
                        _ => panic!("Unkown valid atomic operation"),
                    },
                };
            }
            DecodedInstruction::Noop => InstructionResult::Noop,
            DecodedInstruction::Unkown => InstructionResult::Unkown,
        }
    }

    fn extract_bits(value: u64, low: u32, high: u32) -> u64 {
        let width = high - low + 1;
        let mask = if width == 64 {
            u64::MAX
        } else {
            (1u64 << width) - 1
        };
        (value >> low) & mask
    }

    pub fn decode(&mut self, instruction: u64) -> DecodedInstruction {
        if Self::extract_bits(instruction, 0, 1) != 0b11 {
            // Compressed instructions
            let compressed_opcode = Self::extract_bits(instruction, 0, 1);
            let funct3 = Self::extract_bits(instruction, 13, 15);
            self.compressed = true;
            match compressed_opcode {
                0x0 => {
                    let imm: i32 = match funct3 {
                        0x0 => {
                            let imm5_4 = Self::extract_bits(instruction, 11, 12);
                            let imm9_6 = Self::extract_bits(instruction, 7, 10);
                            let imm2 = Self::extract_bits(instruction, 6, 6);
                            let imm3 = Self::extract_bits(instruction, 5, 5);
                            ((imm2 << 2) | (imm3 << 3) | (imm5_4 << 4) | (imm9_6 << 6)) as i32
                        }
                        0x1 | 0x3 | 0x5 | 0x7 => {
                            let imm7_6 = Self::extract_bits(instruction, 5, 6);
                            let imm5_3 = Self::extract_bits(instruction, 10, 12);
                            ((imm5_3 << 3) | (imm7_6 << 6)) as i32
                        }
                        _ => {
                            let imm6 = Self::extract_bits(instruction, 5, 5);
                            let imm2 = Self::extract_bits(instruction, 6, 6);
                            let imm5_3 = Self::extract_bits(instruction, 10, 12);
                            ((imm2 << 2) | (imm5_3 << 3) | (imm6 << 6)) as i32
                        }
                    };
                    if imm == 0 && funct3 == 0 {
                        return DecodedInstruction::Unkown;
                    }
                    match funct3 {
                        0x0 | 0x1 | 0x2 | 0x3 => {
                            return DecodedInstruction::I {
                                opcode: match funct3 {
                                    0x0 => 0x13, // C.ADDI4SPN
                                    0x1 => 0x7,  // C.FLD
                                    0x2 => 0x3,  // C.LW
                                    0x3 => 0x3,  // C.LD
                                    _ => 0,
                                },
                                funct3: match funct3 {
                                    0x0 => 0x0,
                                    0x1 => 0x3,
                                    0x2 => 0x2,
                                    0x3 => 0x3,
                                    _ => 0,
                                },
                                rd: Self::extract_bits(instruction, 2, 4) as u8 + 8,
                                rs1: if funct3 == 0 {
                                    Register::sp as u8
                                } else {
                                    Self::extract_bits(instruction, 7, 9) as u8 + 8
                                },
                                imm,
                            };
                        }
                        0x5 | 0x6 | 0x7 => {
                            return DecodedInstruction::S {
                                opcode: match funct3 {
                                    0x5 => 0x27, // C.FSD
                                    0x6 => 0x23, // C.SW
                                    0x7 => 0x23, // C.SD
                                    _ => 0,
                                },
                                funct3: match funct3 {
                                    0x5 => 0x3,
                                    0x6 => 0x2,
                                    0x7 => 0x3,
                                    _ => 0,
                                },
                                rs1: Self::extract_bits(instruction, 7, 9) as u8 + 8,
                                rs2: Self::extract_bits(instruction, 2, 4) as u8 + 8,
                                imm,
                            };
                        }
                        _ => return DecodedInstruction::Unkown,
                    }
                }
                0x1 => match funct3 {
                    0x0 => {
                        let rd = Self::extract_bits(instruction, 7, 11) as u8;

                        if rd == 0x0 {
                            // C.NOP
                            return DecodedInstruction::Noop;
                        }

                        let imm4_0 = Self::extract_bits(instruction, 2, 6);
                        let imm5 = Self::extract_bits(instruction, 12, 12);

                        let imm_raw = imm4_0 | (imm5 << 5);
                        let imm = if imm_raw & 0x20 != 0 {
                            imm_raw | 0xFFFFFFE0
                        } else {
                            imm_raw
                        };

                        // C.ADDI
                        return DecodedInstruction::I {
                            opcode: 0x13,
                            rd: rd,
                            funct3: 0x0,
                            rs1: rd,
                            imm: imm as i32,
                        };
                    }
                    0x1 => {
                        // C.ADDIW
                        let rd = Self::extract_bits(instruction, 7, 11) as u8;
                        if rd == 0 {
                            return DecodedInstruction::Unkown;
                        }

                        let imm4_0 = Self::extract_bits(instruction, 2, 6);
                        let imm5 = Self::extract_bits(instruction, 12, 12);

                        let imm_raw = imm4_0 | (imm5 << 5);
                        let imm = if imm_raw & 0x20 != 0 {
                            imm_raw | 0xFFFFFFE0
                        } else {
                            imm_raw
                        };

                        return DecodedInstruction::I {
                            opcode: 0x1B,
                            rd: rd,
                            funct3: 0x0,
                            rs1: rd,
                            imm: imm as i32,
                        };
                    }
                    0x2 => {
                        // C.LI
                        let imm4_0 = Self::extract_bits(instruction, 2, 6);
                        let imm5 = Self::extract_bits(instruction, 12, 12);

                        let imm_raw = imm4_0 | (imm5 << 5);
                        let imm = if imm_raw & 0x20 != 0 {
                            imm_raw | 0xFFFFFFC0
                        } else {
                            imm_raw
                        };

                        return DecodedInstruction::I {
                            opcode: 0x13,
                            rd: Self::extract_bits(instruction, 7, 11) as u8,
                            funct3: 0x0,
                            rs1: Register::zero as u8,
                            imm: imm as i32,
                        };
                    }
                    0x3 => {
                        let rd = Self::extract_bits(instruction, 7, 11) as u8;

                        if rd == 0x2 {
                            // C.ADDI16SP
                            let imm5 = Self::extract_bits(instruction, 2, 2);
                            let imm8_7 = Self::extract_bits(instruction, 3, 4);
                            let imm6 = Self::extract_bits(instruction, 5, 5);
                            let imm4 = Self::extract_bits(instruction, 6, 6);
                            let imm9 = Self::extract_bits(instruction, 12, 12);
                            let imm_raw = (imm4 << 4)
                                | (imm5 << 5)
                                | (imm6 << 6)
                                | (imm8_7 << 7)
                                | (imm9 << 9);

                            let imm = if imm_raw & 0x200 != 0 {
                                imm_raw | 0xFFFFFC00
                            } else {
                                imm_raw
                            };

                            return DecodedInstruction::I {
                                opcode: 0x13,
                                rd: Register::sp as u8,
                                funct3: 0x0,
                                rs1: Register::sp as u8,
                                imm: imm as i32,
                            };
                        }

                        let imm16_12 = Self::extract_bits(instruction, 2, 6);
                        let imm17 = Self::extract_bits(instruction, 12, 12);
                        let imm_raw = (imm16_12 << 12) | (imm17 << 17);

                        let imm = if imm_raw & 0x20000 != 0 {
                            imm_raw | 0xFFFC0000
                        } else {
                            imm_raw
                        };

                        // C.LUI
                        if rd == 0 || imm == 0 {
                            return DecodedInstruction::Unkown;
                        }
                        return DecodedInstruction::U {
                            opcode: 0x37,
                            rd,
                            imm: imm as i32,
                        };
                    }
                    0x4 => {
                        let rd = Self::extract_bits(instruction, 7, 9) as u8 + 8;
                        let rs2 = Self::extract_bits(instruction, 2, 4) as u8 + 8;
                        let imm4_0 = Self::extract_bits(instruction, 2, 6);
                        let imm5 = Self::extract_bits(instruction, 12, 12);
                        let imm_raw = imm4_0 | (imm5 << 5);
                        let imm = if imm_raw & 0x20 != 0 {
                            imm_raw | 0xFFFFFFC0
                        } else {
                            imm_raw
                        };

                        let funct2 = Self::extract_bits(instruction, 10, 11);

                        if funct2 == 0x0 {
                            // C.SRLI funct7 == 0x0
                            return DecodedInstruction::I {
                                opcode: 0x13,
                                rd,
                                funct3: 0x5,
                                rs1: rd,
                                imm: imm_raw as i32,
                            };
                        }
                        if funct2 == 0x1 {
                            // C.SRAI funct7 == 0x10
                            return DecodedInstruction::I {
                                opcode: 0x13,
                                rd,
                                funct3: 0x5,
                                rs1: rd,
                                imm: (imm_raw | (0x10 << 6)) as i32,
                            };
                        }
                        if funct2 == 0x2 {
                            // C.ANDI
                            return DecodedInstruction::I {
                                opcode: 0x13,
                                rd,
                                funct3: 0x7,
                                rs1: rd,
                                imm: imm as i32,
                            };
                        }

                        let funct5 = Self::extract_bits(instruction, 5, 6)
                            | (Self::extract_bits(instruction, 10, 12) << 2);

                        match funct5 {
                            0x0C => {
                                // C.SUB
                                return DecodedInstruction::R {
                                    opcode: 0x33,
                                    rd,
                                    funct3: 0x0,
                                    rs1: rd,
                                    rs2,
                                    funct7: 0x20,
                                };
                            }
                            0x0D => {
                                // C.XOR
                                return DecodedInstruction::R {
                                    opcode: 0x33,
                                    rd,
                                    funct3: 0x4,
                                    rs1: rd,
                                    rs2,
                                    funct7: 0x0,
                                };
                            }
                            0x0E => {
                                // C.OR
                                return DecodedInstruction::R {
                                    opcode: 0x33,
                                    rd,
                                    funct3: 0x6,
                                    rs1: rd,
                                    rs2,
                                    funct7: 0x0,
                                };
                            }
                            0x0F => {
                                // C.AND
                                return DecodedInstruction::R {
                                    opcode: 0x33,
                                    rd,
                                    funct3: 0x7,
                                    rs1: rd,
                                    rs2,
                                    funct7: 0x0,
                                };
                            }
                            0x1C => {
                                // C.SUBW
                                return DecodedInstruction::R {
                                    opcode: 0x3B,
                                    rd,
                                    funct3: 0x0,
                                    rs1: rd,
                                    rs2,
                                    funct7: 0x20,
                                };
                            }
                            0x1D => {
                                // C.ADDW
                                return DecodedInstruction::R {
                                    opcode: 0x3B,
                                    rd,
                                    funct3: 0x0,
                                    rs1: rd,
                                    rs2,
                                    funct7: 0x0,
                                };
                            }
                            _ => return DecodedInstruction::Unkown,
                        }
                    }
                    0x5 => {
                        let imm5 = Self::extract_bits(instruction, 2, 2);
                        let imm3_1 = Self::extract_bits(instruction, 3, 5);
                        let imm7 = Self::extract_bits(instruction, 6, 6);
                        let imm6 = Self::extract_bits(instruction, 7, 7);
                        let imm10 = Self::extract_bits(instruction, 8, 8);
                        let imm9_8 = Self::extract_bits(instruction, 9, 10);
                        let imm4 = Self::extract_bits(instruction, 11, 11);
                        let imm11 = Self::extract_bits(instruction, 12, 12);

                        let imm_raw = (imm3_1 << 1)
                            | (imm4 << 4)
                            | (imm5 << 5)
                            | (imm6 << 6)
                            | (imm7 << 7)
                            | (imm9_8 << 8)
                            | (imm10 << 10)
                            | (imm11 << 11);

                        let imm = if imm_raw & 0x800 != 0 {
                            imm_raw | 0xFFFFF000
                        } else {
                            imm_raw
                        };

                        // C.J
                        return DecodedInstruction::J {
                            opcode: 0x6F,
                            rd: Register::zero as u8,
                            imm: imm as i32,
                        };
                    }
                    0x6 => {
                        // C.BEQZ
                        let imm5 = Self::extract_bits(instruction, 2, 2);
                        let imm2_1 = Self::extract_bits(instruction, 3, 4);
                        let imm7_6 = Self::extract_bits(instruction, 5, 6);
                        let imm4_3 = Self::extract_bits(instruction, 10, 11);
                        let imm8 = Self::extract_bits(instruction, 12, 12);

                        let imm_raw = (imm2_1 << 1)
                            | (imm4_3 << 3)
                            | (imm5 << 5)
                            | (imm7_6 << 6)
                            | (imm8 << 8);

                        let imm = if imm_raw & 0x100 != 0 {
                            imm_raw | 0xFFFFFE00
                        } else {
                            imm_raw
                        };

                        return DecodedInstruction::B {
                            opcode: 0x63,
                            funct3: 0x0,
                            rs1: Self::extract_bits(instruction, 7, 9) as u8 + 8,
                            rs2: Register::zero as u8,
                            imm: imm as i32,
                        };
                    }
                    0x7 => {
                        // C.BNEZ
                        let imm5 = Self::extract_bits(instruction, 2, 2);
                        let imm2_1 = Self::extract_bits(instruction, 3, 4);
                        let imm7_6 = Self::extract_bits(instruction, 5, 6);
                        let imm4_3 = Self::extract_bits(instruction, 10, 11);
                        let imm8 = Self::extract_bits(instruction, 12, 12);

                        let imm_raw = (imm2_1 << 1)
                            | (imm4_3 << 3)
                            | (imm5 << 5)
                            | (imm7_6 << 6)
                            | (imm8 << 8);

                        let imm = if imm_raw & 0x100 != 0 {
                            imm_raw | 0xFFFFFE00
                        } else {
                            imm_raw
                        };

                        return DecodedInstruction::B {
                            opcode: 0x63,
                            funct3: 0x1,
                            rs1: Self::extract_bits(instruction, 7, 9) as u8 + 8,
                            rs2: Register::zero as u8,
                            imm: imm as i32,
                        };
                    }
                    _ => return DecodedInstruction::Unkown,
                },
                0x2 => {
                    let funct4 = Self::extract_bits(instruction, 12, 15);

                    let imm_raw: u64 = match funct4 {
                        0x0 | 0x1 => {
                            let imm4_0 = Self::extract_bits(instruction, 2, 6);
                            let imm5 = Self::extract_bits(instruction, 12, 12);

                            imm4_0 | (imm5 << 5)
                        }
                        0x2 | 0x3 | 0x6 | 0x7 => {
                            let imm8_6 = Self::extract_bits(instruction, 2, 4);
                            let imm4_3 = Self::extract_bits(instruction, 5, 6);
                            let imm5 = Self::extract_bits(instruction, 12, 12);

                            (imm4_3 << 3) | (imm5 << 5) | (imm8_6 << 6)
                        }
                        0x4 | 0x5 => {
                            let imm7_6 = Self::extract_bits(instruction, 2, 3);
                            let imm4_2 = Self::extract_bits(instruction, 4, 6);
                            let imm5 = Self::extract_bits(instruction, 12, 12);

                            (imm4_2 << 2) | (imm5 << 5) | (imm7_6 << 6)
                        }
                        0xA | 0xB | 0xE | 0xF => {
                            let imm8_6 = Self::extract_bits(instruction, 7, 9);
                            let imm5_3 = Self::extract_bits(instruction, 10, 12);

                            (imm5_3 << 3) | (imm8_6 << 6)
                        }
                        0xC | 0xD => {
                            let imm7_6 = Self::extract_bits(instruction, 7, 8);
                            let imm5_2 = Self::extract_bits(instruction, 9, 12);

                            (imm5_2 << 2) | (imm7_6 << 6)
                        }
                        _ => 0x0,
                    };

                    let rs1 = Self::extract_bits(instruction, 7, 11) as u8;
                    let rs2 = Self::extract_bits(instruction, 2, 6) as u8;

                    if (funct4 == 0x8 || funct4 == 0x9) && rs2 != 0x0 {
                        return DecodedInstruction::R {
                            opcode: 0x33,
                            rd: rs1,
                            funct3: 0x0,
                            rs1: if funct4 == 0x8 {
                                Register::zero as u8 // C.MV
                            } else {
                                rs1 // C.ADD
                            },
                            rs2,
                            funct7: 0x0,
                        };
                    }
                    if funct4 == 0x9 && rs1 == 0x0 && rs2 == 0x0 {
                        // C.EBREAK
                        return DecodedInstruction::I {
                            opcode: 0x73,
                            rd: 0x0,
                            funct3: 0x0,
                            rs1: 0x0,
                            imm: 0x1,
                        };
                    }

                    match funct4 {
                        0x0..=0x9 => {
                            if (funct4 == 0x4
                                || funct4 == 0x5
                                || funct4 == 0x6
                                || funct4 == 0x7
                                || funct4 == 0x8)
                                && rs1 == 0
                            {
                                return DecodedInstruction::Unkown;
                            }
                            return DecodedInstruction::I {
                                opcode: match funct4 {
                                    0x0 | 0x1 => 0x13, // C.SLLI
                                    0x2 | 0x3 => 0x7,  // C.FLDSP
                                    0x8 | 0x9 => 0x67, // C.JR, C.JALR
                                    _ => 0x3,          // C.LWSP, C.LDSP
                                },
                                rd: match funct4 {
                                    0x0..=0x7 => rs1,
                                    0x8 => Register::zero as u8,
                                    0x9 => Register::ra as u8,
                                    _ => Register::sp as u8,
                                },
                                funct3: match funct4 {
                                    0x0 | 0x1 => 0x1,
                                    0x4 | 0x5 => 0x2,
                                    0x8 | 0x9 => 0x0,
                                    _ => 0x3,
                                },
                                rs1: match funct4 {
                                    0x0 | 0x1 | 0x8 | 0x9 => rs1,
                                    _ => Register::sp as u8,
                                },
                                imm: match funct4 {
                                    0x8 | 0x9 => 0,
                                    _ => imm_raw as i32,
                                },
                            };
                        }
                        0xA..=0xF => {
                            return DecodedInstruction::S {
                                opcode: match funct3 {
                                    0x5 => 0x27, // C.FSDSP
                                    0x6 => 0x23, // C.SWSP
                                    _ => 0x23,   // C.SDSP
                                },
                                funct3: match funct3 {
                                    0x6 => 0x2,
                                    _ => 0x3,
                                },
                                rs1: Register::sp as u8,
                                rs2,
                                imm: imm_raw as i32,
                            };
                        }
                        _ => return DecodedInstruction::Unkown,
                    }
                }
                _ => return DecodedInstruction::Unkown,
            }
        }

        let opcode = Self::extract_bits(instruction, 0, 6) as u8;
        match opcode {
            0x17 | 0x37 => {
                // U type
                let rd = Self::extract_bits(instruction, 7, 11) as u8;
                let imm = (Self::extract_bits(instruction, 12, 31) as i32) << 12;

                DecodedInstruction::U { opcode, rd, imm }
            }
            0x6F => {
                // J type
                let rd = Self::extract_bits(instruction, 7, 11) as u8;
                let imm19_12 = Self::extract_bits(instruction, 12, 19) as i32;
                let imm11 = Self::extract_bits(instruction, 20, 20) as i32;
                let imm10_1 = Self::extract_bits(instruction, 21, 30) as i32;
                let imm20 = Self::extract_bits(instruction, 31, 31) as i32;
                let imm = (imm10_1 << 1) | (imm11 << 11) | (imm19_12 << 12) | (imm20 << 20);
                let imm = if imm20 != 0 { imm | !0x1FFFFF } else { imm };

                DecodedInstruction::J { opcode, rd, imm }
            }
            0x03 | 0x07 | 0x0F | 0x13 | 0x1B | 0x67 | 0x73 => {
                // I type
                let rd = Self::extract_bits(instruction, 7, 11) as u8;
                let funct3 = Self::extract_bits(instruction, 12, 14) as u8;
                let rs1 = Self::extract_bits(instruction, 15, 19) as u8;
                let imm_raw = Self::extract_bits(instruction, 20, 31) as u32;
                let imm = if imm_raw & 0x800 != 0 {
                    (imm_raw | 0xFFFFF000) as i32
                } else {
                    imm_raw as i32
                };

                DecodedInstruction::I {
                    opcode,
                    rd,
                    funct3,
                    rs1,
                    imm,
                }
            }
            0x23 | 0x27 => {
                // S type
                let imm4_0 = Self::extract_bits(instruction, 7, 11) as i32;
                let funct3 = Self::extract_bits(instruction, 12, 14) as u8;
                let rs1 = Self::extract_bits(instruction, 15, 19) as u8;
                let rs2 = Self::extract_bits(instruction, 20, 24) as u8;
                let imm11_5 = Self::extract_bits(instruction, 25, 31) as i32;
                let imm = imm4_0 | (imm11_5 << 5);
                let imm = if (imm & 0x800) != 0 {
                    imm | !0xFFF
                } else {
                    imm
                };

                DecodedInstruction::S {
                    opcode,
                    funct3,
                    rs1,
                    rs2,
                    imm,
                }
            }
            0x63 => {
                // B type
                let imm11 = Self::extract_bits(instruction, 7, 7) as i32;
                let imm4_1 = Self::extract_bits(instruction, 8, 11) as i32;
                let funct3 = Self::extract_bits(instruction, 12, 14) as u8;
                let rs1 = Self::extract_bits(instruction, 15, 19) as u8;
                let rs2 = Self::extract_bits(instruction, 20, 24) as u8;
                let imm10_5 = Self::extract_bits(instruction, 25, 30) as i32;
                let imm12 = Self::extract_bits(instruction, 31, 31) as i32;
                let imm = (imm4_1 << 1) | (imm10_5 << 5) | (imm11 << 11) | (imm12 << 12);
                let imm = if imm12 != 0 { imm | !0x1FFF } else { imm };

                DecodedInstruction::B {
                    opcode,
                    funct3,
                    rs1,
                    rs2,
                    imm,
                }
            }
            0x33 | 0x3B | 0x53 => {
                // R type
                let rd = Self::extract_bits(instruction, 7, 11) as u8;
                let funct3 = Self::extract_bits(instruction, 12, 14) as u8;
                let rs1 = Self::extract_bits(instruction, 15, 19) as u8;
                let rs2 = Self::extract_bits(instruction, 20, 24) as u8;
                let funct7 = Self::extract_bits(instruction, 25, 31) as u8;

                DecodedInstruction::R {
                    opcode,
                    rd,
                    funct3,
                    rs1,
                    rs2,
                    funct7,
                }
            }
            0x43 | 0x47 | 0x4B | 0x4F => {
                // R4 type
                let rd = Self::extract_bits(instruction, 7, 11) as u8;
                let rm = Self::extract_bits(instruction, 12, 14) as u8;
                let rs1 = Self::extract_bits(instruction, 15, 19) as u8;
                let rs2 = Self::extract_bits(instruction, 20, 24) as u8;
                let funct2 = Self::extract_bits(instruction, 25, 26) as u8;
                let rs3 = Self::extract_bits(instruction, 27, 31) as u8;

                DecodedInstruction::R4 {
                    opcode,
                    rd,
                    rm,
                    rs1,
                    rs2,
                    funct2,
                    rs3,
                }
            }
            0x2F => {
                // Atomics
                let rd = Self::extract_bits(instruction, 7, 11) as u8;
                let funct3 = Self::extract_bits(instruction, 12, 14) as u8;
                let rs1 = Self::extract_bits(instruction, 15, 19) as u8;
                let rs2 = Self::extract_bits(instruction, 20, 24) as u8;
                let rl = Self::extract_bits(instruction, 25, 25) as u8;
                let aq = Self::extract_bits(instruction, 26, 26) as u8;
                let funct5 = Self::extract_bits(instruction, 27, 31) as u8;

                DecodedInstruction::Atomic {
                    opcode,
                    rd,
                    funct3,
                    rs1,
                    rs2,
                    rl,
                    aq,
                    funct5,
                }
            }
            _ => {
                // Unkown
                DecodedInstruction::Unkown
            }
        }
    }

    fn update_fflags(&mut self, flags: ExceptionFlags) {
        let mut bits: u8 = 0;
        if flags.is_invalid() {
            bits |= 0b10000;
        } // NV
        if flags.is_infinite() {
            bits |= 0b01000;
        } // DZ 
        if flags.is_overflow() {
            bits |= 0b00100;
        } // OF
        if flags.is_underflow() {
            bits |= 0b00010;
        } // UF
        if flags.is_inexact() {
            bits |= 0b00001;
        } // NX
        self.csrs
            .write(Csr::Fcsr, self.csrs.read(Csr::Fcsr) | bits as u64);
    }

    fn set_float_reg(&mut self, register: u8, value: u64, width: MemoryWidth) {
        if register > 31 {
            panic!("Invalid register");
        }
        match width {
            Word => self.set_f32_reg(register, f32::from_bits(value as u32)),
            Dword => self.set_f64_reg(register, f64::from_bits(value)),
            _ => panic!("Invalid float width"),
        }
    }

    fn set_f32_reg(&mut self, register: u8, value: f32) {
        if register > 31 {
            panic!("Invalid register");
        }
        self.float_registers[register as usize] = 0xFFFFFFFF00000000 | (value.to_bits() as u64);
    }

    fn set_f64_reg(&mut self, register: u8, value: f64) {
        if register > 31 {
            panic!("Invalid register");
        }
        self.float_registers[register as usize] = value.to_bits() as u64;
    }

    fn set_reg(&mut self, register: u8, value: u64) {
        if register > 31 {
            panic!("Invalid register");
        }
        if register == 0 {
            return;
        }
        self.registers[register as usize] = value;
    }

    pub fn get_float_reg(&mut self, register: u8, width: MemoryWidth) -> u64 {
        if register > 31 {
            panic!("Invalid register");
        }
        match width {
            Word => self.get_f32_reg(register).to_bits() as u64,
            Dword => self.get_f64_reg(register).to_bits() as u64,
            _ => panic!("Invalid float width"),
        }
    }

    pub fn get_f32_reg_bits(&self, register: u8) -> u32 {
        self.float_registers[register as usize] as u32
    }

    pub fn get_f32_reg(&self, register: u8) -> f32 {
        let bits = self.float_registers[register as usize];
        if bits & 0xFFFFFFFF00000000 != 0xFFFFFFFF00000000 {
            f32::from_bits(0x7FC00000)
        } else {
            f32::from_bits(bits as u32)
        }
    }

    pub fn get_f64_reg(&self, register: u8) -> f64 {
        let bits = self.float_registers[register as usize];
        f64::from_bits(bits)
    }

    pub fn get_reg(&self, register: u8) -> u64 {
        if register > 31 {
            panic!("Invalid register");
        }
        if register == 0 {
            return 0;
        }
        return self.registers[register as usize];
    }
}
