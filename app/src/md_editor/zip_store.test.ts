import { describe, expect, test } from "bun:test";
import { zipStore } from "./zip_store";

describe("zipStore", () => {
  test("writes stored UTF-8 entries, CRCs, central directory and end record", () => {
    const bytes = zipStore([{ name: "é", data: "" }, { name: "test", data: new TextEncoder().encode("123456789") }]);
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    let pos = 0;
    for (const [name, size, crc] of [["é", 0, 0], ["test", 9, 0xcbf43926]] as const) {
      expect(view.getUint32(pos, true)).toBe(0x04034b50);
      expect(view.getUint16(pos + 6, true)).toBe(0x800);
      expect(view.getUint16(pos + 8, true)).toBe(0);
      expect(view.getUint32(pos + 14, true)).toBe(crc);
      expect(view.getUint32(pos + 18, true)).toBe(size);
      const length = view.getUint16(pos + 26, true);
      expect(new TextDecoder().decode(bytes.slice(pos + 30, pos + 30 + length))).toBe(name);
      pos += 30 + length + size;
    }
    const central = pos;
    let localOffset = 0;
    for (const [name, size, crc] of [["é", 0, 0], ["test", 9, 0xcbf43926]] as const) {
      expect(view.getUint32(pos, true)).toBe(0x02014b50);
      expect(view.getUint16(pos + 8, true)).toBe(0x800);
      expect(view.getUint16(pos + 10, true)).toBe(0);
      expect(view.getUint32(pos + 16, true)).toBe(crc);
      expect(view.getUint32(pos + 20, true)).toBe(size);
      expect(view.getUint32(pos + 24, true)).toBe(size);
      expect(view.getUint32(pos + 42, true)).toBe(localOffset);
      const length = view.getUint16(pos + 28, true);
      expect(new TextDecoder().decode(bytes.slice(pos + 46, pos + 46 + length))).toBe(name);
      expect(new TextDecoder().decode(bytes.slice(localOffset + 30 + length, localOffset + 30 + length + size))).toBe(size ? "123456789" : "");
      localOffset += 30 + length + size;
      pos += 46 + length;
    }
    expect(view.getUint32(pos, true)).toBe(0x06054b50);
    expect(view.getUint16(pos + 10, true)).toBe(2);
    expect(view.getUint32(pos + 12, true)).toBe(pos - central);
    expect(view.getUint32(pos + 16, true)).toBe(central);
    expect(pos + 22).toBe(bytes.length);
  });
  test("writes an empty archive", () => {
    const bytes = zipStore([]);
    expect(bytes.length).toBe(22);
    expect(new DataView(bytes.buffer).getUint32(0, true)).toBe(0x06054b50);
  });
  test("rejects oversized classic ZIP fields", () => {
    expect(() => zipStore([{ name: "x".repeat(65536), data: "" }])).toThrow();
    expect(() => zipStore(Array.from({ length: 65536 }, () => ({ name: "", data: "" })))).toThrow();
  });
});
