use rand::random;

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

    fn push(&mut self, val: u16) {
        self.stack[self.sp as usize] = val;
        self.sp += 1;
    }

    fn pop(&mut self) -> u16 {
        self.sp -= 1;
        self.stack[self.sp as usize]
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

    pub fn tick(&mut self) {
        let op = self.fetch();
        self.execute(op);
    }

    fn fetch(&mut self) -> u16 {
        let higher_byte = self.ram[self.pc as usize] as u16;
        let lower_byte = self.ram[(self.pc + 1) as usize] as u16;
        let op = (higher_byte << 8) | lower_byte;
        self.pc += 2;
        op
    }

    fn execute(&mut self, op: u16) {
        let kind = (op & 0xF000) >> 12;
        let x = ((op & 0x0F00) >> 8) as usize;
        let y = ((op & 0x00F0) >> 4) as usize;
        let n = op & 0x000F;
        let nn = (op & 0xFF) as u8;
        let nnn = op & 0xFFF;

        match (kind, x, y, n) {
            (0, 0, 0, 0) => (),                                             // NOP
            (0, 0, 0xE, 0) => self.op_cls(),                                // CLS
            (0, 0, 0xE, 0xE) => self.op_ret(),                              // RET
            (1, _, _, _) => self.op_jmp(nnn),                               // JMP NNN
            (2, _, _, _) => self.op_call(nnn),                              // CALL NNN
            (3, _, _, _) => self.op_se_vx_nn(x, nn),                        // SKIP VX == NN
            (4, _, _, _) => self.op_sne_vx_nn(x, nn),                       // SKIP VX != NN
            (5, _, _, 0) => self.op_se_vx_vy(x, y),                         // SKIP VX == VY
            (6, _, _, _) => self.v_reg[x] = nn,                             // VX = NN
            (7, _, _, _) => self.v_reg[x] = self.v_reg[x].wrapping_add(nn), // VX += NN
            (8, _, _, 0) => self.v_reg[x] = self.v_reg[y],                  // VX = VY
            (8, _, _, 1) => self.op_or_vx_vy(x, y),                         // VX |= VY
            (8, _, _, 2) => self.op_and_vx_vy(x, y),                        // VX &= VY
            (8, _, _, 3) => self.op_xor_vx_vy(x, y),                        // VX ^= VY
            (8, _, _, 4) => self.op_add_vx_vy(x, y),                        // VX += VY
            (8, _, _, 5) => self.op_sub_vx_vy(x, y),                        // VX -= VY
            (8, _, _, 6) => self.op_shr_vx(x),                              // VX >>= 1
            (8, _, _, 7) => self.op_subn_vx_vy(x, y),                       // VX = VY - VX
            (8, _, _, 0xE) => self.op_shl_vx(x),                            // VX <<= 1
            (9, _, _, 0) => self.op_sne_vx_vy(x, y),                        // SKIP VX != VY
            (0xA, _, _, _) => self.i_reg = nnn,                             // I = NNN
            (0xB, _, _, _) => self.pc = (self.v_reg[0] as u16) + nnn,       // JMP V0 + NNN
            (0xC, _, _, _) => self.op_rnd_vx_nn(x, nn),                     // VX = rand() & NN
            (0xD, _, _, _) => self.op_draw(x, y, n),                        // DRAW
            (0xE, _, 9, 0xE) => self.op_skp_vx(x),                          // SKIP KEY PRESS
            (0xE, _, 0xA, 1) => self.op_sknp_vx(x),                         // SKIP KEY RELEASE
            (0xF, _, 0, 7) => self.v_reg[x] = self.dt,                      // VX = DT
            (0xF, _, 0, 0xA) => self.op_wait_key(x),                        // WAIT KEY
            (0xF, _, 1, 5) => self.dt = self.v_reg[x],                      // DT = VX
            (0xF, _, 1, 8) => self.st = self.v_reg[x],                      // ST = VX
            (0xF, _, 1, 0xE) => self.op_add_i_vx(x),                        // I += VX
            (0xF, _, 2, 9) => self.op_ld_font_vx(x),                        // I = FONT
            (0xF, _, 3, 3) => self.op_ld_bcd_vx(x),                         // BCD
            (0xF, _, 5, 5) => self.op_ld_i_vx(x),                           // DUMP V0 - VX
            (0xF, _, 6, 5) => self.op_ld_vx_i(x),                           // READ V0 - VX
            _ => panic!("Unknown opcode: {op:04x}"),
        }
    }

    fn op_cls(&mut self) {
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
    }

    fn op_ret(&mut self) {
        let ret_addr = self.pop();
        self.pc = ret_addr;
    }

    fn op_jmp(&mut self, nnn: u16) {
        self.pc = nnn;
    }

    fn op_call(&mut self, nnn: u16) {
        self.push(self.pc);
        self.pc = nnn;
    }

    fn op_se_vx_nn(&mut self, x: usize, nn: u8) {
        if self.v_reg[x] == nn {
            self.pc += 2;
        }
    }

    fn op_sne_vx_nn(&mut self, x: usize, nn: u8) {
        if self.v_reg[x] != nn {
            self.pc += 2;
        }
    }

    fn op_se_vx_vy(&mut self, x: usize, y: usize) {
        if self.v_reg[x] == self.v_reg[y] {
            self.pc += 2;
        }
    }

    fn op_or_vx_vy(&mut self, x: usize, y: usize) {
        self.v_reg[x] |= self.v_reg[y];
    }
    fn op_and_vx_vy(&mut self, x: usize, y: usize) {
        self.v_reg[x] &= self.v_reg[y];
    }
    fn op_xor_vx_vy(&mut self, x: usize, y: usize) {
        self.v_reg[x] ^= self.v_reg[y];
    }

    fn op_add_vx_vy(&mut self, x: usize, y: usize) {
        let (new_vx, carry) = self.v_reg[x].overflowing_add(self.v_reg[y]);
        let new_vf = if carry { 1 } else { 0 };
        self.v_reg[x] = new_vx;
        self.v_reg[0xF] = new_vf;
    }

    fn op_sub_vx_vy(&mut self, x: usize, y: usize) {
        let (new_vx, borrow) = self.v_reg[x].overflowing_sub(self.v_reg[y]);
        let new_vf = if borrow { 0 } else { 1 };
        self.v_reg[x] = new_vx;
        self.v_reg[0xF] = new_vf;
    }

    fn op_shr_vx(&mut self, x: usize) {
        let lsb = self.v_reg[x] & 1;
        self.v_reg[x] >>= 1;
        self.v_reg[0xF] = lsb;
    }

    fn op_subn_vx_vy(&mut self, x: usize, y: usize) {
        let (new_vx, borrow) = self.v_reg[y].overflowing_sub(self.v_reg[x]);
        let new_vf = if borrow { 0 } else { 1 };
        self.v_reg[x] = new_vx;
        self.v_reg[0xF] = new_vf;
    }

    fn op_shl_vx(&mut self, x: usize) {
        let msb = (self.v_reg[x] >> 7) & 1;
        self.v_reg[x] <<= 1;
        self.v_reg[0xF] = msb;
    }

    fn op_sne_vx_vy(&mut self, x: usize, y: usize) {
        if self.v_reg[x] != self.v_reg[y] {
            self.pc += 2;
        }
    }

    fn op_rnd_vx_nn(&mut self, x: usize, nn: u8) {
        let rng: u8 = random();
        self.v_reg[x] = rng & nn;
    }

    fn op_draw(&mut self, x: usize, y: usize, n: u16) {
        let x_coord = self.v_reg[x] as u16;
        let y_coord = self.v_reg[y] as u16;
        let num_rows = n as u16;
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
    }

    fn op_skp_vx(&mut self, x: usize) {
        let vx = self.v_reg[x];
        let key = self.keys[vx as usize];
        if key {
            self.pc += 2;
        }
    }

    fn op_sknp_vx(&mut self, x: usize) {
        let vx = self.v_reg[x];
        let key = self.keys[vx as usize];
        if !key {
            self.pc += 2;
        }
    }

    fn op_wait_key(&mut self, x: usize) {
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
    }

    fn op_add_i_vx(&mut self, x: usize) {
        let vx = self.v_reg[x] as u16;
        self.i_reg = self.i_reg.wrapping_add(vx);
    }

    fn op_ld_font_vx(&mut self, x: usize) {
        let c = self.v_reg[x] as u16;
        self.i_reg = c * 5;
    }

    fn op_ld_bcd_vx(&mut self, x: usize) {
        let vx = self.v_reg[x];
        self.ram[self.i_reg as usize] = vx / 100;
        self.ram[self.i_reg as usize + 1] = (vx % 100) / 10;
        self.ram[self.i_reg as usize + 2] = vx % 10;
    }

    fn op_ld_i_vx(&mut self, x: usize) {
        let i = self.i_reg as usize;
        for idx in 0..=x {
            self.ram[i + idx] = self.v_reg[idx];
        }
    }

    fn op_ld_vx_i(&mut self, x: usize) {
        let i = self.i_reg as usize;
        for idx in 0..=x {
            self.v_reg[idx] = self.ram[i + idx];
        }
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
}

impl Default for Emu {
    fn default() -> Self {
        Self::new()
    }
}
