// SPDX-License-Identifier: GPL-3.0-or-later
//! The T2 ambient light sensor (HID 05ac:8262) through hid-sensor-als.
//!
//! The sensor reports millilux with the spec encoded lux unit. Mainline only
//! knows a placeholder unit code, so it drops the exponent and reports scale
//! 1.0. The exponent is therefore taken from the report descriptor whenever
//! the kernel claims 1.0, which stays correct once the kernel is fixed.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};

const IIO_DEVICES: &str = "/sys/bus/iio/devices";
const T2_ALS: &str = "05AC:8262";
const USAGE_PAGE_SENSOR: u32 = 0x20;
const USAGE_ILLUMINANCE: u32 = 0x04d1;

pub struct Als {
    raw: PathBuf,
    factor: f64,
}

impl Als {
    pub fn find() -> Result<Self> {
        for entry in fs::read_dir(IIO_DEVICES).context("reading IIO devices")? {
            let device = entry?.path();
            if read_trimmed(&device.join("name")).as_deref() != Some("als") {
                continue;
            }
            let real = fs::canonicalize(&device)?;
            if !real.to_string_lossy().to_uppercase().contains(T2_ALS) {
                continue;
            }
            let scale: f64 = read_trimmed(&device.join("in_illuminance_scale"))
                .and_then(|text| text.parse().ok())
                .unwrap_or(1.0);
            let exponent = real
                .ancestors()
                .map(|dir| dir.join("report_descriptor"))
                .find(|file| file.exists())
                .and_then(|file| fs::read(file).ok())
                .and_then(|bytes| illuminance_exponent(&bytes))
                .unwrap_or(0);
            let factor = if (scale - 1.0).abs() < 1e-9 && exponent != 0 {
                10f64.powi(exponent)
            } else {
                scale
            };
            return Ok(Als { raw: device.join("in_illuminance_raw"), factor });
        }
        bail!("no T2 ambient light sensor ({T2_ALS}) under {IIO_DEVICES}")
    }

    /// Reading the raw attribute asks the T2 for a fresh sample, which also
    /// bypasses the 15 % / 2 lux gate it applies to its input reports.
    pub fn lux(&self) -> Result<f32> {
        let text = read_trimmed(&self.raw).with_context(|| format!("reading {}", self.raw.display()))?;
        let raw: f64 = text.parse().with_context(|| format!("parsing {text:?}"))?;
        Ok((raw * self.factor) as f32)
    }

    pub fn factor(&self) -> f64 {
        self.factor
    }

    /// Reads on its own thread: a read blocks for the sensor's integration
    /// time of up to 0.7 s, which must never hold up a fade.
    pub fn watch(self, interval: Duration, sender: Sender<f32>) {
        thread::spawn(move || {
            loop {
                match self.lux() {
                    Ok(lux) => {
                        if sender.send(lux).is_err() {
                            break;
                        }
                    }
                    Err(error) => crate::log(&format!("{error:#}")),
                }
                thread::sleep(interval);
            }
        });
    }
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|text| text.trim().to_string())
}

/// Unit exponent of the illuminance input field in a HID report descriptor.
fn illuminance_exponent(descriptor: &[u8]) -> Option<i32> {
    let mut page = 0u32;
    let mut exponent = 0i32;
    let mut usages: Vec<u32> = Vec::new();
    let mut i = 0;
    while i < descriptor.len() {
        let prefix = descriptor[i];
        if prefix == 0xfe {
            let size = *descriptor.get(i + 1)? as usize;
            i += 3 + size;
            continue;
        }
        let size = [0, 1, 2, 4][(prefix & 3) as usize];
        let data = descriptor.get(i + 1..i + 1 + size)?;
        let value = data.iter().rev().fold(0u32, |acc, &byte| acc << 8 | byte as u32);
        match (prefix >> 2) & 3 {
            0 => {
                let tag = prefix >> 4;
                if tag == 0x8 && page == USAGE_PAGE_SENSOR && usages.contains(&USAGE_ILLUMINANCE) {
                    return Some(exponent);
                }
                if matches!(tag, 0x8 | 0x9 | 0xa | 0xb) {
                    usages.clear();
                }
            }
            1 => match prefix >> 4 {
                0x0 => page = value,
                0x5 => exponent = if value & 0x8 != 0 { value as i32 - 16 } else { value as i32 },
                _ => {}
            },
            2 if prefix >> 4 == 0 => usages.push(if size == 4 { value & 0xffff } else { value }),
            _ => {}
        }
        i += 1 + size;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Report descriptor read from a MacBookAir9,1.
    const MBA91: &str = "05200941a101850105200a16031501250275089501a1020a40080a4108b100c0\
        0a0e0316c80026102775109501b1020a09031501250175089501a1020a3008b1\
        00c00a19031501250475089501a1020a50080a51080a52080a5508b100c00a18\
        0316018026ff7f7510950a5500b1020a01021501250775089501a1020a00080a\
        01080a02080a03080a04080a05080a0608b100c00a0403150026102775109501\
        b1020ad1e4550e150026102775209501b1420ad114550d15002700e1f5057520\
        9501b1420ad31455003500450115002700e1f50575209501b14205200a020235\
        0045001501250175089501a1020a11080a13088100c00ad10467e1000001550d\
        15002700e1f5057520950181020ad2046701000100550015002740420f007520\
        950181020ad404650015002700e1f50555087520950181020ad50415002700e1\
        f5055508752095018102c0";

    fn bytes(hex: &str) -> Vec<u8> {
        let clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        (0..clean.len()).step_by(2).map(|i| u8::from_str_radix(&clean[i..i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn exponent_of_the_t2_sensor_is_milli() {
        assert_eq!(illuminance_exponent(&bytes(MBA91)), Some(-3));
    }
}
