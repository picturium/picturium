use std::time::Duration;
use clokwerk::{Job, ScheduleHandle, Scheduler, TimeUnits};
use crate::cache::buster::bust_cache;

pub fn schedule() -> ScheduleHandle {

    let mut scheduler = Scheduler::new();

    scheduler.every(1.day()).plus(4.hours()).run(bust_cache);

    scheduler.watch_thread(Duration::from_millis(500))

}