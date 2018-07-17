//! Load disk informations
//! Author : Adrien Gaillard

#![deny(
    missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts,
    trivial_numeric_casts, unstable_features, unused_import_braces, unused_qualifications,
    unsafe_code
)]

extern crate libc;

use std::ffi::CString;
use std::io::{Error, ErrorKind, Result};
use std::mem;
use std::path::Path;
use utils::read_file;

/// Struct that contains informations about mounted partition
#[derive(Debug)]
pub struct MountedPartition {
    /// This field describes the block special device or remote filesystem to be mounted.
    pub device: String,

    /// This field describes the block special device or remote filesystem to be mounted.
    pub mountpoint: String,

    /// This field describes the type of the filesystem.
    pub fstype: String,

    /// This field describes the mount options associated with the filesystem.
    pub opts: String,
}

/// Struct that contains disk usage informations
#[derive(Copy, Clone, Debug)]
pub struct DiskUsage {
    /// Total disk in bytes
    pub total: u64,

    /// Disk used part in bytes
    pub used: u64,

    /// Disk free part in bytes
    pub free: u64,

    /// Percentage of used disk
    pub percent: f64,
}

/// Disk counter struct
#[derive(Clone, Copy, Debug)]
pub struct DiskIOCounters {
    /// Number of reads
    pub read_count: u64,

    /// Number of writes
    pub write_count: u64,

    /// Number of bytes read
    pub read_bytes: u64,

    /// Number of bytes written
    pub write_bytes: u64,

    /// Time spent reading from disk (in milliseconds)
    pub read_time: u64,

    /// Time spent writing to disk (in milliseconds)
    pub write_time: u64,

    /// Time spent doing actual I/Os (in milliseconds)
    pub read_merged_count: u64,

    /// Number of merged reads
    pub write_merged_count: u64,

    /// Number of merged writes
    pub busy_time: u64,
}

/// Disk counter struct to use nowrap mode
#[derive(Clone, Debug)]
pub struct DiskIOCountersNoWrap {
    /// Save the total of counters
    disk_io_counters: Vec<DiskIOCounters>,

    /// Save the values of the last call of disk_io_counters
    disk_io_counters_last_call: Vec<DiskIOCounters>,

    initialize: bool,
}

impl DiskIOCountersNoWrap {
    /// Initialize a DiskIOCountersNoWrap struct
    pub fn new() -> DiskIOCountersNoWrap {
        DiskIOCountersNoWrap {
            disk_io_counters: Vec::new(),
            disk_io_counters_last_call: Vec::new(),
            initialize: false,
        }
    }

    /// Reset de cache for disk_io_counter in nowrap mode
    pub fn cache_clear(&mut self) {
        self.disk_io_counters = Vec::new();
        self.disk_io_counters_last_call = Vec::new();
        self.initialize = false;
    }

    /// Return system-wide disk I/O statistics as a DiskIOCounters structs
    ///
    /// If nowrap is true psutil will detect and adjust those numbers across
    /// function calls and add “old value” to “new value” so that the returned
    /// numbers will always be increasing or remain the same, but never decrease.
    /// <DiskIOCountersNoWrap>.cache_clear() can be used to invalidate the nowrap cache.
    pub fn disk_io_counters(&mut self, nowrap: bool) -> Result<DiskIOCounters> {
        let disk_io_counters_vector = self.disk_io_counters_perdisk(nowrap)?;
        let mut disk_io_counters_total = DiskIOCounters {
            read_count: 0,
            write_count: 0,
            read_bytes: 0,
            write_bytes: 0,
            read_time: 0,
            write_time: 0,
            read_merged_count: 0,
            write_merged_count: 0,
            busy_time: 0,
        };
        for disk_io_counters in disk_io_counters_vector {
            disk_io_counters_total.read_count += disk_io_counters.read_count;
            disk_io_counters_total.write_count += disk_io_counters.write_count;
            disk_io_counters_total.read_bytes += disk_io_counters.read_bytes;
            disk_io_counters_total.write_bytes += disk_io_counters.write_bytes;
            disk_io_counters_total.read_time += disk_io_counters.read_time;
            disk_io_counters_total.write_time += disk_io_counters.write_time;
            disk_io_counters_total.read_merged_count += disk_io_counters.read_merged_count;
            disk_io_counters_total.write_merged_count += disk_io_counters.write_merged_count;
            disk_io_counters_total.busy_time += disk_io_counters.busy_time;
        }
        Ok(disk_io_counters_total)
    }

    /// Return system-wide disk I/O statistics per disk as a vector of a DiskIOCounters structs
    ///
    /// If nowrap is true psutil will detect and adjust those numbers across
    /// function calls and add “old value” to “new value” so that the returned
    /// numbers will always be increasing or remain the same, but never decrease.
    /// <DiskIOCountersNoWrap>.cache_clear() can be used to invalidate the nowrap cache.
    pub fn disk_io_counters_perdisk(&mut self, nowrap: bool) -> Result<Vec<DiskIOCounters>> {
        let partitions = read_file(Path::new("/proc/partitions"))?;
        let partitions = get_partitions(&partitions)?;
        let disk_stats = read_file(Path::new("/proc/diskstats"))?;
        let lines: Vec<&str> = disk_stats.lines().collect();
        let mut disks_infos: Vec<DiskIOCounters> = Vec::new();

        for line in lines {
            let mut disk_infos: Vec<&str> = line.split_whitespace().collect();
            if disk_infos.len() == 14 {
                let name: &str = disk_infos[2];
                disk_infos.remove(2);
                disk_infos.remove(1);
                disk_infos.remove(0);
                let disk_infos: Vec<u64> = line_disk_stats(disk_infos)?;

                // This function does not support kernel version under 2.6+
                if partitions.contains(&name) {
                    let ssize = get_sector_size(name)?;
                    disks_infos.push(DiskIOCounters {
                        read_count: disk_infos[0],
                        write_count: disk_infos[4],
                        read_bytes: disk_infos[2] * ssize,
                        write_bytes: disk_infos[6] * ssize,
                        read_time: disk_infos[3],
                        write_time: disk_infos[7],
                        read_merged_count: disk_infos[1],
                        write_merged_count: disk_infos[5],
                        busy_time: disk_infos[9],
                    });
                }
            } else {
                return Err(Error::new(
                      ErrorKind::InvalidData,
                      format!("/proc/diskstats has ne the right number of values. Maybe your kernel version is too old (Kernel 2.6+ minimum)."),
                  ));
            }
        }

        if nowrap {
            if self.initialize {
                self.disk_io_counters =
                    total_disk_io_counters(&self.disk_io_counters_last_call, &disks_infos);
                self.disk_io_counters_last_call = disks_infos;
            } else {
                self.disk_io_counters = disks_infos.clone();
                self.disk_io_counters_last_call = disks_infos;
                self.initialize = true;
            }
            return Ok(self.disk_io_counters.clone());
        } else {
            return Ok(disks_infos);
        }
    }
}

/// Determine filesystem we want to look for
fn fstype(data: &str) -> Vec<&str> {
    let lines: Vec<&str> = data.lines().collect();
    let mut fstypes: Vec<&str> = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields[0] == "nodev" && fields[1] == "zfs" {
            fstypes.push(fields[1]);
        }
        if fields[0] != "nodev" {
            fstypes.push(fields[0]);
        }
    }
    fstypes
}

/// Determine partitions we want to look for
fn get_partitions(data: &str) -> Result<Vec<&str>> {
    let mut lines: Vec<&str> = data.lines().collect();
    // Removal of the two first line of /proc/partitions.
    // This two lines countains no usefull informations.
    if lines.len() >= 2 {
        lines.remove(1);
        lines.remove(0);
    }
    let mut partitions: Vec<&str> = Vec::new();
    for line in lines.iter().rev() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() == 4
            && (fields[3].ends_with("0")
                || fields[3].ends_with("1")
                || fields[3].ends_with("2")
                || fields[3].ends_with("3")
                || fields[3].ends_with("4")
                || fields[3].ends_with("5")
                || fields[3].ends_with("6")
                || fields[3].ends_with("7")
                || fields[3].ends_with("8")
                || fields[3].ends_with("9"))
        {
            // we're dealing with a partition (e.g. 'sda1'); 'sda' will
            // also be around but we want to omit it
            partitions.push(fields[3]);
        } else {
            if fields.len() == 4
                && (partitions.len() == 0
                    || !partitions[partitions.len() - 1].starts_with(fields[3]))
            {
                // we're dealing with a disk entity for which no
                // partitions have been defined (e.g. 'sda' but
                // 'sda1' was not around)
                partitions.push(fields[3]);
            }
            if fields.len() != 4 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("failed to load partition information on /proc/partitions"),
                ));
            }
        }
    }
    Ok(partitions)
}

fn get_sector_size(partition_name: &str) -> Result<u64> {
    let path = format!("/sys/block/{}/queue/hw_sector_size", partition_name);
    let partition_size = match read_file(Path::new(&path)) {
        Ok(r) => r,
        // man iostat states that sectors are equivalent with blocks and
        // have a size of 512 bytes since 2.4 kernels
        Err(_) => return Ok(512),
    };
    match partition_size.trim().parse::<u64>() {
        Ok(v) => Ok(v),
        Err(_) => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("failed to parse {} in get_sector_size", partition_size),
            ))
        }
    }
}

fn line_disk_stats(line: Vec<&str>) -> Result<Vec<u64>> {
    let mut result: Vec<u64> = Vec::new();
    for value in line {
        result.push(match value.parse::<u64>() {
            Ok(v) => v,
            Err(_) => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("failed to parse {} in get_sector_size", value),
                ))
            }
        });
    }

    Ok(result)
}

fn total_disk_io_counters(
    past_disk_io_counters: &Vec<DiskIOCounters>,
    current_disk_io_counters: &Vec<DiskIOCounters>,
) -> Vec<DiskIOCounters> {
    let mut total_disk_io_counters: Vec<DiskIOCounters> = Vec::new();
    let max_value: u64 = 4294967296;
    if past_disk_io_counters.len() == current_disk_io_counters.len() {
        for (iter, past_counters) in past_disk_io_counters.iter().enumerate() {
            let current_counters = current_disk_io_counters[iter];
            total_disk_io_counters.push(DiskIOCounters {
                read_count: {
                    if current_counters.read_count >= past_counters.read_count {
                        current_counters.read_count
                    } else {
                        current_counters.read_count + max_value - past_counters.read_count
                    }
                },
                write_count: {
                    if current_counters.write_count >= past_counters.write_count {
                        current_counters.write_count
                    } else {
                        current_counters.write_count + max_value - past_counters.write_count
                    }
                },
                read_bytes: {
                    if current_counters.read_bytes >= past_counters.read_bytes {
                        current_counters.read_bytes
                    } else {
                        current_counters.read_bytes + max_value - past_counters.read_bytes
                    }
                },
                write_bytes: {
                    if current_counters.write_bytes >= past_counters.write_bytes {
                        current_counters.write_bytes
                    } else {
                        current_counters.write_bytes + max_value - past_counters.write_bytes
                    }
                },
                read_time: {
                    if current_counters.read_time >= past_counters.read_time {
                        current_counters.read_time
                    } else {
                        current_counters.read_time + max_value - past_counters.read_time
                    }
                },
                write_time: {
                    if current_counters.write_time >= past_counters.write_time {
                        current_counters.write_time
                    } else {
                        current_counters.write_time + max_value - past_counters.write_time
                    }
                },
                read_merged_count: {
                    if current_counters.read_merged_count >= past_counters.read_merged_count {
                        current_counters.read_merged_count
                    } else {
                        current_counters.read_merged_count + max_value
                            - past_counters.read_merged_count
                    }
                },
                write_merged_count: {
                    if current_counters.write_merged_count >= past_counters.write_merged_count {
                        current_counters.write_merged_count
                    } else {
                        current_counters.write_merged_count + max_value
                            - past_counters.write_merged_count
                    }
                },
                busy_time: {
                    if current_counters.busy_time >= past_counters.busy_time {
                        current_counters.busy_time
                    } else {
                        current_counters.busy_time + max_value - past_counters.busy_time
                    }
                },
            });
        }
    }
    total_disk_io_counters
}

/// Return all mounted disk partitions as a DiskPartitions struct including device,
/// mount point and filesystem type.
///
/// Similarly to “df” command on UNIX.
/// If all parameter is false it tries to distinguish and return physical devices only
/// (e.g. hard disks, cd-rom drives, USB keys) and ignore all others
/// (e.g. memory partitions such as /dev/shm).
pub fn disk_partitions(all: bool) -> Result<Vec<MountedPartition>> {
    let fstypes = read_file(Path::new("/proc/filesystems"))?;
    let fstype = fstype(&fstypes);
    let partitions = read_file(Path::new("/proc/mounts"))?;
    let partitions_lines: Vec<&str> = partitions.lines().collect();
    let mut mounted_partitions: Vec<MountedPartition> = Vec::new();
    for line in partitions_lines {
        let partition_infos: Vec<&str> = line.split_whitespace().collect();
        if partition_infos.len() >= 4 && all {
            mounted_partitions.push(MountedPartition {
                device: String::from(partition_infos[0]),
                mountpoint: String::from(partition_infos[1]),
                fstype: String::from(partition_infos[2]),
                opts: String::from(partition_infos[3]),
            });
        }
        if partition_infos.len() >= 4
            && !all
            && partition_infos[0] != ""
            && fstype.contains(&partition_infos[2])
        {
            mounted_partitions.push(MountedPartition {
                device: String::from(partition_infos[0]),
                mountpoint: String::from(partition_infos[1]),
                fstype: String::from(partition_infos[2]),
                opts: String::from(partition_infos[3]),
            });
        }
        if partition_infos.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("failed to load partition information on /proc/mounts"),
            ));
        }
    }
    Ok(mounted_partitions)
}

/// Return disk usage associated with path.
///
/// Note: UNIX usually reserves 5% disk space which is not accessible
/// by user. In this function "total" and "used" values reflect the
/// total and used disk space whereas "free" and "percent" represent
/// the "free" and "used percent" user disk space.
#[allow(unsafe_code)]
pub fn disk_usage(path: &str) -> Result<DiskUsage> {
    let mut buf: libc::statvfs = unsafe { mem::uninitialized() };
    let path = CString::new(path).unwrap();
    let result = unsafe { libc::statvfs(path.as_ptr(), &mut buf) };
    if result != 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("failed to use statvfs : statvfs return an error code"),
        ));
    }
    let total = buf.f_blocks * buf.f_frsize;
    let avail_to_root = buf.f_bfree * buf.f_frsize;
    let free = buf.f_bavail * buf.f_frsize;
    let used = total - avail_to_root;
    let total_user = used + free;
    let percent = if total_user > 0 {
        used as f64 / total_user as f64 * 100.
    } else {
        0.
    };
    Ok(DiskUsage {
        total,
        used,
        free,
        percent,
    })
}
