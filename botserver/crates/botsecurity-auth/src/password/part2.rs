use super::*;

impl Default for Argon2Config {
    fn default() -> Self {
        Self {
            memory_cost_kib: 65536,
            time_cost: 3,
            parallelism: 4,
            output_length: 32,
        }
    }
}
impl Argon2Config {
    pub fn high_security() -> Self {
        Self {
            memory_cost_kib: 131072,
            time_cost: 4,
            parallelism: 8,
            output_length: 32,
        }
    }

    pub fn low_memory() -> Self {
        Self {
            memory_cost_kib: 32768,
            time_cost: 4,
            parallelism: 2,
            output_length: 32,
        }
    }
}
