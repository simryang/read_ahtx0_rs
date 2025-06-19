// src/main.rs

// 이 크레이트(프로젝트)의 라이브러리 부분을 가져옵니다.
// read_ahtx0_rs는 Cargo.toml의 [package].name 입니다.
// SensorReading은 read_sensor_internal 함수의 반환 타입으로 추론되므로, 직접 import할 필요가 없습니다.
use read_ahtx0_rs::read_sensor_internal;

fn main() {
    println!("Starting AHTx0 sensor CLI tool...");

    // lib.rs에 정의된 내부 함수를 직접 호출합니다.
    match read_sensor_internal() {
        Ok(reading) => {
            // 성공 시, reading 변수는 자동으로 SensorReading 타입으로 추론됩니다.
            println!("\n--- Sensor Readings ---");
            println!("Temperature: {:.2} °C", reading.temperature);
            println!("Humidity:    {:.2} %RH", reading.humidity);
            println!("---------------------");
        }
        Err(e) => {
            // 실패 시, 에러를 터미널에 출력합니다.
            eprintln!("\n--- Error ---");
            eprintln!("Failed to read sensor: {}", e);
            eprintln!("-------------");
        }
    }
}
