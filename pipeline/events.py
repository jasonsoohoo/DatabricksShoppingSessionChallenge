"""Expose Rust-owned event and click columns to DuckDB through Arrow buffers."""
import ctypes as ct
import os

import pyarrow as pa


class Buffers(ct.Structure):
    _fields_ = [
        ("rows", ct.c_size_t),
        ("users", ct.c_void_p), ("user_offsets", ct.c_void_p), ("user_bytes", ct.c_size_t),
        ("products", ct.c_void_p), ("product_offsets", ct.c_void_p), ("product_bytes", ct.c_size_t),
        ("times", ct.c_void_p), ("cents", ct.c_void_p),
    ]


class People:
    def __init__(self, links):
        self.lib = ct.CDLL("/app/events.so")
        self.lib.people_open.argtypes = [ct.c_char_p]
        self.lib.people_open.restype = ct.c_void_p
        self.lib.people_close.argtypes = [ct.c_void_p]
        self.lib.people_close.restype = None
        self.handle = self.lib.people_open(os.fsencode(links))
        if not self.handle:
            raise RuntimeError("native identity resolver failed")

    def __del__(self):
        if getattr(self, "handle", None):
            self.lib.people_close(self.handle)


class NativeBatches:
    def __init__(self):
        self.lib = ct.CDLL("/app/events.so")
        self.lib.events_open.argtypes = [ct.c_char_p, ct.c_int64, ct.c_int64, ct.c_size_t, ct.POINTER(ct.c_size_t)]
        self.lib.events_open.restype = ct.c_void_p
        self.lib.events_batch.argtypes = [ct.c_void_p, ct.c_size_t, ct.POINTER(Buffers)]
        self.lib.events_batch.restype = None
        self.lib.events_label.argtypes = [ct.c_void_p, ct.c_void_p, ct.c_size_t]
        self.lib.events_label.restype = None
        self.lib.events_close.argtypes = [ct.c_void_p]
        self.lib.events_close.restype = None

    def __del__(self):
        if getattr(self, "handle", None):
            self.lib.events_close(self.handle)

    def _table(self, names):
        chunks = []
        for index in range(self.count):
            b = Buffers()
            self.lib.events_batch(self.handle, index, ct.byref(b))
            def buffer(address, size):
                return pa.foreign_buffer(address, size, base=self)
            def strings(offsets, address, size):
                return pa.Array.from_buffers(pa.large_string(), b.rows, [
                    None, buffer(offsets, 8 * (b.rows + 1)), buffer(address, size)])
            chunks.append(pa.record_batch([
                strings(b.user_offsets, b.users, b.user_bytes),
                pa.Array.from_buffers(pa.int64(), b.rows, [None, buffer(b.times, b.rows * 8)]),
                pa.Array.from_buffers(pa.int32(), b.rows, [None, buffer(b.cents, b.rows * 4)]),
                strings(b.product_offsets, b.products, b.product_bytes),
            ], names=names))
        return pa.Table.from_batches(chunks)


class EventBatches(NativeBatches):
    def __init__(self, folder, start, end, workers):
        super().__init__()
        count = ct.c_size_t()
        self.handle = self.lib.events_open(os.fsencode(folder), start, end, workers, ct.byref(count))
        if not self.handle:
            raise RuntimeError("native event reader failed")
        self.count = count.value
        self._exported = False

    def labeled_table(self, people, workers):
        # The Arrow buffers below borrow this handle. Never replace their storage
        # after export, even if a caller retains the EventBatches object.
        if self._exported:
            raise RuntimeError("event batches were already exported")
        self._exported = True
        self.lib.events_label(self.handle, people.handle, workers)

        return self._table(["person", "begin_ms", "cents", "product"])


class ClickBatches(NativeBatches):
    def __init__(self, folder, people, codes, start, end, workers):
        super().__init__()
        self.lib.clicks_open.argtypes = [ct.c_char_p, ct.c_void_p, ct.c_char_p,
            ct.c_int64, ct.c_int64, ct.c_size_t, ct.POINTER(ct.c_size_t)]
        self.lib.clicks_open.restype = ct.c_void_p
        count = ct.c_size_t()
        self.handle = self.lib.clicks_open(os.fsencode(folder), people.handle,
            os.fsencode(codes), start, end, workers, ct.byref(count))
        if not self.handle:
            raise RuntimeError("native click reader failed")
        self.count = count.value

    def table(self):
        return self._table(["person", "t", "code", "cid"])
