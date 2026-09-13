from __future__ import annotations

import ctypes
from ctypes import wintypes
import json
from pathlib import Path
from typing import NamedTuple


class CredentialError(RuntimeError):
    pass


class _Blob(ctypes.Structure):
    _fields_ = [("size", wintypes.DWORD), ("data", ctypes.POINTER(ctypes.c_ubyte))]


class Credentials(NamedTuple):
    api_key: str
    secret: str

    def __repr__(self) -> str:
        return "Credentials(api_key=<redacted>, secret=<redacted>)"


def _crypt(data: bytes, decrypt: bool) -> bytes:
    source_buffer = ctypes.create_string_buffer(data)
    source = _Blob(len(data), ctypes.cast(source_buffer, ctypes.POINTER(ctypes.c_ubyte)))
    target = _Blob()
    library = ctypes.WinDLL("crypt32", use_last_error=True)
    function = library.CryptUnprotectData if decrypt else library.CryptProtectData
    function.argtypes = [ctypes.POINTER(_Blob), ctypes.c_void_p, ctypes.POINTER(_Blob), ctypes.c_void_p, ctypes.c_void_p, wintypes.DWORD, ctypes.POINTER(_Blob)]
    function.restype = wintypes.BOOL
    if not function(ctypes.byref(source), None, None, None, None, 1 if decrypt else 0, ctypes.byref(target)):
        raise CredentialError("Windows DPAPI credential operation failed")
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.LocalFree.argtypes = [ctypes.c_void_p]
    try:
        return ctypes.string_at(target.data, target.size)
    finally:
        kernel.LocalFree(target.data)


def load_credentials(path: Path) -> Credentials:
    if not path.exists():
        raise CredentialError("AUTH_UNAVAILABLE: DPAPI credential store is missing")
    try:
        data = json.loads(_crypt(path.read_bytes(), True))
        key, secret = data["api_key"], data["secret"]
        if not isinstance(key, str) or not isinstance(secret, str) or not key or not secret:
            raise ValueError
        return Credentials(key, secret)
    except (OSError, ValueError, KeyError, TypeError, UnicodeError):
        raise CredentialError("AUTH_UNAVAILABLE: DPAPI credential store could not be read") from None
