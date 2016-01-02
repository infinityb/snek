import ctypes
from ctypes import c_uint32, CDLL

surface_capi = CDLL('target/debug/libsurface_capi.so')

class Slice(ctypes.Structure):
    _fields_ = [
        ("ptr", ctypes.c_char_p),
        ("len", ctypes.c_void_p),
    ]

    def as_string(self):
        return self.ptr[:self.len]


class SurfaceFormat(object):
    __slots__ = ['_number']
    def __init__(self, number):
        self._number = number

    @classmethod
    def from_name(cls, name):
        errno = c_uint32(0)
        rv = surface_capi.surface_format_from_name(name, ctypes.byref(errno))
        SurfaceError.raise_for_errno(errno.value)
        return SurfaceFormat(rv)

    def get_name(self):
        surface_capi.surface_format_name.restype = Slice
        return surface_capi.surface_format_name(self._number).as_string()

    def __repr__(self):
        return "SurfaceFormat::{}".format(self.get_name())


class SurfaceError(Exception):
    __slots__ = ['_number']
    def __init__(self, number):
        self._number = number
        super(SurfaceError, self).__init__('errno({}): {}'.format(self._number, self.get_name()))

    @classmethod
    def raise_for_errno(cls, errno):
        if errno > 0:
            raise SurfaceError(errno)

    @classmethod
    def from_name(cls, name):
        errno = c_uint32(0)
        rv = surface_capi.surface_error_from_name(name, ctypes.byref(errno))
        if errno.value > 0:
            raise KeyError("Unknown error name {}".format(name))
        return SurfaceError(rv)

    def get_name(self):
        surface_capi.surface_error_name.restype = Slice
        return surface_capi.surface_error_name(self._number).as_string()

    def errno(self):
        return self._number

    def __eq__(self, other):
        if not isinstance(other, SurfaceError):
            return False
        return self._number == other._number


SurfaceError.BadFormatName = SurfaceError.from_name("BadFormatName")
SurfaceError.InvalidSize = SurfaceError.from_name("InvalidSize")
SurfaceError.InvalidArgument = SurfaceError.from_name("InvalidArgument")

# print SurfaceError.from_name("BadFormatName")
# print SurfaceError.from_name("InvalidSize")
# print SurfaceError.from_name("InvalidArgument")

# try:
#     print SurfaceError.from_name("x")
#     raise RuntimeException("failed to raise")
# except KeyError:
#     pass
# print SurfaceFormat.from_name("rgb24")
# try:
#     print SurfaceFormat.from_name("x")
#     raise RuntimeException("failed to raise")
# except SurfaceError as e:
#     assert e == SurfaceError.BadFormatName

class Surface(object):
    __slots__ = ['_ptr']
    def __init__(self, width, height):
        surface_capi.surface_new_from_buf.argtypes = (
            ctypes.c_uint32, ctypes.c_uint32,
            ctypes.c_char_p, ctypes.c_size_t,
            ctypes.c_char_p,
            ctypes.POINTER(ctypes.c_uint32))
        errno = c_uint32(0)
        self._ptr = 0
        surface_ptr = surface_capi.surface_new_from_buf(
            width, height,
            "\x00\x00\x00\x00" * (width * height),
            width * height * 4,
            "RGBA8888",
            ctypes.byref(errno))
        SurfaceError.raise_for_errno(errno.value)
        self._ptr = surface_ptr
    
    def __del__(self):
        if not hasattr(self, '_ptr'):
            return
        if self._ptr == 0:
            return
        surface_capi.surface_free(self._ptr)


xx = Surface(1920, 1080)

# uint32_t
# uint32_t
# *const u8
# size_t
# *const libc::c_char
# *mut uint32_tc