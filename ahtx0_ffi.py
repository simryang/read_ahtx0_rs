# ahtx0_ffi.py
import ctypes
import time
from pathlib import Path

class Ahtx0:
    """
    Rust로 작성된 AHTx0 센서 제어 라이브러리를 호출하기 위한 Python 래퍼 클래스.
    성능 최적화를 위해 라이브러리를 처음 필요할 때 한 번만 로드합니다 (Lazy Loading + Singleton).
    """

    # --- Singleton 패턴을 위한 클래스 변수 ---
    _lib = None
    _read_func = None
    _lib_load_attempted = False

    # --- 버전을 저장할 클래스 변수 ---
    __version__ = "unknown"

    # Rust에서 정의한 것과 동일한 구조체를 파이썬에 정의합니다.
    class _SensorReading(ctypes.Structure):
        _fields_ = [
            ("temperature", ctypes.c_double),
            ("humidity", ctypes.c_double),
            ("status_code", ctypes.c_int),
        ]

    def __init__(self):
        """
        초기화 시에는 내부 변수만 설정하고, 라이브러리 로드는 하지 않습니다.
        """
        self._temperature = 1000.0
        self._humidity = 1000.0
        self._last_read_time = 0

        # 클래스가 처음 인스턴스화될 때만 라이브러리를 로드하고 버전을 설정합니다.
        if not self._lib_load_attempted:
            self._load_library_once()

    @classmethod
    def _load_library_once(cls):
        """
        클래스 메소드로, 프로그램 전체에서 단 한 번만 라이브러리를 로드하도록 보장합니다.
        """
        if cls._lib_load_attempted:
            return

        cls._lib_load_attempted = True

        lib_name = "libread_ahtx0_rs.so"
        script_dir = Path(__file__).parent.resolve()
        local_lib_path = script_dir / lib_name

        lib_path_to_load = str(local_lib_path) if local_lib_path.exists() else lib_name

        print(f"Attempting to load library for the first time: '{lib_path_to_load}'")
        try:
            cls._lib = ctypes.CDLL(lib_path_to_load)

            # 센서 읽기 함수 준비
            read_sensor_func = cls._lib.read_ahtx0_sensor
            read_sensor_func.restype = cls._SensorReading
            cls._read_func = read_sensor_func

            # --- 버전 읽기 및 메모리 관리 수정 ---
            get_version_func = cls._lib.get_library_version
            # 1. 반환 타입을 void 포인터로 지정하여 Python의 자동 변환을 막고 원본 포인터를 얻습니다.
            get_version_func.restype = ctypes.c_void_p

            free_string_func = cls._lib.free_string
            # 2. 메모리 해제 함수의 인자 타입도 void 포인터로 일치시킵니다.
            free_string_func.argtypes = [ctypes.c_void_p]

            version_ptr = get_version_func()
            if version_ptr: # NULL 포인터가 아닌지 확인
                try:
                    # 3. void 포인터를 문자열 포인터로 캐스팅하여 값을 읽습니다.
                    char_ptr = ctypes.cast(version_ptr, ctypes.c_char_p)
                    cls.__version__ = char_ptr.value.decode('utf-8')
                finally:
                    # 4. 원본 포인터를 그대로 전달하여 Rust에서 메모리를 해제합니다.
                    free_string_func(version_ptr)

            print(f"Successfully loaded Rust AHTx0 library v{cls.__version__} from: {getattr(cls._lib, '_name', lib_path_to_load)}")

        except OSError as e:
            print(f"\n--- Library Load Failed ---")
            print(f"Error: {e}")
            print(f"Could not load '{lib_name}'.")
            print("Future sensor readings will fail.")
            print("---------------------------\n")

    def _update_reading(self):
        """
        내부적으로 Rust 함수를 호출하여 센서 값을 갱신합니다.
        """
        if not self._read_func:
            return

        now = time.time()
        if now - self._last_read_time < 0.05:
            return

        result = self._read_func()
        if result.status_code == 0:
            self._temperature = result.temperature
            self._humidity = result.humidity
        else:
            self._temperature = 1000.0
            self._humidity = 1000.0
        self._last_read_time = now

    @property
    def temperature(self):
        self._update_reading()
        return self._temperature

    @property
    def relative_humidity(self):
        self._update_reading()
        return self._humidity
