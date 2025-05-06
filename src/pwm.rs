#[derive(Debug, Clone, Copy)]
pub struct PwmStep {
    pub length: u8,
    pub data: u32,
}

pub struct PwmData {
    pub pwm_levels: [u8; 8],
    pub pwm_steps: [PwmStep; 9],
}

impl PwmData {
    pub const fn new() -> Self {
        let null_step = PwmStep {
            length: 255,
            data: 0,
        };

        Self {
            pwm_levels: [0; 8],
            pwm_steps: [null_step; 9],
        }
    }

    pub fn reflect(&mut self) {
        let mut indices: [usize; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        indices.sort_unstable_by_key(|&i| self.pwm_levels[i]);

        let mut data: u32 = 0b11111111;
        let mut prev_level: u8 = 0;
        let mut cur_level: u8 = 255;

        for (i, &cur_index) in indices.iter().enumerate() {
            cur_level = self.pwm_levels[cur_index];

            self.pwm_steps[i].length = cur_level - prev_level;
            self.pwm_steps[i].data = data;

            data &= !(1 << cur_index); // turn off the pin

            prev_level = cur_level;
        }

        // period after all pins are set low
        self.pwm_steps[8] = PwmStep {
            length: 255 - cur_level,
            data: 0,
        };
    }
}
