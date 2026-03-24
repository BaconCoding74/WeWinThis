use std::panic::catch_unwind;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use thread_priority::{set_current_thread_priority, ThreadPriority};

const MAX_ITER: i32 = 30;
const INTERVAL: u64 = 2;

struct TestResult {
    c1: i32,
    c2: i32,
}

fn print_result(c1: Arc<Mutex<i32>>, c2: Arc<Mutex<i32>>) {
    let c1_data = c1.lock().unwrap();
    let c2_data  = c2.lock().unwrap();

    println!("Thread 1: {}", c1_data);
    println!("Thread 2: {}", c2_data);
}

fn thread_priority_process() -> TestResult {
    let running = Arc::new(AtomicBool::new(true));
    let counter1 = Arc::new(Mutex::new(0));
    let counter2 = Arc::new(Mutex::new(0));

    let c1_t1 = counter1.clone();
    let running_t1 = running.clone();
    let thread1 = thread::spawn(move || {
        set_current_thread_priority(ThreadPriority::Max).expect("set_current_thread_priority failed");

        let result = catch_unwind(move || {
            while running_t1.load(Ordering::Acquire) {
                {
                    let mut data = c1_t1.try_lock().expect("counter1 lock failed");
                    *data += 1;
                }
                thread::sleep(Duration::from_millis(1));
            }
        });

        if result.is_err() {
            println!("counter1 lock failed");
        }
    });

    let c2_t2 = counter2.clone();
    let running_t2 = running.clone();
    let thread2 = thread::spawn(move || {
        set_current_thread_priority(ThreadPriority::Min).expect("set_current_thread_priority failed");

        while running_t2.load(Ordering::Acquire) {
            {
                let mut data = c2_t2.try_lock().expect("counter2 lock failed");
                *data += 1;
            }

            thread::sleep(Duration::from_millis(1));
        }
    });

    thread::sleep(Duration::from_secs(INTERVAL));
    running.store(false, Ordering::Release);
    print_result(counter1.clone(), counter2.clone());

    TestResult {
        c1: *counter1.lock().unwrap(),
        c2: *counter2.lock().unwrap(),
    }
}

pub fn thread_priority_test() {
    let mut counter1_count: i32 = 0;
    let mut counter2_count: i32 = 0;

    for i in 1..=MAX_ITER {
        println!("\n--- Iteration {} ---", i);
        let results = thread_priority_process();
        counter1_count += results.c1;
        counter2_count += results.c2;
    }

    println!("\n--- Summary ---");
    println!("Average Counter 1: {}", counter1_count / MAX_ITER);
    println!("Average Counter 2: {}", counter2_count / MAX_ITER);
}