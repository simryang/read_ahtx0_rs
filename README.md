# read_ahtx0_rs  
Alternative for python adafruit_ahtx0  

프로젝트 최종 정리: Python + Rust FFI를 이용한 AHTx0 센서 제어 안정화

1. 최초의 문제 상황  
   - **블로킹(Blocking) 현상**  
     - 기존 파이썬 `adafruit_ahtx0` 라이브러리를 사용하여 외부 온습도 센서를 읽는 과정에서, 센서가 응답하지 않을 경우 프로그램 전체가 멈추는 불안정한 문제가 발생했습니다.  
   - **원인**  
     - 해당 파이썬 라이브러리가 I2C 통신 시 타임아웃 처리가 미흡하여, 하드웨어로부터 응답이 없을 때 무한 대기 상태에 빠졌습니다.  
   - **확인**  
     - 간단히 C 코드로 I2C 통신 확인 시 이상이 없었습니다. 즉, Python 모듈의 문제이지 실제 센서 동작에는 이상이 없었습니다.  

2. 목표 설정  
   - "파이썬의 불안정한 센서 제어 부분을, 빠르고 안정적인 Rust 코드로 대체하여 성능과 안정성을 모두 잡는다."  
   - 이를 위해 Rust 코드를 작성하여 공유 라이브러리(`.so` 파일)로 만들고, Python에서는 이 라이브러리를 호출하여 사용(FFI, Foreign Function Interface)하는 방식을 채택했습니다.  

3. 구조  
   ```text
   +--------------------------+      +---------------------------+      +-------------------------------+      +---------------+
   |                          |      |                           |      |                               |      |               |
   |  메인 애플리케이션        |      |  파이썬 래퍼(Wrapper) 모듈 |      |  Rust 공유 라이브러리          |      |    하드웨어   |
   |   (utils.py)             | ---> |     (ahtx0_ffi.py)        | ---> |  (libread_ahtx0_rs.so)        | ---> |    (AHTx0)    |
   |                          |      |                           |      |                               |      |               |
   +--------------------------+      +---------------------------+      +-------------------------------+      +---------------+
            |                                |                                    |
       from ahtx0_ffi                       ctypes.CDLL()                        I2C 통신
       import Ahtx0                       로 `.so` 파일 로드
    ```

4. 핵심 로직: Rust 공유 라이브러리 (libread_ahtx0_rs.so)  
  - 역할
    - 모든 저수준(low-level) 하드웨어 제어를 담당합니다. I2C 통신, 센서 초기화, 상태 확인, 데이터 변환 등 불안정하고 성능이 중요한 모든 작업을 처리합니다.
  - 특징
    - 안정성
      - 센서가 응답하지 않더라도 무한 루프에 빠지지 않고, 정해진 횟수(10)만 재시도한 후 명확한 에러를 반환합니다.
    - 인터페이스
      - Python이 쉽게 호출할 수 있도록 read_ahtx0_sensor()라는 단 하나의 C 호환 함수만 외부에 공개합니다.

5. 연결 다리: 파이썬 래퍼 모듈 (ahtx0_ffi.py)
  - 역할
    - 복잡한 FFI(C 라이브러리 호출) 과정을 추상화하여, 다른 Python 코드에서는 마치 일반적인 파이썬 클래스를 사용하듯 쉽게 Rust 코드를 이용할 수 있도록 “연결 다리” 역할을 합니다.
  - 특징
    - ctypes를 사용하여 libread_ahtx0_rs.so 파일을 로드합니다.
    - Ahtx0 클래스를 제공하여 객체지향적인 사용이 가능합니다.
    - @property를 통해 sensor.temperature와 같이 속성에 접근하는 것만으로 내부적으로 Rust 함수가 호출되도록 하여, 기존 코드와의 호환성을 완벽하게 유지합니다.

6. 빌드
  - 전체 디버그 빌드 (라이브러리 + 바이너리)
    `cargo build`
    - 생성 → `target/debug/`
      - `libread_ahtx0_rs.so` (C 스타일 동적 라이브러리)
      - `deps/libread_ahtx0_rs-*.rlib` (Rust 라이브러리)
      - `read_ahtx0_cli` (CLI 실행 파일)
  - 라이브러리만 (디버그)
    `cargo build --lib`
    - 생성 → `target/debug/`
      - `libread_ahtx0_rs.so`
      - `deps/libread_ahtx0_rs-*.rlib`
  - CLI 실행 파일만 (디버그)
    `cargo build --bin read_ahtx0_cli`
    - 생성 → target/debug/read_ahtx0_cli
  - 전체 릴리즈 빌드 (라이브러리 + 바이너리)
    `cargo build --release`
    - 생성 → target/release/
  - 라이브러리만 (릴리즈)
    `cargo build --release --lib`
    - 생성 → target/release/
  - CLI 실행 파일만 (릴리즈)
    `cargo build --release --bin read_ahtx0_cli`
    - 생성 → target/release/read_ahtx0_cli

7.사용 예
  - 역할
    - 실제 라이브러리를 사용하는 Python 코드입니다.
  - 특징
    - `from ahtx0_ffi import Ahtx0` 구문을 통해 Ahtx0 클래스를 간단히 가져옵니다.
    - `ahtx0 = Ahtx0()` 와 같이 객체를 생성하여 사용합니다.

8.정리
  - 위치
    - `libread_ahtx0_rs.so` 파일과 `ahtx0_ffi.py` 파일은 동일 디렉토리에 위치시켜야 합니다.
  - 제거
    - 사용하려는 Python 소스 안의 import adafruit_ahtx0 구문을 제거합니다.
  - 변경
    - import
      - `from ahtx0_ffi import Ahtx0`
    - instance
      - `ahtx0 = Ahtx0()`
    - 센서 핸들러
      - `adafruit_ahtx0.AHTx0(self._i2c)` 에서 `ahtx0` 으로 변경합니다.
  - 값 접근
    - `ahtx0.temperature`, `ahtx0.relative_humidity` 로 각각 온도, 습도로 사용합니다.

> Note: Rust 입문자가 작성한 코드입니다. 수정 제안 언제든지 환영합니다.  
> (This code was written by a Rust beginner. Suggestions for improvements are always welcome!)
