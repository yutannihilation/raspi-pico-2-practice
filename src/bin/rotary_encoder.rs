// Blink LED and change the frequency by a rotary encoder (based on https://github.com/embassy-rs/embassy/blob/main/examples/rp235x/src/bin/pio_rotary_encoder.rs)
//
// Pins:
//   - GPIO4, GPIO5: connect to the rotary encoder
//   - GPIO25: on-board LED

#![no_std]
#![no_main]

use core::sync::atomic::{AtomicU32, Ordering};

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::rotary_encoder::{Direction, PioEncoder, PioEncoderProgram};
use embassy_time::{Duration, Timer};

// Program metadata for `picotool info`.
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 3] = [
    embassy_rp::binary_info::rp_program_name!(c"Rotary encoder example"),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

static FREQENCY: AtomicU32 = AtomicU32::new(10); // 2^10 = 1024 ≒ 1s

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn encoder_task(mut encoder: PioEncoder<'static, PIO0, 0>) {
    loop {
        match encoder.read().await {
            Direction::Clockwise => FREQENCY.fetch_add(1, Ordering::Relaxed),
            Direction::CounterClockwise => FREQENCY.fetch_sub(1, Ordering::Release),
        };
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let mut led = Output::new(p.PIN_25, Level::Low);

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);

    let prg = PioEncoderProgram::new(&mut common);
    let encoder0 = PioEncoder::new(&mut common, sm0, p.PIN_4, p.PIN_5, &prg);

    spawner.must_spawn(encoder_task(encoder0));

    loop {
        let freq = FREQENCY.load(Ordering::Relaxed);
        info!("freq: {}", 2u32.pow(freq));

        let delay = Duration::from_millis(2u32.pow(freq) as _);

        info!("led on!");
        led.set_high();
        Timer::after(delay).await;

        info!("led off!");
        led.set_low();
        Timer::after(delay).await;
    }
}
