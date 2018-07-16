//! Load disk informations
//! Author : Adrien Gaillard

#![deny(
    missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts,
    trivial_numeric_casts, unstable_features, unused_import_braces, unused_qualifications
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
#[derive(Debug, Copy, Clone)]
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

fn fstype(data: &str) -> Result<Vec<&str>> {
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
    return Ok(fstypes);
}

/// Return mounted disk partitions as a MountedPartition struc.
pub fn disk_partition(all: bool) -> Result<Vec<MountedPartition>> {
    let fstypes = read_file(Path::new("/proc/filesystems"))?;
    let fstype = fstype(&fstypes)?;
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
    return Ok(mounted_partitions);
}

/// Return disk usage associated with path.
/// Note: UNIX usually reserves 5% disk space which is not accessible
/// by user. In this function "total" and "used" values reflect the
/// total and used disk space whereas "free" and "percent" represent
/// the "free" and "used percent" user disk space.
pub fn disk_usage(path: &str) -> Result<DiskUsage> {
    unsafe {
        let mut buf: libc::statvfs = mem::uninitialized();
        let path = CString::new(path).unwrap();
        libc::statvfs(path.as_ptr(), &mut buf);

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
}
