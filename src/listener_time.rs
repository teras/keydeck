use crate::event::{send, DeviceEvent};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

/// A scheduled event that will fire at a specific time
#[derive(Eq, PartialEq)]
struct ScheduledEvent {
    fire_at: Instant,
    event: DeviceEvent,
}

impl Ord for ScheduledEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order for min-heap (earliest event first)
        other.fire_at.cmp(&self.fire_at)
    }
}

impl PartialOrd for ScheduledEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Manages all time-based events in a single thread with a priority queue
pub struct TimeManager {
    events: Arc<Mutex<BinaryHeap<ScheduledEvent>>>,
}

impl TimeManager {
    /// Creates a new TimeManager and spawns the background thread
    pub fn new(tx: Sender<DeviceEvent>, still_active: Arc<AtomicBool>) -> Self {
        let events: Arc<Mutex<BinaryHeap<ScheduledEvent>>> = Arc::new(Mutex::new(BinaryHeap::new()));
        let events_clone = events.clone();

        thread::spawn(move || {
            while still_active.load(std::sync::atomic::Ordering::Relaxed) {
                let now = Instant::now();

                // Fire all events that are due
                loop {
                    let next_event = {
                        let mut heap = events_clone.lock().unwrap();
                        if let Some(evt) = heap.peek() {
                            if evt.fire_at <= now {
                                heap.pop()
                            } else {
                                break; // Next event is in the future
                            }
                        } else {
                            break; // No events
                        }
                    };

                    if let Some(evt) = next_event {
                        send(&tx, evt.event);
                    } else {
                        break;
                    }
                }

                // Sleep until next event (or max 100ms)
                let sleep_duration = {
                    let heap = events_clone.lock().unwrap();
                    if let Some(next) = heap.peek() {
                        let until_next = next.fire_at.saturating_duration_since(now);
                        until_next.min(Duration::from_millis(100))
                    } else {
                        Duration::from_millis(100)
                    }
                };

                thread::sleep(sleep_duration);
            }
        });

        TimeManager { events }
    }

    /// Schedule a timer to fire after the specified duration
    pub fn schedule_timer(&self, sn: String, duration: Duration) {
        let mut heap = self.events.lock().unwrap();
        heap.push(ScheduledEvent {
            fire_at: Instant::now() + duration,
            event: DeviceEvent::TimerComplete { sn },
        });
    }

    /// Schedule a brightness change to fire after the specified duration
    pub fn schedule_brightness(&self, sn: String, brightness: u8, duration: Duration) {
        let mut heap = self.events.lock().unwrap();
        heap.push(ScheduledEvent {
            fire_at: Instant::now() + duration,
            event: DeviceEvent::SetBrightness { sn, brightness },
        });
    }
}
