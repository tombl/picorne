#![no_std]
#![no_main]
#![warn(clippy::pedantic)]

mod layout;

#[rtic::app(device = rp_pico::pac)]
mod app {
    use crate::layout::{CustomAction, LAYERS};
    use embedded_hal::{digital::v2::OutputPin, prelude::*};
    use embedded_time::duration::Extensions;
    use keyberon::{
        debounce::Debouncer,
        hid::HidClass,
        key_code::KbHidReport,
        keyboard::Keyboard,
        layout::{CustomEvent, Event, Layout},
        matrix::{Matrix, PressedKeys},
    };
    use picorne::{println, DEBOUNCE_TIME, SCAN_TIME, UART_CONFIG};
    use rp_pico::{
        hal::{
            clocks::init_clocks_and_plls,
            gpio::{bank0::Gpio25, DynPin, FunctionUart, Pin, PushPullOutput},
            rom_data::reset_to_usb_boot,
            timer::Alarm0,
            uart,
            usb::UsbBus,
            Sio, Timer, Watchdog,
        },
        pac::UART0,
        Gp0Uart0Tx, Gp1Uart0Rx, Pins, XOSC_CRYSTAL_FREQ,
    };
    use usb_device::{
        class::UsbClass,
        class_prelude::UsbBusAllocator,
        device::{UsbDevice, UsbDeviceBuilder, UsbVidPid},
    };
    use usbd_serial::SerialPort;

    #[shared]
    struct Shared {
        hid: HidClass<'static, UsbBus, Keyboard<Leds>>,
        serial_port: SerialPort<'static, UsbBus>,
        usb_dev: UsbDevice<'static, UsbBus>,
    }

    #[local]
    struct Local {
        alarm: Alarm0,
        debouncer: Debouncer<PressedKeys<4, 6>>,
        layout: Layout<12, 4, 5, CustomAction>,
        matrix: Matrix<DynPin, DynPin, 4, 6>,
        uart: uart::UartPeripheral<uart::Enabled, UART0, (Gp0Uart0Tx, Gp1Uart0Rx)>,
        watchdog: Watchdog,
    }

    pub struct Leds {
        caps_lock: Pin<Gpio25, PushPullOutput>,
    }
    impl keyberon::keyboard::Leds for Leds {
        fn caps_lock(&mut self, status: bool) {
            if status {
                self.caps_lock.set_high().unwrap();
            } else {
                self.caps_lock.set_low().unwrap();
            }
        }
    }

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

        let mut led = pins.led.into_push_pull_output();
        led.set_high().unwrap();
        let mut delay = cortex_m::delay::Delay::new(
            cx.core.SYST,
            embedded_time::fixed_point::FixedPoint::integer(&rp_pico::hal::Clock::freq(
                &clocks.system_clock,
            )),
        );
        delay.delay_ms(100);
        led.set_low().unwrap();

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

        let mut serial_port = SerialPort::new(usb_bus);

        let leds = Leds { caps_lock: led };
        let hid = keyberon::new_class(usb_bus, leds);

        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x16c0, 0x27db))
            .manufacturer("tom")
            .product("picorne left")
            .build();

        let mut matrix = Matrix::<DynPin, DynPin, 4, 6>::new(
            [
                pins.gpio21.into_pull_up_input().into(),
                pins.gpio20.into_pull_up_input().into(),
                pins.gpio12.into_pull_up_input().into(),
                pins.gpio14.into_pull_up_input().into(),
            ],
            [
                pins.gpio13.into_push_pull_output().into(),
                pins.gpio11.into_push_pull_output().into(),
                pins.gpio10.into_push_pull_output().into(),
                pins.gpio17.into_push_pull_output().into(),
                pins.gpio18.into_push_pull_output().into(),
                pins.gpio19.into_push_pull_output().into(),
            ],
        )
        .unwrap();

        {
            let pressed = matrix.get().unwrap();
            let mut pressed = pressed.iter_pressed();
            if pressed.clone().count() == 1 && pressed.next() == Some((1, 1)) {
                reset_to_usb_boot(0, 0);
            };
        }

        let uart = uart::UartPeripheral::new(
            cx.device.UART0,
            (
                pins.gpio0.into_mode::<FunctionUart>(),
                pins.gpio1.into_mode::<FunctionUart>(),
            ),
            &mut resets,
        )
        .enable(UART_CONFIG, clocks.peripheral_clock.into())
        .unwrap();

        watchdog.start((SCAN_TIME * 100).microseconds());

        println!(&mut serial_port, "init left");
        (
            Shared {
                hid,
                serial_port,
                usb_dev,
            },
            Local {
                alarm,
                debouncer: Debouncer::new(
                    PressedKeys::default(),
                    PressedKeys::default(),
                    DEBOUNCE_TIME,
                ),
                layout: Layout::new(&LAYERS),
                matrix,
                uart,
                watchdog,
            },
            init::Monotonics(),
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    #[task(binds = USBCTRL_IRQ, priority = 2, shared = [serial_port, usb_dev, hid])]
    fn usb_rx(cx: usb_rx::Context) {
        let usb_rx::SharedResources {
            serial_port,
            usb_dev,
            hid,
        } = cx.shared;

        (serial_port, usb_dev, hid).lock(|serial, dev, hid| {
            if dev.poll(&mut [serial, hid]) {
                serial.poll();
                hid.poll();
            }
        });
    }

    #[task(binds = TIMER_IRQ_0, priority = 1, local = [alarm, debouncer, layout, matrix, uart, watchdog], shared = [hid, serial_port, usb_dev])]
    fn scan_timer_irq(cx: scan_timer_irq::Context) {
        let scan_timer_irq::LocalResources {
            alarm,
            debouncer,
            layout,
            matrix,
            uart,
            watchdog,
        } = cx.local;
        let scan_timer_irq::SharedResources {
            mut hid,
            serial_port,
            usb_dev,
        } = cx.shared;

        alarm.clear_interrupt();
        alarm.schedule(SCAN_TIME.microseconds()).unwrap();

        watchdog.feed();

        for event in debouncer.events(matrix.get().unwrap()) {
            let event = event.transform(|i, j| (j, 5 - i));
            layout.event(event);
        }

        while uart.uart_is_readable() {
            let mut msg = [0u8; 3];
            if uart.read_full_blocking(&mut msg).is_err() {
                break;
            }

            let [tag, i, j] = msg;
            let event = match tag {
                255 => Event::Press(i, j),
                254 => Event::Release(i, j),
                _ => panic!("what"),
            }
            .transform(|i, j| (j, i + 6));

            layout.event(event);
        }

        match layout.tick() {
            CustomEvent::NoEvent => {}
            CustomEvent::Press(action) => match action {
                CustomAction::Reset => {
                    cortex_m::interrupt::disable();
                    loop {
                        // the watchdog will reset us
                        cortex_m::asm::nop();
                    }
                }
                CustomAction::Bootsel => reset_to_usb_boot(0, 0),
            },
            CustomEvent::Release(_) => unreachable!(),
        };

        let report = layout.keycodes().collect::<KbHidReport>();
        if hid.lock(|hid| hid.device_mut().set_keyboard_report(report.clone())) {
            while let Ok(0) = hid.lock(|hid| hid.write(report.as_bytes())) {}
        }

        (serial_port, hid, usb_dev).lock(|serial, hid, dev| {
            if dev.poll(&mut [serial, hid]) {
                serial.poll();
                hid.poll();
            };
        });
    }
}
