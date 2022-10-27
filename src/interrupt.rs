use crate::interrupt::InterruptId::{JoypadInt, SerialInt, StatInt, TimerInt, VBlankInt};
use crate::interrupt::InterruptState::{Active, Enabled, Inactive, Requested};
use std::ops::Index;


#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum InterruptId {
    VBlankInt = 0x40,
    StatInt = 0x48,
    TimerInt = 0x50,
    SerialInt = 0x58,
    JoypadInt = 0x60,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum InterruptState {
    Active,
    Inactive,
    Enabled,
    Requested
}

pub struct InterruptHandler {
    flag: u8,
    enable: u8,
    vblank: InterruptMask,
    stat: InterruptMask,
    serial: InterruptMask,
    timer: InterruptMask,
    joypad: InterruptMask,
}

pub const IE_ADDRESS: usize = 0xFFFF;
pub const IF_ADDRESS: usize = 0xFF0F;

impl InterruptHandler {
    pub fn new() -> Self {
        let flag = 0x00;
        let enable = 0x00;
        let vblank = InterruptMask(0x01);
        let stat = InterruptMask(0x02);
        let timer = InterruptMask(0x04);
        let serial = InterruptMask(0x08);
        let joypad = InterruptMask(0x10);
        InterruptHandler {
            flag,
            enable,
            vblank,
            stat,
            timer,
            serial,
            joypad,
        }
    }

    fn calc_state(&self, interrupt: InterruptId) -> InterruptState {
        let mask = self[interrupt].0;
        let enabled = self.enable & mask != 0;
        let requested = self.flag & mask != 0;
        return if requested && enabled {
            Active
        } else if enabled {
            Enabled
        } else if requested {
            Requested
        } else {
            Inactive
        };
    }

    pub fn get_state(&self, interrupt: InterruptId) -> InterruptState {
        let priority = [VBlankInt, StatInt, TimerInt, SerialInt, JoypadInt]
            .iter()
            .take_while(|&&i| i != interrupt)
            .filter(|&&i| self.calc_state(i) == Active)
            .next();

        return if let Some(&priority) = priority {
            self.calc_state(priority)
        } else {
            self.calc_state(interrupt)
        }
    }

    pub fn set(&mut self, interrupts: Vec<InterruptId>, set: bool) {
        if set {
            interrupts
                .iter()
                .for_each(|i| self.flag |= self[*i].0)
        } else {
            interrupts
                .iter()
                .for_each(|i| self.flag &= !self[*i].0)
        }
    }

    pub fn read(&self, address: usize) -> Option<u8> {
        match address {
            IE_ADDRESS => Some(self.enable),
            IF_ADDRESS => Some(self.flag),
            _ => None
        }
    }

    pub fn write(&mut self, address: usize, value: u8) -> bool {
        match address {
            IE_ADDRESS => {
                self.enable = value | 0xE0;
                true
            }
            IF_ADDRESS => {
                self.flag = value | 0xE0;
                true
            }
            _ => false
        }
    }
}

pub struct InterruptMask(u8);

impl Index<InterruptId> for InterruptHandler {
    type Output = InterruptMask;

    fn index(&self, id: InterruptId) -> &Self::Output {
        match id {
            VBlankInt => &self.vblank,
            StatInt => &self.stat,
            TimerInt => &self.timer,
            SerialInt => &self.serial,
            JoypadInt => &self.joypad,
        }
    }
}
