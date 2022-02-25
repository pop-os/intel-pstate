// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MIT

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
//!     let _ = pstate.set_hwp_dynamic_boost(true);
//!     pstate.set_min_perf_pct(50)?;
//!     pstate.set_max_perf_pct(100)?;
//!     pstate.set_no_turbo(false)?;
//!
//!     Ok(())
//! }
//! ```

use derive_setters::Setters;
use smart_default::SmartDefault;
use thiserror::Error;

use std::{
    fmt::Display,
    fs::{self, File},
    io::{self, Write},
    path::Path,
    str::FromStr,
};

const HWP_DYNAMIC_BOOST: &str = "hwp_dynamic_boost";
const MAX_PERF_PCT: &str = "max_perf_pct";
const MIN_PERF_PCT: &str = "min_perf_pct";
const NO_TURBO: &str = "no_turbo";

#[derive(Debug, Error)]
pub enum PStateError {
    #[error("failed to get {} pstate value", src)]
    GetValue {
        src: &'static str,
        source: io::Error,
    },

    #[error("intel_pstate directory not found")]
    NotFound,

    #[error("failed to set {} pstate value", src)]
    SetValue {
        src: &'static str,
        source: io::Error,
    },
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Setters, SmartDefault)]
/// A set of pstate values that was retrieved, or is to be set.
pub struct PStateValues {
    #[setters(strip_option)]
    pub hwp_dynamic_boost: Option<bool>,
    pub min_perf_pct: u8,
    #[default(100)]
    pub max_perf_pct: u8,
    pub no_turbo: bool,
}

/// Handle for fetching and modifying Intel PState kernel parameters.
///
/// # Note
///
/// - Currently, ony Linux is supported.
/// - Setting parameters will require root permissions.
pub struct PState {
    path: &'static str,
}

impl PState {
    /// Attempt to fetch a handle to the Intel PState sysfs kernel instance.
    pub fn new() -> Result<PState, PStateError> {
        let path = "/sys/devices/system/cpu/intel_pstate/";
        if Path::new(path).is_dir() {
            Ok(PState { path })
        } else {
            Err(PStateError::NotFound)
        }
    }

    fn file(&self, file: &str) -> String {
        [self.path, file].concat()
    }

    /// Get the status of HWP dynamic boost, if it is available.
    pub fn hwp_dynamic_boost(&self) -> Result<Option<bool>, PStateError> {
        let file = self.file(HWP_DYNAMIC_BOOST);

        if Path::new(&*file).exists() {
            return parse_file::<u8>(&file)
                .map(|v| Some(v == 1))
                .map_err(|source| PStateError::GetValue {
                    src: HWP_DYNAMIC_BOOST,
                    source,
                });
        }

        Ok(None)
    }

    /// Set the HWP dynamic boost status.
    pub fn set_hwp_dynamic_boost(&self, boost: bool) -> Result<(), PStateError> {
        write_value(&self.file(HWP_DYNAMIC_BOOST), if boost { "1" } else { "0" }).map_err(
            |source| PStateError::SetValue {
                src: HWP_DYNAMIC_BOOST,
                source,
            },
        )
    }

    /// Get the minimum performance percent.
    pub fn min_perf_pct(&self) -> Result<u8, PStateError> {
        parse_file(&self.file(MIN_PERF_PCT)).map_err(|source| PStateError::GetValue {
            src: MIN_PERF_PCT,
            source,
        })
    }

    /// Set the minimum performance percent.
    pub fn set_min_perf_pct(&self, min: u8) -> Result<(), PStateError> {
        write_value(&self.file(MIN_PERF_PCT), min).map_err(|source| PStateError::SetValue {
            src: MIN_PERF_PCT,
            source,
        })
    }

    /// Get the maximum performance percent.
    pub fn max_perf_pct(&self) -> Result<u8, PStateError> {
        parse_file(&self.file(MAX_PERF_PCT)).map_err(|source| PStateError::GetValue {
            src: MAX_PERF_PCT,
            source,
        })
    }

    /// Set the maximum performance percent.
    pub fn set_max_perf_pct(&self, max: u8) -> Result<(), PStateError> {
        write_value(&self.file(MAX_PERF_PCT), max).map_err(|source| PStateError::SetValue {
            src: MAX_PERF_PCT,
            source,
        })
    }

    /// If true, this signifies that turbo is disabled.
    pub fn no_turbo(&self) -> Result<bool, PStateError> {
        let value =
            parse_file::<u8>(&self.file(NO_TURBO)).map_err(|source| PStateError::GetValue {
                src: NO_TURBO,
                source,
            })?;
        Ok(value > 0)
    }

    /// Set the no_turbo value; `true` will disable turbo.
    pub fn set_no_turbo(&self, no_turbo: bool) -> Result<(), PStateError> {
        write_value(&self.file(NO_TURBO), if no_turbo { "1" } else { "0" }).map_err(|source| {
            PStateError::SetValue {
                src: NO_TURBO,
                source,
            }
        })
    }

    /// Get current PState values.
    pub fn values(&self) -> Result<PStateValues, PStateError> {
        let values = PStateValues {
            min_perf_pct: self.min_perf_pct()?,
            max_perf_pct: self.max_perf_pct()?,
            no_turbo: self.no_turbo()?,
            hwp_dynamic_boost: self.hwp_dynamic_boost()?,
        };

        Ok(values)
    }

    /// Set all values in the given config.
    pub fn set_values(&self, values: PStateValues) -> Result<(), PStateError> {
        if let Some(boost) = values.hwp_dynamic_boost {
            let _ = self.set_hwp_dynamic_boost(boost);
        }

        self.set_min_perf_pct(values.min_perf_pct)?;
        self.set_max_perf_pct(values.max_perf_pct)?;
        self.set_no_turbo(values.no_turbo)
    }
}

fn parse_file<F: FromStr>(path: &str) -> io::Result<F>
where
    F::Err: Display,
{
    fs::read_to_string(path)?
        .trim()
        .parse()
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, format!("{}", err)))
}

/// Write a value that implements `Display` to a file
fn write_value<V: Display>(path: &str, value: V) -> io::Result<()> {
    write!(File::create(path)?, "{}", value)
}
