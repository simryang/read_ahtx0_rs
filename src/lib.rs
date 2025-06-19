// src/lib.rs

use std::ffi::CString;
use std::os::raw::c_char;
use thiserror::Error;

// embedded-hal v1.0.0 버전에 맞는 트레이트를 사용합니다.
use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;
// linux-embedded-hal에서 제공하는 구체적인 타입과 공개된 에러 타입을 사용합니다.
use linux_embedded_hal::{Delay, I2cdev, I2CError};


// --- C와 Python이 이해할 수 있는 결과 구조체 (외부 공개) ---
#[repr(C)]
#[derive(Debug)] // main.rs에서 println!으로 출력하기 위해 추가
pub struct SensorReading {
    pub temperature: f64,
    pub humidity: f64,
    /// 0이면 성공, 음수이면 에러를 의미합니다.
    pub status_code: i32,
}

// --- 내부에서 사용할 에러 타입 (외부 공개) ---
#[derive(Error, Debug)]
pub enum InternalError {
    // I2cdev의 write/read 메소드가 반환하는 공개 에러 타입인 I2CError를 받도록 정의합니다.
    #[error("I2C communication error: {0}")]
    I2c(#[from] I2CError),
    #[error("Sensor could not be calibrated.")]
    CalibrationFailed,
    #[error("Sensor is still busy.")]
    SensorStillBusy,
}

// --- 이 함수가 파이썬에서 호출할 수 있도록 외부에 공개됩니다 ---
#[no_mangle]
pub extern "C" fn read_ahtx0_sensor() -> SensorReading {
    // 내부 로직을 호출하고 결과를 C 호환 구조체로 변환합니다.
    match read_sensor_internal() {
        Ok(reading) => reading,
        Err(e) => {
            // 에러가 발생하면 콘솔에 에러를 출력하고, status_code로 실패를 알립니다.
            eprintln!("[Rust Library Error] {}", e);
            SensorReading {
                temperature: 1000.0,
                humidity: 1000.0,
                status_code: -1, // 일반적인 에러 코드
            }
        }
    }
}

// --- 라이브러리 버전을 반환하는 새로운 FFI 함수 ---
#[no_mangle]
pub extern "C" fn get_library_version() -> *const c_char {
    // Cargo.toml의 패키지 버전을 가져옵니다.
    let version_str = env!("CARGO_PKG_VERSION");
    let c_version = CString::new(version_str).unwrap();
    // CString의 소유권을 C 코드로 넘기고, 메모리 해제는 C 코드(Python)가 책임지도록 합니다.
    c_version.into_raw()
}

// --- get_library_version이 할당한 메모리를 해제하는 함수 ---
#[no_mangle]
pub unsafe extern "C" fn free_string(s: *mut c_char) {
    if !s.is_null() {
        // CString::from_raw을 호출하여 C로부터 소유권을 다시 가져오고,
        // 이 함수의 스코프가 끝날 때 자동으로 메모리가 해제되도록 합니다.
        let _ = CString::from_raw(s);
    }
}


// --- 이 함수를 main.rs에서 호출할 것입니다 ---
pub fn read_sensor_internal() -> Result<SensorReading, InternalError> {
    const I2C_BUS_PATH: &str = "/dev/i2c-1";
    const DEVICE_ADDRESS: u8 = 0x38;

    // --- FIX: 에러 타입 불일치 해결 ---
    // I2cdev::new()는 `linux_embedded_hal::i2cdev::linux::LinuxI2CError`를 반환합니다.
    // 이 에러를 `map_err`를 사용하여 우리가 처리할 수 있는 `InternalError::I2c`로 수동 변환합니다.
    let i2c = I2cdev::new(I2C_BUS_PATH).map_err(|e| InternalError::I2c(I2CError::from(e)))?;
    let delay = Delay;

    let mut sensor = Ahtx0::new(i2c, DEVICE_ADDRESS, delay)?;
    let reading = sensor.read_temperature_humidity()?;

    Ok(SensorReading {
        temperature: reading.temperature,
        humidity: reading.humidity,
        status_code: 0, // 성공
    })
}

struct Ahtx0 {
    i2c: I2cdev,
    delay: Delay,
    address: u8,
}

impl Ahtx0 {
    fn new(i2c: I2cdev, address: u8, delay: Delay) -> Result<Self, InternalError> {
        let mut sensor = Self { i2c, address, delay };
        sensor.soft_reset()?;
        sensor.wait_for_calibration()?;
        Ok(sensor)
    }

    fn soft_reset(&mut self) -> Result<(), InternalError> {
        const CMD_SOFT_RESET: u8 = 0xBA;
        self.i2c.write(self.address, &[CMD_SOFT_RESET])?;
        self.delay.delay_ms(20);
        Ok(())
    }

    fn status(&mut self) -> Result<u8, InternalError> {
        let mut buffer = [0u8; 1];
        self.i2c.read(self.address, &mut buffer)?;
        Ok(buffer[0])
    }

    fn wait_for_calibration(&mut self) -> Result<(), InternalError> {
        for _ in 0..10 {
            const STATUS_CALIBRATED: u8 = 0x08;
            if (self.status()? & STATUS_CALIBRATED) == STATUS_CALIBRATED {
                return Ok(());
            }
            self.delay.delay_ms(10);
        }
        Err(InternalError::CalibrationFailed)
    }

    fn read_temperature_humidity(&mut self) -> Result<RawSensorData, InternalError> {
        const CMD_TRIGGER: [u8; 3] = [0xAC, 0x33, 0x00];
        const STATUS_BUSY: u8 = 0x80;

        self.i2c.write(self.address, &CMD_TRIGGER)?;
        self.delay.delay_ms(80);

        for _ in 0..10 {
            if (self.status()? & STATUS_BUSY) == 0 {
                let mut buffer = [0u8; 6];
                self.i2c.read(self.address, &mut buffer)?;
                return Ok(RawSensorData::from_raw_bytes(buffer));
            }
            self.delay.delay_ms(10);
        }
        Err(InternalError::SensorStillBusy)
    }
}

struct RawSensorData {
    temperature: f64,
    humidity: f64,
}

impl RawSensorData {
    fn from_raw_bytes(data: [u8; 6]) -> Self {
        let raw_humidity = ((data[1] as u32) << 12) | ((data[2] as u32) << 4) | ((data[3] as u32) >> 4);
        let raw_temp = (((data[3] as u32) & 0x0F) << 16) | ((data[4] as u32) << 8) | (data[5] as u32);
        let humidity = (raw_humidity as f64 / 2_f64.powi(20)) * 100.0;
        let temperature = ((raw_temp as f64 / 2_f64.powi(20)) * 200.0) - 50.0;
        Self { temperature, humidity }
    }
}
