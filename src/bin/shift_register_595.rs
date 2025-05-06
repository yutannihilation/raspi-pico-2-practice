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

// Program metadata for `picotool info`.
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 3] = [
    embassy_rp::binary_info::rp_program_name!(c"74CH595"),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

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
        cfg.clock_divider = 10_000.to_fixed(); // TODO

        sm.set_config(&cfg);
        sm.set_enable(true);

        Self { sm }
    }

    pub fn send(&mut self, v: u32) {
        self.sm.tx().push(v);
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);

    let mut shift_register = PioShiftRegister::new(
        &mut common,
        sm0,
        p.PIN_2, // out_pin
        p.PIN_3, // click_pin
        p.PIN_4, // ratch_pin
    );

    let delay = Duration::from_millis(100);
    let mut i = 0u32;
    loop {
        info!("i: {:032b}", i);
        shift_register.send(i);
        Timer::after(delay).await;

        i += 1;
    }
}
