#[derive(Debug)]
pub enum RVRegister {
    A0,
}

impl std::fmt::Display for RVRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RVRegister::A0 => write!(f, "a0"),
        }
    }
}