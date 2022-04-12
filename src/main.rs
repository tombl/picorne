#![no_std]
#![no_main]

mod layout;
mod matrix;

use defmt_rtt as _;
use panic_probe as _;
use rp_pico::hal as _;

#[rtic::app(device = rp_pico::pac)]
mod app {
    use crate::{
        layout::{CustomAction, LAYERS},
        matrix::Matrix,
    };
    use embedded_hal::prelude::*;
    use embedded_time::duration::Extensions;
    use keyberon::{
        debounce::Debouncer, hid::HidClass, key_code::KbHidReport, keyboard::Keyboard,
        layout::Layout, matrix::PressedKeys,
    };
    use rp_pico::{
        hal::{
            clocks::init_clocks_and_plls, gpio::DynPin, rom_data::reset_to_usb_boot, timer::Alarm0,
            usb::UsbBus, Sio, Timer, Watchdog,
        },
        Pins, XOSC_CRYSTAL_FREQ,
    };
    use usb_device::{
        class_prelude::UsbBusAllocator,
        device::{UsbDevice, UsbDeviceBuilder, UsbVidPid},
    };

    const SCAN_TIME: u32 = 1000;

    #[shared]
    struct Shared {
        #[lock_free]
        alarm: Alarm0,
        #[lock_free]
        debouncer: Debouncer<PressedKeys<6, 4>>,
        #[lock_free]
        layout: Layout<CustomAction>,
        #[lock_free]
        usb_class: HidClass<'static, UsbBus, Keyboard<()>>,
        #[lock_free]
        usb_dev: UsbDevice<'static, UsbBus>,
        #[lock_free]
        matrix: Matrix<DynPin, DynPin, 6, 4>,
        #[lock_free]
        watchdog: Watchdog,
    }

    #[local]
    struct Local {}

    #[init(local = [usb_bus: Option<UsbBusAllocator<UsbBus>> = None])]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut watchdog = Watchdog::new(cx.device.WATCHDOG);
        let mut resets = cx.device.RESETS;

        let clocks = init_clocks_and_plls(
            XOSC_CRYSTAL_FREQ,
            cx.device.XOSC,
            cx.device.CLOCKS,
            cx.device.PLL_SYS,
            cx.device.PLL_USB,
            &mut resets,
            &mut watchdog,
        )
        .ok()
        .unwrap();

        let sio = Sio::new(cx.device.SIO);
        let pins = Pins::new(
            cx.device.IO_BANK0,
            cx.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut resets,
        );

        let mut timer = Timer::new(cx.device.TIMER, &mut resets);
        let mut alarm = timer.alarm_0().unwrap();
        alarm.schedule(SCAN_TIME.microseconds()).unwrap();
        alarm.enable_interrupt();

        let usb_bus: &'static _ = cx.local.usb_bus.insert(UsbBusAllocator::new(UsbBus::new(
            cx.device.USBCTRL_REGS,
            cx.device.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut resets,
        )));

        let usb_class = keyberon::new_class(usb_bus, ());
        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x16c0, 0x27db))
            .manufacturer("tom")
            .product("picorne")
            .build();

        let matrix = Matrix::<DynPin, DynPin, 6, 4>::new(
            [
                pins.gpio13.into_push_pull_output().into(),
                pins.gpio11.into_push_pull_output().into(),
                pins.gpio10.into_push_pull_output().into(),
                pins.gpio17.into_push_pull_output().into(),
                pins.gpio18.into_push_pull_output().into(),
                pins.gpio19.into_push_pull_output().into(),
            ],
            [
                pins.gpio21.into_pull_up_input().into(),
                pins.gpio20.into_pull_up_input().into(),
                pins.gpio12.into_pull_up_input().into(),
                pins.gpio14.into_pull_up_input().into(),
            ],
        )
        .unwrap();

        watchdog.start((SCAN_TIME * 10).microseconds());

        (
            Shared {
                alarm,
                debouncer: Debouncer::new(PressedKeys::default(), PressedKeys::default(), 10),
                layout: Layout::new(LAYERS),
                usb_class,
                usb_dev,
                matrix,
                watchdog,
            },
            Local {},
            init::Monotonics(),
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    #[task(binds = USBCTRL_IRQ, shared = [usb_dev, usb_class])]
    fn usb_rx(cx: usb_rx::Context) {
        cx.shared.usb_dev.poll(&mut [cx.shared.usb_class]);
    }

    #[task(binds = TIMER_IRQ_0, shared = [alarm, debouncer, layout, matrix, usb_class, usb_dev, watchdog])]
    fn scan_timer_irq(cx: scan_timer_irq::Context) {
        let alarm = cx.shared.alarm;
        alarm.clear_interrupt();
        alarm.schedule(SCAN_TIME.microseconds()).unwrap();

        cx.shared.watchdog.feed();

        for event in cx.shared.debouncer.events(cx.shared.matrix.get().unwrap()) {
            cx.shared.layout.event(event);
        }

        match cx.shared.layout.tick() {
            keyberon::layout::CustomEvent::NoEvent => {}
            keyberon::layout::CustomEvent::Press(action) => match action {
                CustomAction::Reset => {
                    cortex_m::interrupt::disable();
                    loop {
                        // the watchdog will reset us
                        cortex_m::asm::nop();
                    }
                }
                CustomAction::Bootsel => reset_to_usb_boot(0, 0),
            },
            keyberon::layout::CustomEvent::Release(_) => unreachable!(),
        };

        let report = cx.shared.layout.keycodes().collect::<KbHidReport>();
        if cx
            .shared
            .usb_class
            .device_mut()
            .set_keyboard_report(report.clone())
        {
            while let Ok(0) = cx.shared.usb_class.write(report.as_bytes()) {}
        }

        cx.shared.usb_dev.poll(&mut [cx.shared.usb_class]);
    }
}
