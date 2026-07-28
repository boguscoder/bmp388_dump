#![no_std]
#![no_main]

mod usb;

use bmp388_embedded::{Address, Oversampling, r#async::Bmp388Async};
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::i2c::{self, Config, InterruptHandler as I2CInterruptHandler};
use embassy_rp::peripherals::I2C1;
use embassy_time::{Delay, Duration, Ticker};
use panic_probe as _;

bind_interrupts!(struct Irqs {
    I2C1_IRQ => I2CInterruptHandler<I2C1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    spawner.must_spawn(usb::usb_setup(p.USB));

    // IMU via i2c
    let sda = p.PIN_2;
    let scl = p.PIN_3;

    log::info!("set up i2c ");
    let i2c = i2c::I2c::new_async(p.I2C1, scl, sda, Irqs, Config::default());

    let baro_result = Bmp388Async::new(i2c, Delay, Address::Secondary).await;

    let Ok(mut baro) = baro_result else {
        panic!("Failed to initialize Barometer")
    };

    baro.set_oversampling(Oversampling::X8, Oversampling::X2)
        .await
        .unwrap();

    let mut imu_ticker = Ticker::every(Duration::from_hz(1000));

    log::info!("Pressure(Pa), Temperature(C)");
    loop {
        let Ok(measurement) = baro.forced_measurement().await else {
            continue;
        };

        log::info!("{}, {}", measurement.pressure, measurement.temperature);
        // Delay until next loop
        imu_ticker.next().await;
    }
}
