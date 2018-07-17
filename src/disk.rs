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
    device: String,

    /// This field describes the block special device or remote filesystem to be mounted.
    mountpoint: String,

    /// This field describes the type of the filesystem.
    fstype: String,

    /// This field describes the mount options associated with the filesystem.
    opts: String,
}

/// Struct that contains disk usage informations
#[derive(Copy, Clone, Debug)]
pub struct DiskUsage {
    /// Total disk in bytes
    total: u64,

    /// Disk used part in bytes
    used: u64,

    /// Disk free part in bytes
    free: u64,

    /// Percentage of used disk
    percent: f64,
}

/// Disk counter struct
#[derive(Clone, Copy, Debug)]
pub struct DiskIOCounters {
    /// Number of reads
    read_count: u64,

    /// Number of writes
    write_count: i64,

    /// Number of bytes read
    read_bytes: u64,

    /// Number of bytes written
    write_bytes: u64,

    /// Time spent reading from disk (in milliseconds)
    read_time: u64,

    /// Time spent writing to disk (in milliseconds)
    write_time: u64,

    /// Time spent doing actual I/Os (in milliseconds)
    read_merged_count: u64,

    /// Number of merged reads
    write_merged_count: u64,

    /// Number of merged writes
    busy_time: u64,
}

/// Disk counter struct to use nowrap mode
#[derive(Clone, Copy, Debug)]
pub struct DiskIOCountersNoWrap {
    /// Save the total of counters
    disk_io_counters: DiskIOCounters,

    /// Save the values of the last call of disk_io_counters
    disk_io_counters_last_call: DiskIOCounters,
}

impl DiskIOCountersNoWrap {
    /// Initialize a DiskIOCountersNoWrap struct
    pub fn new() -> DiskIOCountersNoWrap {
        DiskIOCountersNoWrap {
            disk_io_counters: DiskIOCounters {
                read_count: 0,
                write_count: 0,
                read_bytes: 0,
                write_bytes: 0,
                read_time: 0,
                write_time: 0,
                read_merged_count: 0,
                write_merged_count: 0,
                busy_time: 0,
            },
            disk_io_counters_last_call: DiskIOCounters {
                read_count: 0,
                write_count: 0,
                read_bytes: 0,
                write_bytes: 0,
                read_time: 0,
                write_time: 0,
                read_merged_count: 0,
                write_merged_count: 0,
                busy_time: 0,
            },
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
