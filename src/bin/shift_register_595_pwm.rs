// Control 74CH595 (shift register) via PIO
//
// ### Wires from RasPi Pico
//
//         ┌─────v─────┐
//       1 │           │ 16
//       2 │           │ 15
//       3 │           │ 14 Input  <------------ GPIO_2
//       4 │           │ 13
//       5 │           │ 12 Clock for input  <-- GPIO_3
//       6 │           │ 11 Clock for output  <- GPIO_4
//       7 │           │ 10
//       8 │           │  9
//         └───────────┘
//
// ### Output
//
//          ┌─────v─────┐
//     QB 1 │           │ 16
//     QC 2 │           │ 15 QA
//     QD 3 │           │ 14
//     QE 4 │           │ 13
//     QF 5 │           │ 12
//     QG 6 │           │ 11
//     QH 7 │           │ 10
//        8 │           │  9
//          └───────────┘

// ### Other pins
//
//           ┌─────v─────┐
//         1 │           │ 16 Vcc
//         2 │           │ 15
//         3 │           │ 14
//         4 │           │ 13 Disable  <--- GND
//         5 │           │ 12
//         6 │           │ 11
//         7 │           │ 10 Clear  <----- Vcc
//     GND 8 │           │  9 (chain to next)
//           └───────────┘

#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use fixed::traits::ToFixed;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_rp::{
    Peri, bind_interrupts,
    peripherals::PIO0,
    pio::{
        Common, Config, Instance, InterruptHandler, Pio, PioPin, StateMachine, program::pio_file,
    },
};
use embassy_time::{Duration, Timer};

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;

extern crate rp2350_misc;
use rp2350_misc::pwm::PwmData;

// Program metadata for `picotool info`.
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 3] = [
    embassy_rp::binary_info::rp_program_name!(c"74CH595"),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

static DATA: Mutex<ThreadModeRawMutex, PwmData> = Mutex::new(PwmData::new());

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

pub struct PioShiftRegister<'d, T: Instance, const SM: usize> {
    sm: StateMachine<'d, T, SM>,
}

impl<'d, T: Instance, const SM: usize> PioShiftRegister<'d, T, SM> {
    pub fn new(
        pio: &mut Common<'d, T>,
        mut sm: StateMachine<'d, T, SM>,
        out_pin: Peri<'d, impl PioPin>,
        click_pin: Peri<'d, impl PioPin>,
        ratch_pin: Peri<'d, impl PioPin>,
    ) -> Self {
        let out_pin = pio.make_pio_pin(out_pin);
        let clock_pin = pio.make_pio_pin(click_pin);
        let ratch_pin = pio.make_pio_pin(ratch_pin);

        sm.set_pin_dirs(
            embassy_rp::pio::Direction::Out,
            &[&out_pin, &clock_pin, &ratch_pin],
        );

        let prg = pio_file!("pio/shift_register.pio");
        let mut cfg = Config::default();
        cfg.use_program(&pio.load_program(&prg.program), &[&clock_pin, &ratch_pin]);
        cfg.set_out_pins(&[&out_pin]);
        cfg.clock_divider = 100.to_fixed(); // TODO

        sm.set_config(&cfg);
        sm.set_enable(true);

        Self { sm }
    }

    pub fn send(&mut self, v: u32) {
        self.sm.tx().push(v);
    }
}

#[embassy_executor::task]
async fn shift_register_task(mut shift_register: PioShiftRegister<'static, PIO0, 0>) {
    loop {
        let data = DATA.lock().await;
        let pwm_steps = data.pwm_steps.clone();
        drop(data);

        for step in pwm_steps {
            // info!("{:032b} (length: {})", step.data, step.length);

            if step.length > 0 {
                shift_register.send(step.data);
                Timer::after(Duration::from_micros(step.length as u64 * 10)).await;
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);

    let shift_register = PioShiftRegister::new(
        &mut common,
        sm0,
        p.PIN_2, // out_pin
        p.PIN_3, // click_pin
        p.PIN_4, // ratch_pin
    );

    spawner.must_spawn(shift_register_task(shift_register));

    let mut data = DATA.lock().await;
    data.pwm_levels = [0, 2, 5, 0, 0, 100, 0, 0];
    data.reflect();
    drop(data);

    // const IDX: [usize; 8] = [1, 2, 3, 4, 5, 6, 7, 4]; // 0 = nowhere

    // let mut phase = 0.0_f32;
    // let mut cur_index = 0usize;
    // let mut next_index = 0usize;

    // loop {
    //     let mut data = DATA.lock().await;

    //     if phase >= 1.0 {
    //         // reset the current segment
    //         data.pwm_levels[cur_index] = 0;

    //         cur_index = next_index;
    //         next_index = (next_index + 1) % 8;

    //         phase -= 1.0;
    //     }

    //     data.pwm_levels[IDX[cur_index]] = (255. * (1.0 - phase)) as u32;
    //     data.pwm_levels[IDX[next_index]] = (255. * (phase - 0.4) * 1.667) as u32;

    //     data.reflect();

    //     // must unlock before sleeping, otherwise the other task cannot grasp the lock!
    //     drop(data);

    //     Timer::after(Duration::from_millis(1000)).await;
    // }
}
