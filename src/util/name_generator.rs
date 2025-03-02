// A unique name generator for basic block

pub struct NameGenerator {
    counter: u32,
}

impl NameGenerator {
    pub fn new() -> Self {
        NameGenerator {
            counter: 0,
        }
    }

    pub fn generate_group(&mut self, prefixes: &[&str]) -> Vec<String> {
        let group: Vec<String> = prefixes
            .iter()
            .map(|prefix| format!("{}{}", prefix, self.counter))
            .collect();
        self.counter += 1;
        group
    }

    pub fn generate(&mut self, prefix: &str) -> String {
        let name = format!("{}{}", prefix, self.counter);
        self.counter += 1;
        name
    }
}