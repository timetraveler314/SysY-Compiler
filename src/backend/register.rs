use std::collections::HashSet;

#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub enum RVRegister {
    A0,
    T0, T1, T2, T3, T4, T5, T6,
    Zero,
}

impl std::fmt::Display for RVRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RVRegister::A0 => write!(f, "a0"),

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
            self.avail.remove(&register);
        }
        register
    }

    pub fn release(&mut self, register: RVRegister) {
        self.avail.insert(register);
    }
}
