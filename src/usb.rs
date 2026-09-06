use embassy_futures::join::join;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, Config};
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

type UsbDriver = Driver<'static, USB>;
type UsbDevice = embassy_usb::UsbDevice<'static, UsbDriver>;

async fn usb_log_task(class: CdcAcmClass<'static, UsbDriver>) {
    embassy_usb_logger::with_class!(1024, log::LevelFilter::Info, class).await
}

async fn usb_run_task(mut dev: UsbDevice) {
    dev.run().await;
}

#[embassy_executor::task]
pub async fn usb_setup(p: embassy_rp::Peri<'static, embassy_rp::peripherals::USB>) {
    let driver = Driver::new(p, Irqs);

    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Embassy");
    config.product = Some("USB-serial console");
    config.serial_number = Some("0xC0DECAFE");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    let mut builder = {
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

        Builder::new(
            driver,
            config,
            CONFIG_DESCRIPTOR.init([0; 256]),
            BOS_DESCRIPTOR.init([0; 256]),
            &mut [],
            CONTROL_BUF.init([0; 64]),
        )
    };

    let logger_class = {
        static STATE: StaticCell<State> = StaticCell::new();
        let state = STATE.init(State::new());
        CdcAcmClass::new(&mut builder, state, 64)
    };

    let usb = builder.build();
    join(usb_run_task(usb), usb_log_task(logger_class)).await;
}
