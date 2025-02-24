#[derive(Debug, Copy, Clone)]
pub enum RVRegister {
    A0,
    T0, T1, T2, T3, T4, T5, T6,
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
        }
    }
}

// TODO: Temporary solution
// An iterator that iterates over t0-t6

#[derive(Copy, Clone)]
pub struct RVRegisterIterator {
    current: usize,
}

impl RVRegisterIterator {
    pub fn new() -> Self {
        RVRegisterIterator { current: 1 }
    }
}

impl Iterator for RVRegisterIterator {
    type Item = RVRegister;

    fn next(&mut self) -> Option<Self::Item> {
        // Iterates from T0 to T6 (indices 1 to 7)
        if self.current >= 1 && self.current <= 6 {
            let reg = match self.current {
                1 => RVRegister::T0,
                2 => RVRegister::T1,
                3 => RVRegister::T2,
                4 => RVRegister::T3,
                5 => RVRegister::T4,
                6 => RVRegister::T5,
                7 => RVRegister::T6,
                _ => return None,
            };
            self.current += 1;
            Some(reg)
        } else {
            None
        }
    }
}
