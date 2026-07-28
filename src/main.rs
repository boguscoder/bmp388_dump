#![no_std]
#![no_main]

mod usb;

use bmp388_embedded::{
    Address, IirFilter, OutputDataRate, Oversampling, PowerMode, SensorConfig, r#async::Bmp388Async,
};
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::i2c::{self, InterruptHandler as I2CInterruptHandler};
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
    let mut i2c_config = i2c::Config::default();
    i2c_config.frequency = 400_000;
    let i2c = i2c::I2c::new_async(p.I2C1, scl, sda, Irqs, i2c_config);

    let baro_result = Bmp388Async::new(i2c, Delay, Address::Secondary).await;

    let Ok(mut baro) = baro_result else {
        panic!("Failed to initialize Barometer")
    };

    baro.set_sensor_config(SensorConfig {
        pressure_oversampling: Oversampling::X8,
        temperature_oversampling: Oversampling::X1,
        iir_filter: IirFilter::Off,
        output_data_rate: OutputDataRate::Hz50,
    })
    .await
    .ok();

    embassy_time::Timer::after_millis(10).await;

    baro.set_power_control(true, true, PowerMode::Normal)
        .await
        .unwrap();

    embassy_time::Timer::after_millis(10).await;

    let pwr = baro.power_control().await.unwrap();
    log::info!("Barometer Power Status: {:?}", pwr.mode);
    let mut imu_ticker = Ticker::every(Duration::from_hz(50));

    log::info!("Pressure(Pa), Temperature(C)");
    loop {
        let Ok(measurement) = baro.sensor_data().await else {
            continue;
        };

        log::info!("Barometer Power Status: {:?}", pwr.mode);
        log::info!("{}, {}", measurement.pressure, measurement.temperature);
        imu_ticker.next().await;
    }
}
