use crate::Instruction;
use crate::instruction_fetcher::Gameboy;
use crate::register::{ByteRegister, FlagRegister, WordRegister, RegisterId};
use crate::instruction::Instruction::*;
use crate::register::WordRegister::StackPointer;
use std::cmp::{min, max};
use crate::interrupt::InterruptId::{VBlank, STAT, Timer, Serial, Joypad};
use crate::interrupt::InterruptState;
use InterruptState::Active;

#[deny(unreachable_patterns)]
pub fn execute_instruction(gb: &mut Gameboy, (op, instruction): (u8, Instruction)) -> u8 {
    println!("op: {} | pc: {} | sp: {} | a: {} b: {} c: {} d: {} e: {} h: {} l: {} | f: {}", op, gb.pc.0 + 1, gb.sp.to_address(), gb.a.0, gb.b.0, gb.c.0, gb.d.0, gb.e.0, gb.h.0, gb.l.0, gb.f.value());
    if gb.pc.0 == 10682 {
       print!("")
    }
    gb.pc.0 += instruction.size() as u16;
    let hl = gb.hl();
    let mut condition = true;
    match instruction {
        NOP => {}

        ADD_A_R8(ByteRegister(n, _)) | ADD_A_N8(n) => {
            let (add, carry) = calc_with_carry(vec![gb.a.0, n], &mut 0, |a, b| a.overflowing_add(b));
            gb.set_flags(add == 0, false, half_carry_8_add(gb.a.0, n, 0), carry);
            gb.a.0 = add;
        }
        ADC_A_R8(ByteRegister(n, _)) | ADC_A_N8(n) => {
            let carry = if gb.f.c { 1 } else { 0 };
            let (add, new_carry) = calc_with_carry(vec![gb.a.0, n, carry], &mut 0, |a, b| a.overflowing_add(b));
            gb.set_flags(add == 0, false, half_carry_8_add(gb.a.0, n, carry), new_carry);
            gb.a.0 = add;
        }
        ADC_A_HL | ADD_A_HL => {
            let carry = if let ADD_A_HL = instruction { 1 } else { if gb.f.c { 1 } else { 0 } };
            let (add, new_carry) = calc_with_carry(vec![gb.a.0, gb.mem[hl], carry], &mut 0, |a, b| a.overflowing_add(b));
            gb.set_flags(add == 0, false, half_carry_8_add(gb.a.0, gb.mem[hl], carry), new_carry);
            gb.a.0 = add;
        }
        AND_A_R8(ByteRegister(n, _)) | AND_A_N8(n) => {
            gb.a.0 &= n;
            gb.set_flags(gb.a.0 == 0, false, true, false);
        }
        AND_A_HL => {
            gb.a.0 &= gb.mem[hl];
            gb.set_flags(gb.a.0 == 0, false, true, false);
        }
        CP_A_R8(ByteRegister(n, _)) | CP_A_N8(n) =>
            gb.set_flags(gb.a.0 == n, true, half_carry_8_sub(gb.a.0, n, 0), n > gb.a.0),
        CP_A_HL => {
            let n = gb.mem[hl];
            gb.set_flags(gb.a.0 == n, true, half_carry_8_sub(gb.a.0, n, 0), n > gb.a.0);
        }

        DEC_R8(ByteRegister(_, id)) => {
            let reg = gb.get_register(id).0;
            gb.get_register(id).0 = reg.wrapping_sub(1);
            let z = gb.get_register(id).0 == 0;
            gb.set_flags(z, true, half_carry_8_sub(reg, 1, 0), gb.f.c);
        }

        INC_R8(ByteRegister(_, id)) => {
            let reg = gb.get_register(id).0;
            gb.get_register(id).0 = reg.wrapping_add(1);
            let z = gb.get_register(id).0 == 0;
            let hc = half_carry_8_add(reg, 1, 0);
            gb.set_flags(z, false, hc, gb.f.c);
        }
        OR_A_R8(ByteRegister(n, _)) | OR_A_N8(n) => {
            gb.a.0 |= n;
            gb.set_flags(gb.a.0 == 0, false, false, false);
        }
        OR_A_HL => {
            gb.a.0 |= gb.mem[hl];
            gb.set_flags(gb.a.0 == 0, false, false, false);
        }
        SUB_A_R8(ByteRegister(n, _)) | SUB_A_N8(n) => {
            let (sub, c) = calc_with_carry(vec![gb.a.0, n], &mut 0, |a, b| a.overflowing_sub(b));
            gb.a.0 = sub;
            gb.set_flags(gb.a.0 == 0, true, half_carry_8_sub(gb.a.0, n, 0), c);
        }
        SBC_A_R8(ByteRegister(n, _)) | SBC_A_N8(n) => {
            let carry = if gb.f.c { 1 } else { 0 };
            let (sub, new_carry) = calc_with_carry(vec![gb.a.0, n, carry], &mut 0, |a, b| a.overflowing_sub(b));
            gb.a.0 = sub;
            gb.set_flags(gb.a.0 == 0, true, half_carry_8_sub(gb.a.0, n, carry), new_carry);
        }
        SBC_A_HL | SUB_A_HL => {
            let carry = if let SUB_A_HL = instruction { 1 } else { if gb.f.c { 1 } else { 0 } };
            let (sub, new_carry) = calc_with_carry(vec![gb.a.0, gb.mem[hl], carry], &mut 0, |a, b| a.overflowing_sub(b));
            gb.a.0 = sub;
            gb.set_flags(gb.a.0 == 0, true, half_carry_8_sub(gb.a.0, gb.mem[hl], carry), new_carry);
        }
        XOR_A_R8(ByteRegister(n, _)) | XOR_A_N8(n) => {
            gb.a.0 ^= n;
            gb.set_flags(gb.a.0 == 0, false, false, false);
        }
        XOR_A_HL => {
            gb.a.0 ^= gb.mem[hl];
            gb.set_flags(gb.a.0 == 0, false, false, false);
        }
        ADD_HL_R16(reg) => {
            let hc = half_carry_16_add(gb.hl().to_address(), reg.to_address(), 0);
            let (hl, carry) = gb.hl().to_address().overflowing_add(reg.to_address());
            gb.set_word_register(hl, gb.hl());
            gb.set_flags(gb.f.z, false, hc, carry);
        }
        DECH_HL => {
            let old = gb.mem[hl];
            gb.mem *= (hl, old.wrapping_sub(1));
            let hc = half_carry_8_sub(old, 1, 0);
            gb.set_flags(gb.mem[hl] == 0, true, hc, gb.f.c);
        },
        INCH_HL => {
            let old = gb.mem[hl];
            gb.mem *= (hl, old.wrapping_sub(1));
            let hc = half_carry_8_add(old, 1, 0);
            gb.set_flags(gb.mem[hl] == 0, false, hc, gb.f.c);
        },
        DEC_R16(reg) => gb.set_word_register(reg.to_address().wrapping_sub(1), reg),
        INC_R16(reg) => gb.set_word_register(reg.to_address().wrapping_add(1), reg),
        BIT_U3_R8(_, _) | BIT_U3_HL(_) | RES_U3_R8(_, _) |
        RES_U3_HL(_) | SET_U3_R8(_, _) | SET_U3_HL(_) => {
            match instruction {
                BIT_U3_R8(bit, ByteRegister(n, _)) => gb.f.z = n & bit.0 == 0,
                BIT_U3_HL(bit) => gb.f.z = gb.mem[hl] & bit.0 == 0,
                RES_U3_R8(bit, ByteRegister(_, id)) => {
                    gb.get_register(id).0 &= !bit.0
                },
                RES_U3_HL(bit) => {
                    let hl = gb.hl().to_address();
                    gb.mem *= (hl, gb.mem[hl] & !bit.0)
                }
                SET_U3_R8(bit, ByteRegister(_, id)) => gb.get_register(id).0 |= bit.0,
                SET_U3_HL(bit) => {
                    let hl = gb.hl().to_address();
                    gb.mem *= (hl, gb.mem[hl] | bit.0)
                }
                _ => panic!()
            };
        }
        SWAP_R8(ByteRegister(n, id)) => {
            gb.set_flags(n == 0, false, false, false);
            gb.get_register(id).0 = n.rotate_left(4);
        }
        SWAP_HL => {
            gb.set_flags(gb.hl().to_address() == 0, false, false, false);
            gb.set_word_register(gb.hl().to_address().rotate_left(8), gb.hl());
        }
        LD_R8_R8(a, b) => gb.get_register(a.1).0 = b.0,
        LD_R8_N8(a, b) => gb.get_register(a.1).0 = b,
        LD_R16_N16(a, b) => gb.set_word_register(b, a),
        LD_HL_R8(ByteRegister(n, _)) | LD_HL_N8(n) => { gb.mem *= (hl, n); }
        LD_R8_HL(a) => gb.get_register(a.1).0 = gb.mem[hl],
        LD_R16_A(n) => gb.mem *= (n, gb.a),
        LDH_N16_A(n) => gb.mem *= (n, gb.a),
        LDH_C_A => gb.mem *= (gb.c, gb.a),
        LD_A_N8(n) => gb.a.0 = n,
        LD_A_R16(n) => gb.a.0 = gb.mem[n],
        LD_A_N16(n) => gb.a.0 = gb.mem[n],
        LDH_A_N8(n) => {
            let x = gb.mem[n];
            gb.a.0 = x;
        },
        LDH_N8_A(n) => {
            if n == 0 {
                let v = (gb.mem[n] & 0xCF) | gb.a.0; // Joypad
                gb.mem *= (n, v)
            } else {
                let x = gb.a.0;
                gb.mem *= (n, x);
            }
        },
        LDH_HL_N8(n) => gb.mem *= (hl, n),
        LDH_A_C => gb.a.0 = gb.mem[gb.c],
        LD_A_HLD => {
            gb.set_word_register(hl.to_address().wrapping_sub(1), gb.hl());
            gb.a.0 = gb.mem[hl];
        }
        LD_HLD_A => {
            gb.set_word_register(hl.to_address().wrapping_sub(1), gb.hl());
            gb.mem *= (hl, gb.a);
        }
        LD_A_HLI => {
            gb.a.0 = gb.mem[hl];
            gb.set_word_register(hl.to_address().wrapping_add(1), gb.hl());
        }
        LD_HLI_A => {
            gb.mem *= (hl, gb.a);
            gb.set_word_register(hl.to_address().wrapping_add(1), gb.hl());
        }
        CALL_N16(n) => {
            let [lo, hi] = gb.pc.0.to_le_bytes();
            gb.sp = StackPointer(gb.sp.to_address() - 1);
            gb.mem *= (gb.sp, hi);
            gb.sp = StackPointer(gb.sp.to_address() - 1);
            gb.mem *= (gb.sp, lo);
            gb.pc.0 = n;
        }

        JP_HL => gb.pc.0 = gb.hl().to_address(),
        JP_N16(n) => gb.pc.0 = n,
        JR_E8(n) => gb.pc.0 = (gb.pc.0 as i16 + n as i16) as u16,
        CPL => {
            gb.a.0 = !gb.a.0;
            gb.set_flags(gb.f.z, true, true, gb.f.c);
        }
        RET => {
            let lo = gb.mem[gb.sp.to_address()];
            let hi = gb.mem[gb.sp.to_address().wrapping_add(1)];
            gb.pc.0 = u16::from_le_bytes([lo, hi]);
            gb.set_word_register(gb.sp.to_address().wrapping_add(2), gb.sp);
        }
        RETI => {
            let lo = gb.mem[gb.sp.to_address()];
            let hi = gb.mem[gb.sp.to_address().wrapping_add(1)];
            gb.pc.0 = u16::from_le_bytes([lo, hi]);
            gb.set_word_register(gb.sp.to_address().wrapping_add(2), gb.sp);
            gb.ime_counter = 1;
        }
        RST(rst_vec) => {
            let [lo, hi] = gb.pc.0.to_le_bytes();
            gb.sp = StackPointer(gb.sp.to_address().wrapping_sub(1));
            gb.mem *= (gb.sp, hi);
            gb.sp = StackPointer(gb.sp.to_address().wrapping_sub(1));
            gb.mem *= (gb.sp, lo);
            gb.pc.0 = rst_vec as u16
        },
        ADD_HL_SP => {
            let (add, carry) = gb.hl().to_address().overflowing_add(gb.sp.to_address());
            gb.set_flags(add == 0, true, half_carry_16_add(gb.hl().to_address(), gb.sp.to_address(), 0), carry);
            gb.set_word_register(add, gb.hl());
        }
        ADD_SP_E8(n) | LD_HL_SP_E8(n) => {
            let (add, carry) = if n < 0 {
                gb.sp.to_address().overflowing_sub((n as u8 & !0x80) as u16)
            } else {
                gb.sp.to_address().overflowing_add((n as u8 & !0x80) as u16)
            };
            let half_carry = if n < 0 {
                half_carry_16_sub(gb.sp.to_address(), (n as u8 & !0x80) as u16, 0)
            } else {
                half_carry_16_add(gb.sp.to_address(), (n as u8 & !0x80) as u16, 0)
            };
            gb.set_flags(false, false, half_carry, carry);
            gb.set_word_register(add, if let ADD_SP_E8(n) = instruction { gb.sp } else { gb.hl() })
        }
        LD_N16_SP(n) => {
            let [lo, hi] = gb.sp.to_address().to_le_bytes();
            gb.mem *= (n, lo);
            gb.mem *= (n+1, hi);
        }
        LD_N8_A(n) => gb.a.0 = n,
        LD_SP_HL => gb.set_word_register(gb.hl().to_address(), gb.sp),
        POP_R16(reg) => {
            match reg {
                WordRegister::Double(ByteRegister(_, high), ByteRegister(_, low)) => {
                    for id in &[low, high] {
                        gb.get_register(*id).0 = gb.mem[gb.sp.to_address()];
                        gb.set_word_register(gb.sp.to_address().wrapping_add(1), gb.sp);
                    }
                },
                WordRegister::AccFlag(mut a, mut f) => {
                    gb.f.set(gb.mem[gb.sp.to_address()]);
                    gb.a.0 = gb.mem[gb.sp.to_address().wrapping_add(1)];
                    gb.set_word_register(gb.sp.to_address().wrapping_add(2), gb.sp);
                }

                _ => panic!()
            }
        }
        PUSH_AF => {
            gb.set_word_register(gb.sp.to_address().wrapping_sub(1), gb.sp);
            gb.mem *= (gb.sp, gb.a);
            gb.set_word_register(gb.sp.to_address().wrapping_sub(1), gb.sp);
            gb.mem *= (gb.sp, gb.f.value());
        }
        PUSH_R16(reg) => {
            match reg {
                WordRegister::Double(ByteRegister(_, high), ByteRegister(_, low)) => {
                    for id in &[high, low] {
                        gb.set_word_register(gb.sp.to_address().wrapping_sub(1), gb.sp);
                        let sp = gb.sp.to_address();
                        let value = gb.get_register(*id).0;
                        gb.mem *= (gb.sp, value);
                    }
                }
                _ => panic!()
            }
        }
        CCF => {
            gb.f.n = false;
            gb.f.h = false;
            gb.f.c = !gb.f.c;
        }
        DAA => {
            // note: assumes a is a uint8_t and wraps from 0xff to 0
            if !gb.f.n {  // after an addition, adjust if (half-)carry occurred or if result is out of bounds
                if gb.f.c || gb.a.0 > 0x99 {
                    gb.a.0 += 0x60;
                    gb.f.c = true;
                }
                if gb.f.h || (gb.a.0 & 0x0f) > 0x09 {
                    gb.a.0 += 0x6;
                }
            } else {
                if gb.f.c { gb.a.0 -= 0x60; }
                if gb.f.h { gb.a.0 -= 0x6; }
            }
            gb.f.z = gb.a.0 == 0;
            gb.f.h = false;
        }
        DI => { gb.ime = false; }
        EI => {
            gb.ime_counter = 2;
        }
        HALT => {}
        SCF => {
            gb.f.n = false;
            gb.f.h = false;
            gb.f.c = true;
        }

        RET_CC(cc) => if gb.cc_flag(cc) {
            let lo = gb.mem[gb.sp.to_address()];
            let hi = gb.mem[gb.sp.to_address().wrapping_add(1)];
            gb.pc.0 = u16::from_le_bytes([lo, hi]);
            gb.set_word_register(gb.sp.to_address().wrapping_add(2), gb.sp);
        } else { condition = false }

        JP_CC_N16(cc, n) => if gb.cc_flag(cc) { gb.pc.0 = n; } else { condition = false }

        JR_CC_E8(cc, n) => if gb.cc_flag(cc) { gb.pc.0 = (gb.pc.0 as i16 + n as i16) as u16; } else { condition = false }

        CALL_CC_N16(cc, n) => if gb.cc_flag(cc) {
            let [lo, hi] = gb.pc.0.to_le_bytes();
            gb.sp = StackPointer(gb.sp.to_address() - 1);
            gb.mem *= (gb.sp, hi);
            gb.sp = StackPointer(gb.sp.to_address() - 1);
            gb.mem *= (gb.sp, lo);
            gb.pc.0 = n;
        } else { condition = false }

        STOP => {}
        _ => panic!()
    };
    instruction.cycles(condition) + if handle_interrupts(gb) { 4 } else { 0 }
}

fn calc_with_carry<T: Copy>(operands: Vec<T>, acc: &mut T, op: fn(T, T) -> (T, bool)) -> (T, bool) {
    let mut c = false;
    for x in operands {
        if !c {
            let res = op(*acc, x);
            *acc = res.0;
            c = res.1;
        } else {
            *acc = op(*acc, x).0
        }
    }
    (*acc, c)
}

fn handle_interrupts(gb: &mut Gameboy) -> bool {
    gb.ime_counter -= 1;
    if gb.ime_counter == 0 {
        gb.ime = true;
    } else {
        gb.ime_counter = max(gb.ime_counter, -1);
    }
    if !gb.ime { return false; }
    for interrupt_id in [VBlank, STAT, Timer, Serial, Joypad].iter() {
        let state = gb.mem.interrupt.state(*interrupt_id);
        match state {
            Active => {
                gb.ime = false;
                gb.mem.interrupt.set(*interrupt_id, false);
                let [lo, hi] = gb.pc.0.to_le_bytes();
                gb.sp = StackPointer(gb.sp.to_address() - 1);
                gb.mem *= (gb.sp, hi);
                gb.sp = StackPointer(gb.sp.to_address() - 1);
                gb.mem *= (gb.sp, lo);
                gb.pc.0 = *interrupt_id as u16;
                return true
            }
            _ => {  }
        }
    }
    return false
}

fn half_carry_8_add(a: u8, b: u8, c: u8) -> bool { (((a & 0xF) + ((b + c) & 0xF)) & 0x10) == 0x10 }

fn half_carry_8_sub(a: u8, b: u8, c: u8) -> bool { (((a & 0xF).wrapping_sub(b.wrapping_add(c) & 0xF)) & 0x10) == 0x10 }

fn half_carry_16_add(a: u16, b: u16, c: u16) -> bool { ((a & 0xFF).wrapping_add((b.wrapping_add(c)) & 0xFF)) & 0x10 == 0x1000 }

fn half_carry_16_sub(a: u16, b: u16, c: u16) -> bool { ((a & 0xFF).wrapping_sub(b.wrapping_add(c) & 0xFF)) & 0x10 == 0x1000 }