use std::collections::HashSet;

#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub enum RVRegister {
    Ra, Sp,
    A0, A1, A2, A3, A4, A5, A6, A7,
    T0, T1, T2, T3, T4, T5, T6,
    Zero,
}

impl RVRegister {
    pub fn is_temp(&self) -> bool {
        match self {
            RVRegister::T0 | RVRegister::T1 | RVRegister::T2 | RVRegister::T3 |
            RVRegister::T4 | RVRegister::T5 | RVRegister::T6 => true,
            _ => false,
        }
    }

    pub fn get_arg_reg(index: usize) -> RVRegister {
        match index {
            0 => RVRegister::A0,
            1 => RVRegister::A1,
            2 => RVRegister::A2,
            3 => RVRegister::A3,
            4 => RVRegister::A4,
            5 => RVRegister::A5,
            6 => RVRegister::A6,
            7 => RVRegister::A7,
            _ => panic!("Invalid argument index: {}", index),
        }
    }
}

impl std::fmt::Display for RVRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RVRegister::Ra => write!(f, "ra"),
            RVRegister::Sp => write!(f, "sp"),
            RVRegister::A0 => write!(f, "a0"),
            RVRegister::A1 => write!(f, "a1"),
            RVRegister::A2 => write!(f, "a2"),
            RVRegister::A3 => write!(f, "a3"),
            RVRegister::A4 => write!(f, "a4"),
            RVRegister::A5 => write!(f, "a5"),
            RVRegister::A6 => write!(f, "a6"),
            RVRegister::A7 => write!(f, "a7"),

            RVRegister::T0 => write!(f, "t0"),
            RVRegister::T1 => write!(f, "t1"),
            RVRegister::T2 => write!(f, "t2"),
            RVRegister::T3 => write!(f, "t3"),
            RVRegister::T4 => write!(f, "t4"),
            RVRegister::T5 => write!(f, "t5"),
            RVRegister::T6 => write!(f, "t6"),

            RVRegister::Zero => write!(f, "x0"),
        }
    }
}

// TODO: Temporary solution
// An iterator that iterates over t0-t6

#[derive(Clone)]
pub struct RVRegisterPool {
    avail: HashSet<RVRegister>
}

impl RVRegisterPool {
    pub fn new_temp_pool() -> Self {
        RVRegisterPool {
            avail: vec![
                RVRegister::T0, RVRegister::T1, RVRegister::T2, RVRegister::T3,
                RVRegister::T4, RVRegister::T5, RVRegister::T6
            ].into_iter().collect()
        }
    }

    pub fn next(&mut self) -> Option<RVRegister> {
        let register = self.avail.iter().next().cloned();
        if let Some(register) = register {
            // println!("Allocating register: {}", register);
            self.avail.remove(&register);
        }
        register
    }

    pub fn release(&mut self, register: RVRegister) {
        if register.is_temp() {
            // println!("Releasing register: {}", register);
            self.avail.insert(register);
        } else {
            // println!("Trying to release a non-temporary register: {}", register);
        }
    }
}
