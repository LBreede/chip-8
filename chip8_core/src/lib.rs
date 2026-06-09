use rand::random;
use std::fmt;

pub const FONTSET_SIZE: usize = 80;
const FONTSET: [u8; FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;
const RAM_SIZE: usize = 4096;
const NUM_REGS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;

const START_ADDR: u16 = 0x200;

pub type EmuResult<T = ()> = Result<T, EmuError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmuError {
    PcOutOfBounds { pc: u16 },
    MemoryOutOfBounds { addr: usize, len: usize },
    StackOverflow,
    StackUnderflow,
    InvalidKey { key: u8 },
    UnknownOpcode { opcode: u16 },
}

impl fmt::Display for EmuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PcOutOfBounds { pc } => write!(f, "program counter out of bounds: {pc:#05x}"),
            Self::MemoryOutOfBounds { addr, len } => {
                write!(
                    f,
                    "memory access out of bounds: addr={addr:#05x}, len={len}"
                )
            }
            Self::StackOverflow => write!(f, "stack overflow"),
            Self::StackUnderflow => write!(f, "stack underflow"),
            Self::InvalidKey { key } => write!(f, "invalid key register value: {key:#04x}"),
            Self::UnknownOpcode { opcode } => write!(f, "unknown opcode: {opcode:04x}"),
        }
    }
}

impl std::error::Error for EmuError {}

pub struct Emu {
    pc: u16,
    ram: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    v_reg: [u8; NUM_REGS],
    i_reg: u16,
    sp: u16,
    stack: [u16; STACK_SIZE],
    keys: [bool; NUM_KEYS],
    dt: u8,
    st: u8,
}
impl Emu {
    pub fn new() -> Self {
        let mut new_emu = Self {
            pc: START_ADDR,
            ram: [0; RAM_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            v_reg: [0; NUM_REGS],
            i_reg: 0,
            sp: 0,
            stack: [0; STACK_SIZE],
            keys: [false; NUM_KEYS],
            dt: 0,
            st: 0,
        };
        new_emu.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
        new_emu
    }

    pub fn reset(&mut self) {
        self.pc = START_ADDR;
        self.ram = [0; RAM_SIZE];
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.v_reg = [0; NUM_REGS];
        self.i_reg = 0;
        self.sp = 0;
        self.stack = [0; STACK_SIZE];
        self.keys = [false; NUM_KEYS];
        self.dt = 0;
        self.st = 0;
        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }

    pub fn tick(&mut self) -> EmuResult {
        let op = self.fetch()?;
        self.execute(op)
    }

    pub fn tick_timers(&mut self) {
        if self.dt > 0 {
            self.dt -= 1;
        }
        if self.st > 0 {
            if self.st == 1 {
                // BEEP
            }
            self.st -= 1;
        }
    }

    pub fn get_display(&self) -> &[bool] {
        &self.screen
    }

    pub fn keypress(&mut self, idx: usize, pressed: bool) {
        self.keys[idx] = pressed;
    }

    pub fn load(&mut self, data: &[u8]) {
        let start = START_ADDR as usize;
        let end = (START_ADDR as usize) + data.len();
        assert!(end <= RAM_SIZE);
        self.ram[start..end].copy_from_slice(data);
    }

    fn fetch(&mut self) -> EmuResult<u16> {
        let pc = self.pc as usize;
        if pc.checked_add(1).is_none_or(|last| last >= RAM_SIZE) {
            return Err(EmuError::PcOutOfBounds { pc: self.pc });
        }
        let higher_byte = self.ram[pc] as u16;
        let lower_byte = self.ram[pc + 1] as u16;
        let op = (higher_byte << 8) | lower_byte;
        self.pc += 2;
        Ok(op)
    }

    fn execute(&mut self, op: u16) -> EmuResult {
        let opcode_class = (op & 0xF000) >> 12;
        let x = ((op & 0x0F00) >> 8) as usize;
        let y = ((op & 0x00F0) >> 4) as usize;
        let n = op & 0x000F;
        let nn = (op & 0xFF) as u8;
        let nnn = op & 0xFFF;

        match (opcode_class, x, y, n) {
            (0, 0, 0, 0) => self.op_nop()?,              // NOP
            (0, 0, 0xE, 0) => self.op_cls()?,            // CLS
            (0, 0, 0xE, 0xE) => self.op_ret()?,          // RET
            (1, _, _, _) => self.op_jmp(nnn)?,           // JMP NNN
            (2, _, _, _) => self.op_call(nnn)?,          // CALL NNN
            (3, _, _, _) => self.op_se_vx_nn(x, nn)?,    // SKIP VX == NN
            (4, _, _, _) => self.op_sne_vx_nn(x, nn)?,   // SKIP VX != NN
            (5, _, _, 0) => self.op_se_vx_vy(x, y)?,     // SKIP VX == VY
            (6, _, _, _) => self.op_ld_vx_nn(x, nn)?,    // VX = NN
            (7, _, _, _) => self.op_add_vx_nn(x, nn)?,   // VX += NN
            (8, _, _, 0) => self.op_ld_vx_vy(x, y)?,     // VX = VY
            (8, _, _, 1) => self.op_or_vx_vy(x, y)?,     // VX |= VY
            (8, _, _, 2) => self.op_and_vx_vy(x, y)?,    // VX &= VY
            (8, _, _, 3) => self.op_xor_vx_vy(x, y)?,    // VX ^= VY
            (8, _, _, 4) => self.op_add_vx_vy(x, y)?,    // VX += VY
            (8, _, _, 5) => self.op_sub_vx_vy(x, y)?,    // VX -= VY
            (8, _, _, 6) => self.op_shr_vx(x)?,          // VX >>= 1
            (8, _, _, 7) => self.op_subn_vx_vy(x, y)?,   // VX = VY - VX
            (8, _, _, 0xE) => self.op_shl_vx(x)?,        // VX <<= 1
            (9, _, _, 0) => self.op_sne_vx_vy(x, y)?,    // SKIP VX != VY
            (0xA, _, _, _) => self.op_ld_i_nnn(nnn)?,    // I = NNN
            (0xB, _, _, _) => self.op_jmp_v0_nnn(nnn)?,  // JMP V0 + NNN
            (0xC, _, _, _) => self.op_rnd_vx_nn(x, nn)?, // VX = rand() & NN
            (0xD, _, _, _) => self.op_draw(x, y, n)?,    // DRAW
            (0xE, _, 9, 0xE) => self.op_skp_vx(x)?,      // SKIP KEY PRESS
            (0xE, _, 0xA, 1) => self.op_sknp_vx(x)?,     // SKIP KEY RELEASE
            (0xF, _, 0, 7) => self.op_ld_vx_dt(x)?,      // VX = DT
            (0xF, _, 0, 0xA) => self.op_wait_key(x)?,    // WAIT KEY
            (0xF, _, 1, 5) => self.op_ld_dt_vx(x)?,      // DT = VX
            (0xF, _, 1, 8) => self.op_ld_st_vx(x)?,      // ST = VX
            (0xF, _, 1, 0xE) => self.op_add_i_vx(x)?,    // I += VX
            (0xF, _, 2, 9) => self.op_ld_font_vx(x)?,    // I = FONT
            (0xF, _, 3, 3) => self.op_ld_bcd_vx(x)?,     // BCD
            (0xF, _, 5, 5) => self.op_ld_i_vx(x)?,       // DUMP V0 - VX
            (0xF, _, 6, 5) => self.op_ld_vx_i(x)?,       // READ V0 - VX
            _ => return Err(EmuError::UnknownOpcode { opcode: op }),
        }

        Ok(())
    }

    fn checked_ram_range(&self, addr: usize, len: usize) -> EmuResult<std::ops::Range<usize>> {
        let end = addr
            .checked_add(len)
            .ok_or(EmuError::MemoryOutOfBounds { addr, len })?;
        if end > RAM_SIZE {
            return Err(EmuError::MemoryOutOfBounds { addr, len });
        }
        Ok(addr..end)
    }

    fn push(&mut self, val: u16) -> EmuResult {
        if self.sp as usize >= STACK_SIZE {
            return Err(EmuError::StackOverflow);
        }
        self.stack[self.sp as usize] = val;
        self.sp += 1;
        Ok(())
    }

    fn pop(&mut self) -> EmuResult<u16> {
        if self.sp == 0 {
            return Err(EmuError::StackUnderflow);
        }
        self.sp -= 1;
        Ok(self.stack[self.sp as usize])
    }
}

impl Emu {
    fn op_nop(&mut self) -> EmuResult {
        Ok(())
    }

    fn op_cls(&mut self) -> EmuResult {
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        Ok(())
    }

    fn op_ret(&mut self) -> EmuResult {
        let ret_addr = self.pop()?;
        self.pc = ret_addr;
        Ok(())
    }

    fn op_jmp(&mut self, nnn: u16) -> EmuResult {
        self.pc = nnn;
        Ok(())
    }

    fn op_call(&mut self, nnn: u16) -> EmuResult {
        self.push(self.pc)?;
        self.pc = nnn;
        Ok(())
    }

    fn op_se_vx_nn(&mut self, x: usize, nn: u8) -> EmuResult {
        if self.v_reg[x] == nn {
            self.pc += 2;
        }
        Ok(())
    }

    fn op_sne_vx_nn(&mut self, x: usize, nn: u8) -> EmuResult {
        if self.v_reg[x] != nn {
            self.pc += 2;
        }
        Ok(())
    }

    fn op_se_vx_vy(&mut self, x: usize, y: usize) -> EmuResult {
        if self.v_reg[x] == self.v_reg[y] {
            self.pc += 2;
        }
        Ok(())
    }

    fn op_ld_vx_nn(&mut self, x: usize, nn: u8) -> EmuResult {
        self.v_reg[x] = nn;
        Ok(())
    }

    fn op_add_vx_nn(&mut self, x: usize, nn: u8) -> EmuResult {
        self.v_reg[x] = self.v_reg[x].wrapping_add(nn);
        Ok(())
    }

    fn op_ld_vx_vy(&mut self, x: usize, y: usize) -> EmuResult {
        self.v_reg[x] = self.v_reg[y];
        Ok(())
    }

    fn op_or_vx_vy(&mut self, x: usize, y: usize) -> EmuResult {
        self.v_reg[x] |= self.v_reg[y];
        Ok(())
    }

    fn op_and_vx_vy(&mut self, x: usize, y: usize) -> EmuResult {
        self.v_reg[x] &= self.v_reg[y];
        Ok(())
    }

    fn op_xor_vx_vy(&mut self, x: usize, y: usize) -> EmuResult {
        self.v_reg[x] ^= self.v_reg[y];
        Ok(())
    }

    fn op_add_vx_vy(&mut self, x: usize, y: usize) -> EmuResult {
        let (new_vx, carry) = self.v_reg[x].overflowing_add(self.v_reg[y]);
        let new_vf = if carry { 1 } else { 0 };
        self.v_reg[x] = new_vx;
        self.v_reg[0xF] = new_vf;
        Ok(())
    }

    fn op_sub_vx_vy(&mut self, x: usize, y: usize) -> EmuResult {
        let (new_vx, borrow) = self.v_reg[x].overflowing_sub(self.v_reg[y]);
        let new_vf = if borrow { 0 } else { 1 };
        self.v_reg[x] = new_vx;
        self.v_reg[0xF] = new_vf;
        Ok(())
    }

    fn op_shr_vx(&mut self, x: usize) -> EmuResult {
        let lsb = self.v_reg[x] & 1;
        self.v_reg[x] >>= 1;
        self.v_reg[0xF] = lsb;
        Ok(())
    }

    fn op_subn_vx_vy(&mut self, x: usize, y: usize) -> EmuResult {
        let (new_vx, borrow) = self.v_reg[y].overflowing_sub(self.v_reg[x]);
        let new_vf = if borrow { 0 } else { 1 };
        self.v_reg[x] = new_vx;
        self.v_reg[0xF] = new_vf;
        Ok(())
    }

    fn op_shl_vx(&mut self, x: usize) -> EmuResult {
        let msb = (self.v_reg[x] >> 7) & 1;
        self.v_reg[x] <<= 1;
        self.v_reg[0xF] = msb;
        Ok(())
    }

    fn op_sne_vx_vy(&mut self, x: usize, y: usize) -> EmuResult {
        if self.v_reg[x] != self.v_reg[y] {
            self.pc += 2;
        }
        Ok(())
    }

    fn op_ld_i_nnn(&mut self, nnn: u16) -> EmuResult {
        self.i_reg = nnn;
        Ok(())
    }

    fn op_jmp_v0_nnn(&mut self, nnn: u16) -> EmuResult {
        self.pc = (self.v_reg[0] as u16) + nnn;
        Ok(())
    }

    fn op_rnd_vx_nn(&mut self, x: usize, nn: u8) -> EmuResult {
        let rng: u8 = random();
        self.v_reg[x] = rng & nn;
        Ok(())
    }

    fn op_draw(&mut self, x: usize, y: usize, n: u16) -> EmuResult {
        let x_coord = self.v_reg[x] as u16;
        let y_coord = self.v_reg[y] as u16;
        let num_rows = n;
        self.checked_ram_range(self.i_reg as usize, num_rows as usize)?;
        let mut flipped = false;
        for y_line in 0..num_rows {
            let addr = self.i_reg + y_line;
            let pixels = self.ram[addr as usize];
            for x_line in 0..8 {
                if (pixels & (0b1000_0000 >> x_line)) != 0 {
                    let x = (x_coord + x_line) as usize % SCREEN_WIDTH;
                    let y = (y_coord + y_line) as usize % SCREEN_HEIGHT;
                    let idx = x + SCREEN_WIDTH * y;
                    flipped |= self.screen[idx];
                    self.screen[idx] ^= true;
                }
            }
        }
        if flipped {
            self.v_reg[0xF] = 1;
        } else {
            self.v_reg[0xF] = 0;
        }
        Ok(())
    }

    fn op_skp_vx(&mut self, x: usize) -> EmuResult {
        let vx = self.v_reg[x];
        if vx as usize >= NUM_KEYS {
            return Err(EmuError::InvalidKey { key: vx });
        }
        let key = self.keys[vx as usize];
        if key {
            self.pc += 2;
        }
        Ok(())
    }

    fn op_sknp_vx(&mut self, x: usize) -> EmuResult {
        let vx = self.v_reg[x];
        if vx as usize >= NUM_KEYS {
            return Err(EmuError::InvalidKey { key: vx });
        }
        let key = self.keys[vx as usize];
        if !key {
            self.pc += 2;
        }
        Ok(())
    }

    fn op_ld_vx_dt(&mut self, x: usize) -> EmuResult {
        self.v_reg[x] = self.dt;
        Ok(())
    }

    fn op_wait_key(&mut self, x: usize) -> EmuResult {
        let mut pressed = false;
        for i in 0..self.keys.len() {
            if self.keys[i] {
                self.v_reg[x] = i as u8;
                pressed = true;
                break;
            }
        }
        if !pressed {
            self.pc -= 2;
        }
        Ok(())
    }

    fn op_ld_dt_vx(&mut self, x: usize) -> EmuResult {
        self.dt = self.v_reg[x];
        Ok(())
    }

    fn op_ld_st_vx(&mut self, x: usize) -> EmuResult {
        self.st = self.v_reg[x];
        Ok(())
    }

    fn op_add_i_vx(&mut self, x: usize) -> EmuResult {
        let vx = self.v_reg[x] as u16;
        self.i_reg = self.i_reg.wrapping_add(vx);
        Ok(())
    }

    fn op_ld_font_vx(&mut self, x: usize) -> EmuResult {
        let c = self.v_reg[x] as u16;
        self.i_reg = c * 5;
        Ok(())
    }

    fn op_ld_bcd_vx(&mut self, x: usize) -> EmuResult {
        let vx = self.v_reg[x];
        let range = self.checked_ram_range(self.i_reg as usize, 3)?;
        let i = range.start;
        self.ram[i] = vx / 100;
        self.ram[i + 1] = (vx % 100) / 10;
        self.ram[i + 2] = vx % 10;
        Ok(())
    }

    fn op_ld_i_vx(&mut self, x: usize) -> EmuResult {
        let i = self.i_reg as usize;
        self.checked_ram_range(i, x + 1)?;
        for idx in 0..=x {
            self.ram[i + idx] = self.v_reg[idx];
        }
        Ok(())
    }

    fn op_ld_vx_i(&mut self, x: usize) -> EmuResult {
        let i = self.i_reg as usize;
        self.checked_ram_range(i, x + 1)?;
        for idx in 0..=x {
            self.v_reg[idx] = self.ram[i + idx];
        }
        Ok(())
    }
}

impl Default for Emu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_opcode(emu: &mut Emu, addr: u16, opcode: u16) {
        let addr = addr as usize;
        emu.ram[addr] = (opcode >> 8) as u8;
        emu.ram[addr + 1] = opcode as u8;
    }

    fn run_opcode(emu: &mut Emu, opcode: u16) -> EmuResult {
        emu.pc = START_ADDR;
        write_opcode(emu, START_ADDR, opcode);
        emu.tick()
    }

    #[test]
    fn new_initializes_core_state() {
        let emu = Emu::new();

        assert_eq!(emu.pc, START_ADDR);
        assert_eq!(&emu.ram[..FONTSET_SIZE], FONTSET);
        assert!(emu.screen.iter().all(|pixel| !pixel));
        assert_eq!(emu.v_reg, [0; NUM_REGS]);
        assert_eq!(emu.i_reg, 0);
        assert_eq!(emu.sp, 0);
        assert_eq!(emu.stack, [0; STACK_SIZE]);
        assert_eq!(emu.keys, [false; NUM_KEYS]);
        assert_eq!(emu.dt, 0);
        assert_eq!(emu.st, 0);
    }

    #[test]
    fn reset_restores_initial_state() {
        let mut emu = Emu::new();
        emu.pc = 0x300;
        emu.ram[START_ADDR as usize] = 0xAA;
        emu.screen[0] = true;
        emu.v_reg[1] = 0xBB;
        emu.i_reg = 0x444;
        emu.sp = 1;
        emu.stack[0] = 0x222;
        emu.keys[0] = true;
        emu.dt = 12;
        emu.st = 34;

        emu.reset();

        assert_eq!(emu.pc, START_ADDR);
        assert_eq!(&emu.ram[..FONTSET_SIZE], FONTSET);
        assert_eq!(emu.ram[START_ADDR as usize], 0);
        assert!(emu.screen.iter().all(|pixel| !pixel));
        assert_eq!(emu.v_reg, [0; NUM_REGS]);
        assert_eq!(emu.i_reg, 0);
        assert_eq!(emu.sp, 0);
        assert_eq!(emu.stack, [0; STACK_SIZE]);
        assert_eq!(emu.keys, [false; NUM_KEYS]);
        assert_eq!(emu.dt, 0);
        assert_eq!(emu.st, 0);
    }

    #[test]
    fn load_writes_rom_at_start_address() {
        let mut emu = Emu::new();
        let rom = [0x12, 0x34, 0xAB, 0xCD];

        emu.load(&rom);

        let start = START_ADDR as usize;
        assert_eq!(&emu.ram[start..start + rom.len()], rom);
    }

    #[test]
    fn flow_opcodes_update_pc_and_stack() {
        let mut emu = Emu::new();
        write_opcode(&mut emu, START_ADDR, 0x2300);
        write_opcode(&mut emu, 0x300, 0x00EE);

        emu.tick().unwrap();
        assert_eq!(emu.pc, 0x300);
        assert_eq!(emu.sp, 1);
        assert_eq!(emu.stack[0], START_ADDR + 2);

        emu.tick().unwrap();
        assert_eq!(emu.pc, START_ADDR + 2);
        assert_eq!(emu.sp, 0);

        run_opcode(&mut emu, 0x1456).unwrap();
        assert_eq!(emu.pc, 0x456);
    }

    #[test]
    fn skip_opcodes_advance_pc_when_condition_matches() {
        let mut emu = Emu::new();

        emu.v_reg[1] = 0x42;
        run_opcode(&mut emu, 0x3142).unwrap();
        assert_eq!(emu.pc, START_ADDR + 4);

        run_opcode(&mut emu, 0x3143).unwrap();
        assert_eq!(emu.pc, START_ADDR + 2);

        run_opcode(&mut emu, 0x4143).unwrap();
        assert_eq!(emu.pc, START_ADDR + 4);

        emu.v_reg[2] = 0x42;
        run_opcode(&mut emu, 0x5120).unwrap();
        assert_eq!(emu.pc, START_ADDR + 4);

        emu.v_reg[2] = 0x99;
        run_opcode(&mut emu, 0x9120).unwrap();
        assert_eq!(emu.pc, START_ADDR + 4);
    }

    #[test]
    fn register_and_arithmetic_opcodes_update_values_and_flags() {
        let mut emu = Emu::new();

        emu.execute(0x61FE).unwrap();
        assert_eq!(emu.v_reg[1], 0xFE);

        emu.execute(0x7105).unwrap();
        assert_eq!(emu.v_reg[1], 0x03);

        emu.v_reg[1] = 0b1010_0001;
        emu.v_reg[2] = 0b1100_0011;
        emu.execute(0x8120).unwrap();
        assert_eq!(emu.v_reg[1], 0b1100_0011);
        emu.execute(0x8121).unwrap();
        assert_eq!(emu.v_reg[1], 0b1100_0011);
        emu.execute(0x8122).unwrap();
        assert_eq!(emu.v_reg[1], 0b1100_0011);
        emu.execute(0x8123).unwrap();
        assert_eq!(emu.v_reg[1], 0);

        emu.v_reg[1] = 250;
        emu.v_reg[2] = 10;
        emu.execute(0x8124).unwrap();
        assert_eq!(emu.v_reg[1], 4);
        assert_eq!(emu.v_reg[0xF], 1);

        emu.v_reg[1] = 10;
        emu.v_reg[2] = 3;
        emu.execute(0x8125).unwrap();
        assert_eq!(emu.v_reg[1], 7);
        assert_eq!(emu.v_reg[0xF], 1);

        emu.v_reg[1] = 3;
        emu.v_reg[2] = 10;
        emu.execute(0x8125).unwrap();
        assert_eq!(emu.v_reg[1], 249);
        assert_eq!(emu.v_reg[0xF], 0);

        emu.v_reg[1] = 3;
        emu.execute(0x8106).unwrap();
        assert_eq!(emu.v_reg[1], 1);
        assert_eq!(emu.v_reg[0xF], 1);

        emu.v_reg[1] = 3;
        emu.v_reg[2] = 10;
        emu.execute(0x8127).unwrap();
        assert_eq!(emu.v_reg[1], 7);
        assert_eq!(emu.v_reg[0xF], 1);

        emu.v_reg[1] = 0b1000_0001;
        emu.execute(0x810E).unwrap();
        assert_eq!(emu.v_reg[1], 0b0000_0010);
        assert_eq!(emu.v_reg[0xF], 1);
    }

    #[test]
    fn index_and_memory_opcodes_update_i_and_ram() {
        let mut emu = Emu::new();

        emu.execute(0xA300).unwrap();
        assert_eq!(emu.i_reg, 0x300);

        emu.v_reg[0] = 0x10;
        emu.execute(0xB300).unwrap();
        assert_eq!(emu.pc, 0x310);

        emu.i_reg = 0x300;
        emu.v_reg[1] = 0x22;
        emu.execute(0xF11E).unwrap();
        assert_eq!(emu.i_reg, 0x322);

        emu.v_reg[2] = 0x0A;
        emu.execute(0xF229).unwrap();
        assert_eq!(emu.i_reg, 50);

        emu.i_reg = 0x350;
        emu.v_reg[3] = 234;
        emu.execute(0xF333).unwrap();
        assert_eq!(&emu.ram[0x350..0x353], [2, 3, 4]);

        emu.i_reg = 0x360;
        emu.v_reg[0] = 1;
        emu.v_reg[1] = 2;
        emu.v_reg[2] = 3;
        emu.execute(0xF255).unwrap();
        assert_eq!(&emu.ram[0x360..0x363], [1, 2, 3]);

        emu.v_reg[0] = 0;
        emu.v_reg[1] = 0;
        emu.v_reg[2] = 0;
        emu.execute(0xF265).unwrap();
        assert_eq!(emu.v_reg[0], 1);
        assert_eq!(emu.v_reg[1], 2);
        assert_eq!(emu.v_reg[2], 3);
    }

    #[test]
    fn draw_sets_pixels_wraps_and_reports_collisions() {
        let mut emu = Emu::new();
        emu.i_reg = 0x300;
        emu.ram[0x300] = 0b1100_0000;
        emu.v_reg[1] = (SCREEN_WIDTH - 1) as u8;
        emu.v_reg[2] = (SCREEN_HEIGHT - 1) as u8;

        emu.execute(0xD121).unwrap();

        let bottom_right = (SCREEN_WIDTH - 1) + SCREEN_WIDTH * (SCREEN_HEIGHT - 1);
        let bottom_left = SCREEN_WIDTH * (SCREEN_HEIGHT - 1);
        assert!(emu.screen[bottom_right]);
        assert!(emu.screen[bottom_left]);
        assert_eq!(emu.v_reg[0xF], 0);

        emu.execute(0xD121).unwrap();
        assert!(!emu.screen[bottom_right]);
        assert!(!emu.screen[bottom_left]);
        assert_eq!(emu.v_reg[0xF], 1);
    }

    #[test]
    fn key_opcodes_skip_based_on_key_state() {
        let mut emu = Emu::new();
        emu.v_reg[1] = 0xA;

        run_opcode(&mut emu, 0xE19E).unwrap();
        assert_eq!(emu.pc, START_ADDR + 2);

        emu.keypress(0xA, true);
        run_opcode(&mut emu, 0xE19E).unwrap();
        assert_eq!(emu.pc, START_ADDR + 4);

        run_opcode(&mut emu, 0xE1A1).unwrap();
        assert_eq!(emu.pc, START_ADDR + 2);

        emu.keypress(0xA, false);
        run_opcode(&mut emu, 0xE1A1).unwrap();
        assert_eq!(emu.pc, START_ADDR + 4);
    }

    #[test]
    fn wait_key_repeats_until_a_key_is_pressed() {
        let mut emu = Emu::new();

        run_opcode(&mut emu, 0xF10A).unwrap();
        assert_eq!(emu.pc, START_ADDR);

        emu.keypress(0xC, true);
        run_opcode(&mut emu, 0xF10A).unwrap();
        assert_eq!(emu.v_reg[1], 0xC);
        assert_eq!(emu.pc, START_ADDR + 2);
    }

    #[test]
    fn timer_opcodes_read_write_and_tick_timers() {
        let mut emu = Emu::new();
        emu.v_reg[1] = 5;
        emu.v_reg[2] = 7;

        emu.execute(0xF115).unwrap();
        emu.execute(0xF218).unwrap();
        assert_eq!(emu.dt, 5);
        assert_eq!(emu.st, 7);

        emu.execute(0xF307).unwrap();
        assert_eq!(emu.v_reg[3], 5);

        emu.tick_timers();
        assert_eq!(emu.dt, 4);
        assert_eq!(emu.st, 6);
    }

    #[test]
    fn rnd_applies_mask() {
        let mut emu = Emu::new();

        emu.execute(0xC10F).unwrap();

        assert_eq!(emu.v_reg[1] & 0xF0, 0);
    }

    #[test]
    fn tick_returns_error_for_unknown_opcode() {
        let mut emu = Emu::new();

        let err = run_opcode(&mut emu, 0xFFFF).unwrap_err();

        assert_eq!(err, EmuError::UnknownOpcode { opcode: 0xFFFF });
    }

    #[test]
    fn stack_errors_are_reported() {
        let mut emu = Emu::new();

        assert_eq!(emu.execute(0x00EE), Err(EmuError::StackUnderflow));

        for _ in 0..STACK_SIZE {
            emu.execute(0x2200).unwrap();
        }
        assert_eq!(emu.execute(0x2200), Err(EmuError::StackOverflow));
    }

    #[test]
    fn invalid_key_register_value_is_reported() {
        let mut emu = Emu::new();
        emu.v_reg[1] = 0x10;

        assert_eq!(emu.execute(0xE19E), Err(EmuError::InvalidKey { key: 0x10 }));
        assert_eq!(emu.execute(0xE1A1), Err(EmuError::InvalidKey { key: 0x10 }));
    }

    #[test]
    fn out_of_bounds_pc_and_i_are_reported() {
        let mut emu = Emu::new();
        emu.pc = (RAM_SIZE - 1) as u16;
        assert_eq!(
            emu.tick(),
            Err(EmuError::PcOutOfBounds {
                pc: (RAM_SIZE - 1) as u16
            })
        );

        emu.i_reg = (RAM_SIZE - 1) as u16;
        assert_eq!(
            emu.execute(0xF133),
            Err(EmuError::MemoryOutOfBounds {
                addr: RAM_SIZE - 1,
                len: 3
            })
        );

        emu.i_reg = (RAM_SIZE - 1) as u16;
        assert_eq!(
            emu.execute(0xD002),
            Err(EmuError::MemoryOutOfBounds {
                addr: RAM_SIZE - 1,
                len: 2
            })
        );
    }
}
