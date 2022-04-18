#![no_std]
#![no_main]
#![warn(clippy::pedantic)]

#[rtic::app(device = rp_pico::pac)]
mod app {
    use embedded_hal::{digital::v2::OutputPin, prelude::*};
    use embedded_time::duration::Extensions;
    use keyberon::{
        debounce::Debouncer,
        layout::Event,
        matrix::{Matrix, PressedKeys},
    };
    use picorne::{println, DEBOUNCE_TIME, SCAN_TIME, UART_CONFIG};
    use rp_pico::{
        hal::{
            clocks::init_clocks_and_plls,
            gpio::{DynPin, FunctionUart},
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
        class_prelude::UsbBusAllocator,
        device::{UsbDevice, UsbDeviceBuilder, UsbVidPid},
    };
    use usbd_serial::SerialPort;

    #[shared]
    struct Shared {
        serial_port: SerialPort<'static, UsbBus>,
        usb_dev: UsbDevice<'static, UsbBus>,
    }

    #[local]
    struct Local {
        alarm: Alarm0,
        debouncer: Debouncer<PressedKeys<4, 6>>,
        matrix: Matrix<DynPin, DynPin, 4, 6>,
        uart: uart::UartPeripheral<uart::Enabled, UART0, (Gp0Uart0Tx, Gp1Uart0Rx)>,
        watchdog: Watchdog,
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

        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("tom")
            .product("picorne right")
            .device_class(2)
            .build();

        let mut matrix = Matrix::<DynPin, DynPin, 4, 6>::new(
            [
                pins.gpio10.into_pull_up_input().into(),
                pins.gpio11.into_pull_up_input().into(),
                pins.gpio19.into_pull_up_input().into(),
                pins.gpio17.into_pull_up_input().into(),
            ],
            [
                pins.gpio18.into_push_pull_output().into(),
                pins.gpio20.into_push_pull_output().into(),
                pins.gpio21.into_push_pull_output().into(),
                pins.gpio14.into_push_pull_output().into(),
                pins.gpio13.into_push_pull_output().into(),
                pins.gpio12.into_push_pull_output().into(),
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

        println!(&mut serial_port, "init right");
        (
            Shared {
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

    #[task(binds = USBCTRL_IRQ, priority = 2, shared = [serial_port, usb_dev])]
    fn usb_rx(cx: usb_rx::Context) {
        (cx.shared.serial_port, cx.shared.usb_dev).lock(|serial, dev| dev.poll(&mut [serial]));
    }

    #[task(binds = TIMER_IRQ_0, priority = 1, local = [alarm, debouncer, matrix, uart, watchdog], shared = [serial_port, usb_dev])]
    fn scan_timer_irq(cx: scan_timer_irq::Context) {
        let scan_timer_irq::LocalResources {
            alarm,
            debouncer,
            matrix,
            uart,
            watchdog,
        } = cx.local;
        let scan_timer_irq::SharedResources {
            serial_port,
            usb_dev,
        } = cx.shared;

        let alarm = alarm;
        alarm.clear_interrupt();
        alarm.schedule(SCAN_TIME.microseconds()).unwrap();

        watchdog.feed();

        for event in debouncer.events(matrix.get().unwrap()) {
            let (i, j) = event.coord();
            uart.write_full_blocking(&[
                match event {
                    Event::Press(_, _) => 255,
                    Event::Release(_, _) => 254,
                },
                i,
                j,
            ]);
        }

        (serial_port, usb_dev).lock(|serial, dev| {
            dev.poll(&mut [serial]);
        });
    }
}
