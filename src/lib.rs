#![deny(clippy::all)]

//! Crate for fetching and modifying the intel_pstate kernel parameters.
//!
//! # Example
//!
//! ```rust,no_run
//! use intel_pstate::{PState, PStateError};
//!
//! fn main() -> Result<(), PStateError> {
//!     let pstate = PState::new()?;
//!
//!     pstate.set_min_perf_pct(50)?;
//!     pstate.set_max_perf_pct(100)?;
//!     pstate.set_no_turbo(false)?;
//!
//!     Ok(())
//! }
//! ```

#[macro_use]
extern crate smart_default;

use std::{
    fmt::Display,
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    str::FromStr,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PStateError {
    #[error("failed to get min perf pstate value: {0}")]
    GetMinPerf(io::Error),
    #[error("failed to get max perf pstate value: {0}")]
    GetMaxPerf(io::Error),
    #[error("failed to get no turbo pstate value: {0}")]
    GetNoTurbo(io::Error),
    #[error("intel_pstate directory not found")]
    NotFound,
    #[error("failed to set min perf pstate value to {0}: {1}")]
    SetMinPerf(u8, io::Error),
    #[error("failed to set max perf pstate value to {0}: {1}")]
    SetMaxPerf(u8, io::Error),
    #[error("failed to set no turbo pstate value to {0}: {1}")]
    SetNoTurbo(bool, io::Error),
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, SmartDefault)]
/// A set of pstate values that was retrieved, or is to be set.
pub struct PStateValues {
    pub min_perf_pct: u8,
    #[default(100)]
    pub max_perf_pct: u8,
    pub no_turbo:     bool,
}

impl PStateValues {
    pub fn new(min: u8, max: u8, no_turbo: bool) -> Self {
        Self { min_perf_pct: min, max_perf_pct: max, no_turbo }
    }
}

/// Handle for fetching and modifying Intel PState kernel parameters.
///
/// # Note
///
/// - Currently, ony Linux is supported.
/// - Setting parameters will require root permissions.
pub struct PState {
    path: PathBuf,
}

impl PState {
    /// Attempt to fetch a handle to the Intel PState sysfs kernel instance.
    pub fn new() -> Result<PState, PStateError> {
        let path = PathBuf::from("/sys/devices/system/cpu/intel_pstate");
        if path.is_dir() {
            Ok(PState { path })
        } else {
            Err(PStateError::NotFound)
        }
    }

    /// Get the minimum performance percent.
    pub fn min_perf_pct(&self) -> Result<u8, PStateError> {
        parse_file(self.path.join("min_perf_pct")).map_err(PStateError::GetMinPerf)
    }

    /// Set the minimum performance percent.
    pub fn set_min_perf_pct(&self, min: u8) -> Result<(), PStateError> {
        write_file(self.path.join("min_perf_pct"), format!("{}", min))
            .map_err(|why| PStateError::SetMinPerf(min, why))
    }

    /// Get the maximum performance percent.
    pub fn max_perf_pct(&self) -> Result<u8, PStateError> {
        parse_file(self.path.join("max_perf_pct")).map_err(PStateError::GetMaxPerf)
    }

    /// Set the maximum performance percent.
    pub fn set_max_perf_pct(&self, max: u8) -> Result<(), PStateError> {
        write_file(self.path.join("max_perf_pct"), format!("{}", max))
            .map_err(|why| PStateError::SetMaxPerf(max, why))
    }

    /// If true, this signifies that turbo is disabled.
    pub fn no_turbo(&self) -> Result<bool, PStateError> {
        let value =
            parse_file::<u8, _>(self.path.join("no_turbo")).map_err(PStateError::GetNoTurbo)?;
        Ok(value > 0)
    }

    /// Set the no_turbo value; `true` will disable turbo.
    pub fn set_no_turbo(&self, no_turbo: bool) -> Result<(), PStateError> {
        write_file(self.path.join("no_turbo"), if no_turbo { "1" } else { "0" })
            .map_err(|why| PStateError::SetNoTurbo(no_turbo, why))
    }

    pub fn values(&self) -> Result<PStateValues, PStateError> {
        let values = PStateValues {
            min_perf_pct: self.min_perf_pct()?,
            max_perf_pct: self.max_perf_pct()?,
            no_turbo:     self.no_turbo()?,
        };

        Ok(values)
    }

    /// Set all values in the given config.
    pub fn set_values(&self, values: PStateValues) -> Result<(), PStateError> {
        self.set_min_perf_pct(values.min_perf_pct)?;
        self.set_max_perf_pct(values.max_perf_pct)?;
        self.set_no_turbo(values.no_turbo)
    }
}

fn read_file<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut data = String::new();

    {
        let mut file = File::open(path.as_ref())?;
        file.read_to_string(&mut data)?;
    }

    Ok(data)
}

fn write_file<P: AsRef<Path>, S: AsRef<[u8]>>(path: P, data: S) -> io::Result<()> {
    {
        let mut file = OpenOptions::new().write(true).open(path)?;
        file.write_all(data.as_ref())?
    }

    Ok(())
}

fn parse_file<F: FromStr, P: AsRef<Path>>(path: P) -> io::Result<F>
where
    F::Err: Display,
{
    read_file(path)?
        .trim()
        .parse()
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, format!("{}", err)))
}
