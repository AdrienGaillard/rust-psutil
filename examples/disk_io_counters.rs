extern crate psutil;

use std::{thread, time};

fn main() {
    let mut disk_io_counters_collector = psutil::disk::DiskIOCountersNoWrap::new();

    loop {
        let block_time = time::Duration::from_millis(2000);
        thread::sleep(block_time);

        let disk_io_counters = match disk_io_counters_collector.disk_io_counters(true) {
            Ok(disk_io_counters) => disk_io_counters,
            Err(_) => {
                println!("Could not loading disk informations");
                continue;
            }
        };

        println!(
            "Disk general usage:
            read_count:         {}
            write_count:        {}
            read_bytes:         {}
            write_bytes:        {}
            read_time:          {}
            write_time:         {}
            read_merged_time:   {}
            write_merged_time:  {}
            busy_time:          {}",
            disk_io_counters.read_count,
            disk_io_counters.write_count,
            disk_io_counters.read_bytes,
            disk_io_counters.write_bytes,
            disk_io_counters.read_time,
            disk_io_counters.write_time,
            disk_io_counters.read_merged_count,
            disk_io_counters.write_merged_count,
            disk_io_counters.busy_time,
        );
    }
}
