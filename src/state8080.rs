use crate::memory::Memory;
use crate::disassemble::process_instruction;

use std::collections::HashMap;

// cool way to count cycles for an opcode I found here:
// https://github.com/nav97/Intel-8080-Emulator/tree/master
const CYCLES_8080: [u8; 256] = [
    4, 10, 7, 5, 5, 5, 7, 4, 4, 10, 7, 5, 5, 5, 7, 4, //0x00..0x0f
	4, 10, 7, 5, 5, 5, 7, 4, 4, 10, 7, 5, 5, 5, 7, 4, //0x10..0x1f
	4, 10, 16, 5, 5, 5, 7, 4, 4, 10, 16, 5, 5, 5, 7, 4, //etc
	4, 10, 13, 5, 10, 10, 10, 4, 4, 10, 13, 5, 5, 5, 7, 4,
	
	5, 5, 5, 5, 5, 5, 7, 5, 5, 5, 5, 5, 5, 5, 7, 5, //0x40..0x4f
	5, 5, 5, 5, 5, 5, 7, 5, 5, 5, 5, 5, 5, 5, 7, 5,
	5, 5, 5, 5, 5, 5, 7, 5, 5, 5, 5, 5, 5, 5, 7, 5,
	7, 7, 7, 7, 7, 7, 7, 7, 5, 5, 5, 5, 5, 5, 7, 5,
	
	4, 4, 4, 4, 4, 4, 7, 4, 4, 4, 4, 4, 4, 4, 7, 4, //0x80..8x4f
	4, 4, 4, 4, 4, 4, 7, 4, 4, 4, 4, 4, 4, 4, 7, 4,
	4, 4, 4, 4, 4, 4, 7, 4, 4, 4, 4, 4, 4, 4, 7, 4,
	4, 4, 4, 4, 4, 4, 7, 4, 4, 4, 4, 4, 4, 4, 7, 4,
	
	11, 10, 10, 10, 17, 11, 7, 11, 11, 10, 10, 10, 10, 17, 7, 11, //0xc0..0xcf
	11, 10, 10, 10, 17, 11, 7, 11, 11, 10, 10, 10, 10, 17, 7, 11, 
	11, 10, 10, 18, 17, 11, 7, 11, 11, 5, 10, 5, 17, 17, 7, 11, 
	11, 10, 10, 4, 17, 11, 7, 11, 11, 5, 10, 4, 17, 17, 7, 11, 
];

pub struct ConditionCodes {
    z: u8,
    s: u8,
    p: u8,
    cy: u8,
    ac: u8,
    pad: u8,
}

pub struct Port {
    pub write2: u8,
    pub shift0: u8,
    pub shift1: u8,
    pub io_ports: HashMap<u8, u8>,
}

pub struct State8080 {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,
    memory: Memory, // Use the Memory struct
    pub port: Port,
    cc: ConditionCodes,
    int_enable: u8,
}

impl State8080 {
    // Write a byte to memory at the specified address
    #[allow(dead_code)]
    pub fn write_mem(&mut self, address: u16, value: u8) {
        self.memory.write_byte(address, value);
    }

    pub fn write_rom_mem(&mut self, address: u16, value: u8) {
        self.memory.rom_write_byte(address, value)
    }

    // Read a byte from memory at the specified address
    #[allow(dead_code)]
    pub fn read_mem(&self, address: u16) -> u8 {
        self.memory.read_byte(address)
    }

    pub fn get_reg(emu8080: &State8080, reg: char) -> u8 {
        match reg {
            'a' => emu8080.a,
            'b' => emu8080.b,
            'c' => emu8080.c,
            'd' => emu8080.d,
            'e' => emu8080.e,
            'h' => emu8080.h,
            'l' => emu8080.l,
            _ => panic!("Invalid register: {}", reg),
        }
    }

    pub fn read_mem_chunk(&self, start_address: u16, end_address: u16) -> &[u8] {
        self.memory.read_byte_chunk(start_address, end_address)
    }

    #[allow(dead_code)]
    pub fn get_pc(&self) -> u16 {
        self.pc
    }

    #[allow(dead_code)]
    pub fn set_pc(&mut self,value: u16) {
        self.pc = value;
    }

    pub fn interrupt_enabled(&self) -> bool {
        self.int_enable != 0
    }

}

impl Default for State8080 {
    fn default() -> Self {
        State8080 {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
            memory: Memory::new(),
            port: Port{write2:0,shift0:0,shift1:0,io_ports:HashMap::new()},
            cc: ConditionCodes {
                z: 0,
                s: 0,
                p: 0,
                cy: 0,
                ac: 0,
                pad: 3,
            },
            int_enable: 0,
        }
    }
}


// run an instruction and return the number of cycles
pub fn emulate_8080_op(state: &mut State8080) -> u8{
    let opcode = state.memory.read_byte(state.pc);

    // may not need this in any given opcode, nice to have up here to save LOC
    let next_bytes = [state.memory.read_byte(state.pc + 1), state.memory.read_byte(state.pc + 2)];

    state.pc += 1; // Increment the program counter for the opcode

    match opcode {
        0x00 => {/*NOP*/},
        0x01 => {//LXI B,word
            state.c = next_bytes[0];
            state.b = next_bytes[1];
            state.pc += 2;
        },
        0x02 => { // STAX B
            let bc = (state.b as u16) << 8 | state.c as u16;
            state.memory.write_byte(bc, state.a);
        },        
        0x03 => { // INX B
            let bc = (state.b as u16) << 8 | state.c as u16;
            let result = bc.wrapping_add(1);
            state.b = ((result & 0xFF00) >> 8) as u8;
            state.c = (result & 0x00FF) as u8;
        },
        0x04 => { // INR B
            state.b = state.b.wrapping_add(1);
            update_state(state, state.b, false, 0b0111);
        },
        0x05 => {
            let (answer,carry) = state.b.overflowing_sub(1);
            update_state(state, answer, carry, 0b0111);
            state.b=answer;
        }
        0x06 => {
            state.b = next_bytes[0];
            state.pc+=1;
        }//MVI B, D8
        0x07 => { // RLC
            let carry = (state.a & 0x80) >> 7; // bit 0
            state.a = (state.a << 1) | carry;
            state.cc.cy = carry;
        },

        0x09 => {
            let hl = (state.h as u16) << 8 | state.l as u16;
            let bc = (state.b as u16) << 8 | state.c as u16;
            let (result,carry) = hl.overflowing_add(bc); // Perform the addition, allowing overflow
            state.h = ((result & 0xff00) >> 8) as u8;    // Store the high byte of the result in H
            state.l = result as u8;           // Store the low byte of the result in L

            update_state(state, 0, carry, 0b1000); //set carry only
        }
        0x0A => { // LDAX B
            let bc = (state.b as u16) << 8 | state.c as u16;
            state.a = state.memory.read_byte(bc);
        },
        0x0B => { //DCR C
            let result = state.b.wrapping_sub(1); // Decrement the value of register C
            state.b=result;
        }
        0x0C => { // INR C
            let result = state.c.wrapping_add(1);
            state.c = result;
            update_state(state, result, false, 0b0111);
        },
        0x0D => { //DCR C
            let result = state.c.wrapping_sub(1); // Decrement the value of register C
            state.c=result;

            update_state(state, result, false, 0b0111);
        }
        0x0E => { //MVI C, D8
            state.c = next_bytes[0];
            state.pc+=1;
        }
        0x0F => { // RRC
            let cy = state.a & 0x01; // carry bit
            state.a = (state.a >> 1) | (cy << 7);
            state.cc.cy = cy as u8;
        }

        0x11 => {//LXI D,word
            state.e = next_bytes[0];
            state.d = next_bytes[1];
            state.pc += 2;
        }
        0x12 => { // STAX D
            let de = (state.d as u16) << 8 | state.e as u16;
            state.memory.write_byte(de, state.a);
        },
        0x13 => {//INX D
            let cy;
            (state.e, cy) = state.e.overflowing_add(1);
            state.d = state.d.wrapping_add(cy as u8);
        }
        0x14 => {
            let (answer,carry) = state.d.overflowing_add(1);
            update_state(state, answer, carry, 0b0111);
            state.d=answer;
        }
        0x15 => { // DCR D
            let result = state.d.wrapping_sub(1);
        
            update_state(state, result, false, 0b0111);
        
            state.d = result;
        },
        0x16 => { // MVI D, D8
            state.d = next_bytes[0];
            state.pc += 1;
        },

        0x19 => {
            let hl: u16 = u16::from(state.h) << 8 | u16::from(state.l);
            let de: u16 = u16::from(state.d) << 8 | u16::from(state.e);

            let (result,carry) = hl.overflowing_add(de);
            state.h = ((result & 0xFF00) >> 8) as u8;
            state.l = (result & 0x00FF) as u8;

            state.cc.cy = carry as u8;
        }

        0x1A => { //LDAX D
            let de = (state.d as u16) << 8 | state.e as u16;
            state.a = state.memory.read_byte(de);
        }
        0x1B => { // DCX D
            let de = (state.d as u16) << 8 | state.e as u16;
            let result = de.wrapping_sub(1);
            state.d = (result >> 8) as u8;
            state.e = result as u8;
        },

        
        0x1F => { // RAR
            let carry = state.a & 0x01;
            state.a = (state.cc.cy << 7) | (state.a >> 1);
            state.cc.cy = carry as u8;
        },
        

        0x21 => {//LXI H,word
            state.l = next_bytes[0];
            state.h = next_bytes[1];
            state.pc += 2;
        },
        0x22 => { // SHLD adr
            let address = (next_bytes[1] as u16) << 8 | (next_bytes[0] as u16);
            let l = state.l;
            let h = state.h;
            state.memory.write_byte(address, l);
            state.memory.write_byte(address + 1, h);
            state.pc += 2;
        },
        
        0x23 => { //INX H
            let cy;
            (state.l, cy) = state.l.overflowing_add(1);
            state.h = state.h.wrapping_add(cy as u8);
        }

        0x26 => { //MVI H, D8
            state.h = next_bytes[0];
            state.pc+=1;
        }
        0x27 => { // DAA
            let mut adjust = 0;
            let mut carry = false;
        
            // Check if lower nibble of accumulator is greater than 9 or Auxiliary Carry Flag is set.
            if state.a & 0x0F > 9 || state.cc.ac==1 {
                adjust |= 0x06;
            }
        
            // Adds adjust value to Accumulator
            let old_a = state.a;
            state.a = state.a.wrapping_add(adjust);
            
            // Check if accumulator is greater than 9 or Carry Flag is set after eight-bit binary addition
            if (old_a > 0x99) || state.cc.cy==1 {
                adjust |= 0x60;
                carry = true;
            }
             
            // After addition update Sign Flag
            if (state.a & 0x80) != 0 {
                state.cc.s = 1;
            }
        
            // Adjust accumulator again if old_a > 0x99 or Carry Flag was set
            if carry {
                state.a = state.a.wrapping_add(adjust);
            }
        
            update_state(state, state.a, carry, 0b1111);
        },

        0x29 => { //DAD H
            let hl: u16 = u16::from(state.h) << 8 | u16::from(state.l);

            let (result,carry) = hl.overflowing_add(hl);
            state.h = ((result & 0xFF00) >> 8) as u8;
            state.l = (result & 0x00FF) as u8;

            state.cc.cy = carry as u8;
        }
        0x2A => { // LHLD adr
            let address = (next_bytes[1] as u16) << 8 | next_bytes[0] as u16;
            state.l = state.memory.read_byte(address);
            state.h = state.memory.read_byte(address + 1);
            state.pc += 2;
        },
        0x2B => { // DCX H
            let hl = (state.h as u16) << 8 | state.l as u16;
            let result = hl.wrapping_sub(1);
            state.h = (result >> 8) as u8;
            state.l = result as u8;
        },
        0x2C => { //INR L
            state.l = state.l.wrapping_add(1);
            update_state(state, state.l, false, 0b0111);
        }

        0x2E => { // MVI L, D8
            state.l = next_bytes[0];
            state.pc += 1;
        },
        0x2F => { // CMA
            state.a = !state.a;
        },

        0x31 => {
            let d16 = ((next_bytes[1] as u16) << 8) | (next_bytes[0] as u16);
            state.sp = d16;
            state.pc+=2;
        }
        0x32 => { // STA adr
            let address = (next_bytes[1] as u16) << 8  | next_bytes[0] as u16;
            state.memory.write_byte(address, state.a);
            state.pc += 2;
        }

        0x34 => { // INR M
            let hl = (state.h as u16) << 8 | state.l as u16;
            let value = state.memory.read_byte(hl);
            let result = value.wrapping_add(1);
            state.memory.write_byte(hl, result);
            update_state(state, result, false, 0b0111);
        },
        
        0x35 => { // DCR M
            let address = (state.h as u16) << 8 | state.l as u16;
            let value = state.memory.read_byte(address);
            let result = value.wrapping_sub(1);
            state.memory.write_byte(address, result);
        
            update_state(state, result, false, 0b0111);
        }
        0x36 => {
            let hl16 = ((state.h as u16) << 8) | (state.l as u16);
            state.memory.write_byte(hl16, next_bytes[0]);
            state.pc+=1;
        } //MVI M, D8
        0x37 => { // STC
            state.cc.cy = 1u8;
        },

        0x39 => {
            let hl = ((state.h as u16) << 8) | (state.l as u16);
            let (result,carry) = hl.overflowing_add(state.sp); // TIL this is a thing in rust, sweet

            state.h = ((result & 0xFF00) >> 8) as u8;
            state.l = (result & 0xFF) as u8;

            state.cc.cy = carry as u8;
        }
        0x3A => {
            let address = (next_bytes[1] as u16) << 8  | next_bytes[0] as u16;
            state.a = state.memory.read_byte(address);
            state.pc += 2;
        }

        0x3c => { // INR A
            state.a = state.a.wrapping_add(1);
            update_state(state, state.a, false, 0b0111);
        },
        0x3D => { // DCR A
            state.a = state.a.wrapping_sub(1);
            update_state(state, state.a, false, 0b0111);
        },
        0x3E => { // MVI A, D8
            state.a = next_bytes[0];
            state.pc += 1;
        }


        0x40 => {state.b = state.b},//MOV B,B
        0x41 => {state.b = state.c},//MOV B,C
        0x42 => {state.b = state.d},//MOV B,D
        0x43 => {state.b = state.e},//MOV B,E
        0x44 => {state.b = state.h},//MOV B,H
        0x45 => {state.b = state.l},//MOV B,L
        0x46 => {state.b = state.memory.read_byte(((state.h as u16) << 8) | (state.l as u16))},//MOV B,M
        0x47 => {state.b = state.a},//MOV B,A
        0x48 => {state.c = state.b},//MOV C,B
        0x49 => {state.c = state.c},//MOV C,C
        0x4A => {state.c = state.d},//MOV C,H
        0x4B => {state.c = state.e},//MOV C,H
        0x4C => {state.c = state.h},//MOV C,H
        0x4D => {state.c = state.l},//MOV C,H
        0x4E => {state.c = state.memory.read_byte(((state.h as u16) << 8) | (state.l as u16))},//MOV C,H
        0x4F => {state.c = state.a},//MOV C,H
        0x50 => {state.d = state.b},//MOV C,D
        0x51 => {state.d = state.c},//MOV D,E
        0x52 => {state.d = state.d},//MOV C,H
        0x53 => {state.d = state.e},//MOV C,L
        0x54 => {state.d = state.h},//MOV C,M
        0x55 => {state.d = state.l},//MOV C,A
        0x56 => {
            let hl = ((state.h as u16) << 8) | (state.l as u16);
            state.d = state.memory.read_byte(hl);
        }
        0x57 => {state.d = state.a},//MOV D,A

        0x59 => {state.e = state.c},//MOV D,A

        0x5E => {
            let hl = ((state.h as u16) << 8) | (state.l as u16);
            state.e = state.memory.read_byte(hl);
        }
        0x5F => {state.e = state.a},//MOV E,A

        0x61 => {state.h = state.c},//MOV H,C  

        0x65 => {state.h = state.l},//MOV H,C  
        0x66 => {
            let hl = ((state.h as u16) << 8) | (state.l as u16);
            state.h = state.memory.read_byte(hl);
        }
        0x67 => {state.h = state.a},//MOV H,A
        0x68 => {state.l = state.b},//MOV L,B  
        0x69 => {state.l = state.c},//MOV L,A   

        0x6E => { //MOV L,M
            let hl = ((state.h as u16) << 8) | (state.l as u16);
            state.l = state.memory.read_byte(hl);
        }
        0x6F => {state.l = state.a},//MOV L,A       
        0x70 => {
            let hl = (state.h as u16) << 8 | state.l as u16;
            state.memory.write_byte(hl, state.b);
        },//MOV M, A
        0x71 => {
            let hl = (state.h as u16) << 8 | state.l as u16;
            state.memory.write_byte(hl, state.c);
        },//MOV M, A
        0x72 => {
            let hl = (state.h as u16) << 8 | state.l as u16;
            state.memory.write_byte(hl, state.d);
        },//MOV M, A
        0x73 => {
            let hl = (state.h as u16) << 8 | state.l as u16;
            state.memory.write_byte(hl, state.e);
        },//MOV M, A
        0x74 => {
            let hl = (state.h as u16) << 8 | state.l as u16;
            state.memory.write_byte(hl, state.h);
        },//MOV M, A
        0x75 => {
            let hl = (state.h as u16) << 8 | state.l as u16;
            state.memory.write_byte(hl, state.l);
        },//MOV M, A

        0x77 => {
            let hl = (state.h as u16) << 8 | state.l as u16;
            state.memory.write_byte(hl, state.a);
        },//MOV M, A
        0x78 => {state.a = state.b},//MOV A,B
        0x79 => {state.a = state.c},//MOV A,C
        0x7A => {state.a = state.d},//MOV A,D
        0x7B => {state.a = state.e},//MOV A,E
        0x7C => {state.a = state.h},//MOV A,H
        0x7D => {state.a = state.l},//MOV A,L
        0x7E => {
            let hl = (state.h as u16) << 8 | state.l as u16;
            state.a=state.memory.read_byte(hl);
        },//MOV A,M
        0x7F => {state.a = state.a},//MOV A,A

        //register form addition
        0x80 => {//ADD B
            let (answer,carry) = state.a.overflowing_add(state.b );
            update_state(state, answer, carry, 0b1111);
            state.a = answer;
        }
        0x81 => {//ADD C
            let (answer,carry) = state.a.overflowing_add(state.c );
            update_state(state, answer, carry, 0b1111);
            state.a = answer;
        }
        0x82 => {//ADD D
            let (answer,carry) = state.a.overflowing_add(state.d );
            update_state(state, answer, carry, 0b1111);
            state.a = answer;
        }
        0x83 => {//ADD E
            let (answer,carry) = state.a.overflowing_add(state.e );
            update_state(state, answer, carry, 0b1111);
            state.a = answer;
        }

        0x85 => { // ADD L
            let (result, carry) = state.a.overflowing_add(state.l);
            state.a = result;
            update_state(state, result, carry, 0b1111);
        },
        0x86 => { // ADD M
            let hl = (state.h as u16) << 8 | state.l as u16;
            let value = state.memory.read_byte(hl);
        
            let (result, carry) = state.a.overflowing_add(value);
        
            update_state(state, result, carry, 0b1111);
        
            state.a = result;
        },

        0x8A => { // ADC D        
            let (mut result, mut carry) = state.a.overflowing_add(state.d);
            (result,carry) = result.overflowing_add(state.cc.cy);
        
            update_state(state, result, carry, 0b1111);
        
            state.a = result;
        },

        0x8E => { // ADC M
            let hl = (state.h as u16) << 8 | state.l as u16;
            let value = state.memory.read_byte(hl);
        
            let (mut result, mut carry) = state.a.overflowing_add(value);
            (result,carry) = result.overflowing_add(state.cc.cy);
        
            update_state(state, result, carry, 0b1111);
        
            state.a = result;
        },

        0x91 => {// SUB C
            let (value,carry) = state.a.overflowing_sub(state.c);
            update_state(state, value, carry, 0b1111);
            state.a = value;
        }

        0x97 => {// SUB A
            let (value,carry) = state.a.overflowing_sub(state.a);
            update_state(state, value, carry, 0b1111);
            state.a = value;
        }

        0x9E => { // SBB M
            let hl = (state.h as u16) << 8 | state.l as u16;
            let value = state.memory.read_byte(hl);
        
            // Cast operands to i16
            let res = state.a as i16 - value as i16 - state.cc.cy as i16;
        
            // Carry occurs if result is negative 
            let cy = res > 0xff;
        
            // Perform wrapping manually
            state.a = res as u8;
        
            update_state(state, state.a, cy, 0b1111);
        },

        0xA0 => { // ANA B
            state.a &= state.b;
            update_state(state, state.a, false, 0b1111);
        },

        0xA6 => { // ANA M
            let hl = (state.h as u16) << 8 | state.l as u16;
            let value = state.memory.read_byte(hl);
            let result = state.a & value;
            state.a = result;
            update_state(state, result, false, 0b1111);
        },
        0xA7 => { // ANA A
            let result = state.a & state.a;
            state.a = result;
            update_state(state, result, false, 0b1111);
        },
        0xA8 => { // XRA B
            state.a ^= state.b;
            update_state(state, state.a, false, 0b1111);
        },

        0xAF => { // XRA A
            state.a ^= state.a;
            update_state(state, state.a, false, 0b1111);
        },
        0xB0 => { // ORA B
            state.a |= state.b;
            update_state(state, state.a, false, 0b1111);
        },
        0xB1 => { // ORA C
            state.a |= state.c;
            update_state(state, state.a, false, 0b1111);
        },

        0xB4 => { // ORA H
            state.a |= state.h;
            update_state(state, state.a, false, 0b1111);
        },
        

        0xB6 => { // ORA M
            let address = (state.h as u16) << 8 | state.l as u16;
            let value = state.memory.read_byte(address);
            state.a |= value;
            update_state(state, state.a, false, 0b1111);
        },

        0xB8 => { // CMP B
            let (result,carry) = state.a.overflowing_sub(state.b);
            update_state(state, result, carry, 0b1111);
        },

        0xBC => { // CMP B
            let (result,carry) = state.a.overflowing_sub(state.h);
            update_state(state, result, carry, 0b1111);
        },

        0xBE => { // CMP B
            let value = state.memory.read_byte((state.h as u16) << 8 | state.l as u16);
            let (result,carry) = state.a.overflowing_sub(value);
            update_state(state, result, carry, 0b1111);
        },

        0xC0 => { // RNZ
            if state.cc.z == 0 {
                let return_address = state.memory.read_byte(state.sp) as u16 | (state.memory.read_byte(state.sp + 1) as u16) << 8;
                state.sp += 2;
                state.pc = return_address;
            } 
        },
        0xC1 => { // POP B
            state.c = state.memory.read_byte(state.sp);
            state.b = state.memory.read_byte(state.sp + 1);
            state.sp += 2;
        }
        0xC2=> {//JNZ
            if state.cc.z == 0 {
                state.pc = ((next_bytes[1] as u16) << 8) | next_bytes[0] as u16;
            } else {
                state.pc += 2;
            }
        }
        0xC3 => {//JMP
            state.pc = ((next_bytes[1] as u16) << 8) | next_bytes[0] as u16;
        }
        0xC4 => { // CNZ adr
            if state.cc.z == 0 {
                let ret = state.pc + 2;
                state.memory.write_byte(state.sp - 1, (ret >> 8) as u8);
                state.memory.write_byte(state.sp - 2, ret as u8);
                state.sp -= 2;
                state.pc = (next_bytes[1] as u16) << 8 | next_bytes[0] as u16;
            } else {
                state.pc += 2;
            }
        },
        0xC5 => { // PUSH B
            state.memory.write_byte(state.sp - 1, state.b);
            state.memory.write_byte(state.sp - 2, state.c);
            state.sp -= 2;
        }
        0xC6 => { // ADI D8
            let data = next_bytes[0];
            let (result, carry) = state.a.overflowing_add(data);
            state.a = result;
            update_state(state, state.a, carry, 0b1111); // Update flags Z, S, P, CY
            state.pc += 1;
        }

        0xC8 => { // RZ
            if state.cc.z != 0 {
                let pc_low = state.memory.read_byte(state.sp) as u16;
                let pc_high = state.memory.read_byte(state.sp + 1) as u16;
                state.pc = (pc_high << 8) | pc_low;
                state.sp += 2;
            }
        },
        0xC9 => { // RET
            state.pc = state.memory.read_byte(state.sp) as u16 | (state.memory.read_byte(state.sp + 1) as u16) << 8;
            state.sp += 2;
        }
        0xCA => { // JZ adr
            if state.cc.z != 0 {
                let address = (next_bytes[1] as u16) << 8 | next_bytes[0] as u16;
                state.pc = address;
            } else {
                state.pc += 2;
            }
        },

        0xCC => { // CZ adr
            if state.cc.z != 0 {
                let return_address = state.pc + 2;
                let address = (next_bytes[1] as u16) << 8 | next_bytes[0] as u16;
                state.memory.write_byte(state.sp - 1, ((return_address >> 8) & 0xFF) as u8);
                state.memory.write_byte(state.sp - 2, ((return_address) & 0xFF) as u8);
                state.sp -= 2;
                state.pc = address;
            } else {
                state.pc += 2;
            }
        },
        0xCD => { // CALL address
            let ret = state.pc + 2;
            state.memory.write_byte(state.sp - 1, (ret >> 8) as u8);
            state.memory.write_byte(state.sp - 2, ret as u8);
            state.sp -= 2;
            state.pc = (next_bytes[1] as u16) << 8 | next_bytes[0] as u16;
        }



        0xD0 => { // RNC
            if state.cc.cy == 0 {
                // NCY, return
                let ret = (state.memory.read_byte(state.sp + 1) as u16) << 8 | state.memory.read_byte(state.sp) as u16;
                state.sp += 2;
                state.pc = ret;
            } 
        }
        0xD1 => { //POP D
            state.e = state.memory.read_byte(state.sp);
            state.d = state.memory.read_byte(state.sp + 1);
            state.sp += 2;
        }
        0xD2 => { // JNC adr
            if state.cc.cy == 0 {
                let address = (next_bytes[1] as u16) << 8 | (next_bytes[0] as u16);
                state.pc = address;
            } else {
                state.pc += 2;
            }
        },
        0xD3 => {//OUT D8
            //OUT D8
            let port = next_bytes[0];
            //println!("out");
            machine_out(state, port);
            state.pc += 1;
        }
        0xD4 => { // CNC adr
            if state.cc.cy == 0 {
                let ret = state.pc + 2;
                state.memory.write_byte(state.sp - 1, (ret >> 8) as u8);
                state.memory.write_byte(state.sp - 2, ret as u8);
                state.sp -= 2;
                state.pc = (next_bytes[1] as u16) << 8 | next_bytes[0] as u16;
            } else {
                state.pc += 2;
            }
        },
        0xD5 => { //PUSH D
            state.memory.write_byte(state.sp - 1, state.d);
            state.memory.write_byte(state.sp - 2, state.e);
            state.sp = state.sp.wrapping_sub(2);
        }
        0xD6 => { // SUI D8
            let data = next_bytes[0];
            let (result, carry) = state.a.overflowing_sub(data);
            state.a = result;
            update_state(state, result, carry, 0b1111);
            state.pc += 1;
        },

        0xD8 => { // RC
            if state.cc.cy == 1 {
                let low_byte = state.memory.read_byte(state.sp) as u16;
                let high_byte = state.memory.read_byte(state.sp + 1) as u16;
                state.sp += 2;
                state.pc = (high_byte << 8) | low_byte;
            } 
        },

        0xDA => { // JC adr
            if state.cc.cy != 0 {
                let address = (next_bytes[1] as u16) << 8 | next_bytes[0] as u16;
                state.pc = address;
            } else {
                state.pc += 2;
            }
        },
        0xDB => {
            // IN D8
            let port = next_bytes[0];
            state.a = machine_in(state, port);
            state.pc += 1; // Skip over the data byte
        }

        0xDE => { // SBB D8
            let value = next_bytes[0];
        
            // Cast operands to i16
            let res = state.a as i16 - value as i16 - state.cc.cy as i16;
        
            // Carry occurs if result is negative 
            let cy = res > 0xff;
        
            // Perform wrapping manually
            state.a = res as u8;
        
            update_state(state, state.a, cy, 0b1111);
            state.pc+=1;
        },

        0xE1 => { //POP H
            state.l = state.memory.read_byte(state.sp);
            state.h = state.memory.read_byte(state.sp + 1);
            state.sp += 2;
        }

        0xE3 => { // XTHL
            let sp = (state.sp as u16);
            let l = state.memory.read_byte(sp) as u16;
            let h = state.memory.read_byte(sp + 1) as u16;
            state.memory.write_byte(sp, state.l);
            state.memory.write_byte(sp + 1, state.h);
            state.l = l as u8;
            state.h = h as u8;
        },

        0xE5 => { //PUSH H
            state.memory.write_byte(state.sp - 1, state.h);
            state.memory.write_byte(state.sp - 2, state.l);
            state.sp = state.sp.wrapping_sub(2);
        }
        0xE6 => { // ANI D8
            let data = next_bytes[0];
            state.a = state.a & data;
            let carry = false; // carry is cleared 
            update_state(state, state.a, carry, 0b1111); // Update flags Z, S, P
            state.pc += 1;
        }

        0xE9 => { // PCHL
            state.pc = (state.h as u16) << 8 | state.l as u16;
        },

        0xEB => { //XCHG
            std::mem::swap(&mut state.h, &mut state.d);
            std::mem::swap(&mut state.l, &mut state.e);
        } 
        0xEC => { // CPE adr
            if state.cc.p == 1 {
                let ret = state.pc+2;
                let address = (next_bytes[1] as u16) << 8 | next_bytes[0] as u16 ;
                state.memory.write_byte(state.sp - 1, (ret >> 8) as u8);
                state.memory.write_byte(state.sp - 2, ret as u8);
                state.sp -= 2;
                state.pc = address;
            } else {
                state.pc += 2;
            }
        },  
        
        0xF1 => { // POP PSW
            state.a = state.memory.read_byte(state.sp + 1);
            let psw = state.memory.read_byte(state.sp);
            state.cc.z = (0x01 == (psw & 0x01)) as u8;
            state.cc.s = (0x02 == (psw & 0x02)) as u8;
            state.cc.p = (0x04 == (psw & 0x04)) as u8;
            state.cc.cy = (0x08 == (psw & 0x08)) as u8;
            state.cc.ac = (0x10 == (psw & 0x10)) as u8;
            state.sp += 2;
        }

        0xF3 => { // DI
            state.int_enable = 0u8;
        },

        0xF5 => { // PUSH PSW
            state.memory.write_byte(state.sp - 1, state.a);
            let psw = (state.cc.z as u8)
                | (state.cc.s as u8) << 1
                | (state.cc.p as u8) << 2
                | (state.cc.cy as u8) << 3
                | (state.cc.ac as u8) << 4;
            state.memory.write_byte(state.sp - 2, psw);
            state.sp -= 2;
        }
        0xF6 => { // ORI D8
            let data = next_bytes[0];
            state.a |= data;
            update_state(state, state.a, false, 0b1111);
            state.pc += 1;
        },
        
        0xFA => { // JM adr
            if state.cc.s != 0 {
                state.pc = (next_bytes[1] as u16) << 8 | next_bytes[0] as u16;
            } else {
                state.pc += 2;
            }
        },
        0xFB => { // EI
            state.int_enable = 1u8;
        },

        0xFE => { // CPI D8
            let (result,carry) = state.a.overflowing_sub(next_bytes[0]);
            update_state(state, result, carry, 0b1111);
            state.pc+=1;
        }
        0xFF => { // RST 7
            let ret = state.pc;
            state.memory.write_byte(state.sp - 1, (ret >> 8) as u8);
            state.memory.write_byte(state.sp - 2, ret as u8);
            state.sp -= 2;
            state.pc = 0x38;
        },

        _ => unimplemented_instruction(opcode,state), // Default case for unknown opcodes
    }

    CYCLES_8080[opcode as usize]

}


// Utility code
pub fn print_state(state: &State8080) {
    let (_,inst) = process_instruction(state.read_mem(state.pc), &[state.read_mem(state.pc+1),state.read_mem(state.pc+2)]);
    println!("=== State8080 ===");
    println!("A: 0x{:02X}   B: 0x{:02X}   C: 0x{:02X}", state.a, state.b, state.c);
    println!("D: 0x{:02X}   E: 0x{:02X}   H: 0x{:02X}   L: 0x{:02X}", state.d, state.e, state.h, state.l);
    println!("SP: 0x{:04X}   PC: 0x{:04X}", state.sp, state.pc);
    println!("CC - Z: {}  S: {}  P: {}  CY: {}  AC: {}  PAD: {}",
             state.cc.z, state.cc.s, state.cc.p, state.cc.cy, state.cc.ac, state.cc.pad);
    println!("Interrupt Enable: {}", state.int_enable);
    println!("Opcode: {:02X}", state.read_mem(state.pc));
    println!("Instruction: {}",inst);
    println!("=================");
}

fn parity(value: u8) -> bool {
    let mut bits: u8 = 0;
        for i in 0..8 {
            bits += (value >> i) & 1;
        }
        (bits & 1) == 0
}

fn unimplemented_instruction(opcode: u8,_state: &mut State8080) {
    println!("Error: Unimplemented instruction:");
    println!("{:02X}",opcode);
    println!("State:");
    print_state(_state);
    std::process::exit(1);
}

fn update_state(state: &mut State8080, value: u8, carry: bool, flags_to_set: u8) {
    
    if flags_to_set & 0b0001 != 0 {
        state.cc.z = (value == 0) as u8;
    }
    
    if flags_to_set & 0b0010 != 0 {
        state.cc.s = ((value & 0x80) != 0) as u8;
    }
    
    if flags_to_set & 0b0100 != 0 {
        state.cc.p = parity(value) as u8;
    }
    
    if flags_to_set & 0b1000 != 0 {
        if carry {
            state.cc.cy = 1u8;
        }
        else {
            state.cc.cy = 0u8;
        }
    }
}

fn machine_out(state: &mut State8080, port: u8) {
    match port {
        
        2 => {
            state.port.write2 = state.a & 0x7;
        }
        4 => {
            state.port.shift0 = state.port.shift1;
            state.port.shift1 = state.a;
        }
        _ => {}
    }
}

fn machine_in(state: &mut State8080, port: u8) -> u8 {
    let a: u8;
    match port {
        1 => { a = *state.port.io_ports.get(&1).unwrap_or(&0)}
        3 => {
            let v: u16 = ((state.port.shift1 as u16) << 8) | (state.port.shift0 as u16);
            a = ((v >> (8 - state.port.write2)) & 0xFF) as u8;
        }
        _ => {
            // Handle other ports if needed
            // Set a default value for 'a'
            a=0;
        }
    }
    a
}

pub fn generate_interrupt(state: &mut State8080, interrupt_num: u8) {
    // Perform "PUSH PC"
    push(state, (state.pc >> 8) as u8, (state.pc & 0xFF) as u8);
    
    // Set the PC to the low memory vector.
    // This is identical to an "RST interrupt_num" instruction.
    state.pc = (8 * interrupt_num) as u16;
    // Disable interrupts
    state.int_enable = 0u8;
}

fn push(state: &mut State8080, high_byte: u8, low_byte: u8) {
    state.memory.write_byte(state.sp - 1, high_byte);
    state.memory.write_byte(state.sp - 2, low_byte);

    state.sp -= 2;
}