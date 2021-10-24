#![no_std]
#![no_main]

use core::cell::RefCell;
use core::ops::DerefMut;

use longan_nano::hal::delay::McycleDelay;
use longan_nano::hal::eclic::{EclicExt, Level, LevelPriorityBits, Priority, TriggerType};
use longan_nano::hal::pac::{ECLIC, Interrupt, TIMER0};
use longan_nano::hal::timer::{Event, Timer};
use panic_halt as _;

use longan_nano::hal::{pac, rcu::RcuExt, prelude::*};

use longan_nano::led::{Led, rgb};

use riscv_rt::entry;
use riscv::interrupt::{self, Mutex, free};

// Sharing the timer and led between interrupt and main
static INTERRUPT_TIMER: Mutex<RefCell<Option<Timer<TIMER0>>>> = Mutex::new(RefCell::new(None));
static LED: Mutex<RefCell<Option<longan_nano::led::BLUE>>> = Mutex::new(RefCell::new(None));

#[allow(non_snake_case)]
#[no_mangle]
fn TIMER0_UP() {
    free(|cs| {
        if let Some(ref mut interrupt_timer) = INTERRUPT_TIMER.borrow(*cs).borrow_mut().deref_mut() {
            interrupt_timer.clear_update_interrupt_flag();
        }
    });

    free(|cs| {
        if let Some(ref mut led) = LED.borrow(*cs).borrow_mut().deref_mut() {
            if led.is_on() {
                led.off();
            } else {
                led.on();
            }
        }
    });


}

#[entry]
fn main() -> ! {

    let dp = pac::Peripherals::take().unwrap();

    let mut rcu = dp
        .RCU
        .configure()
        .ext_hf_clock(8.mhz())
        .sysclk(108.mhz())
        .freeze();

    let mut delay = McycleDelay::new(&rcu.clocks);

    let gpioa = dp.GPIOA.split(&mut rcu);
    let gpioc = dp.GPIOC.split(&mut rcu);

    let (mut red, mut green, mut blue) = rgb(gpioc.pc13, gpioa.pa1, gpioa.pa2);

    // Make sure all leds are off
    red.off();
    green.off();
    blue.off();

    // Move led to shared variable
    free(|cs| {
        LED.borrow(*cs).replace(Some(blue));
    });

    // Timer arguments
    let timer = dp.TIMER0;
    let timeout = 2.hz();

    // Intialize timer
    let mut interrupt_timer: Timer<TIMER0> = Timer::timer0(timer, timeout, &mut rcu);
    interrupt_timer.listen(Event::Update);

    // Move timer to shared variable
    free(|cs| {
        INTERRUPT_TIMER.borrow(*cs).replace(Some(interrupt_timer));
    });


    // Reset and config ECLIC
    ECLIC::reset();
    ECLIC::set_threshold_level(Level::L0);
    ECLIC::set_level_priority_bits(LevelPriorityBits::L3P1);

    // Setup the interrupt
    ECLIC::setup(
        Interrupt::TIMER0_UP,
        TriggerType::Level,
        Level::L1,
        Priority::P1,
    );
    
    // Unmask and enable interrupt
    unsafe { ECLIC::unmask(Interrupt::TIMER0_UP) };
    unsafe { riscv::interrupt::enable() };

    loop {
        delay.delay_ms(1000);
    }
}



