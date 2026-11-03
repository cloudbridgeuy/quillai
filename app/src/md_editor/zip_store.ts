const encoder = new TextEncoder();
const limit = 0xffffffff;

function crc32(data: Uint8Array): number {
  let crc = 0xffffffff;
  for (const byte of data) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
  }
  return (crc ^ 0xffffffff) >>> 0;
}

export function zipStore(files: { name: string; data: Uint8Array | string }[]): Uint8Array {
  if (files.length > 0xffff) throw new RangeError("Too many ZIP entries");
  const entries = files.map(file => ({
    name: encoder.encode(file.name),
    data: typeof file.data === "string" ? encoder.encode(file.data) : file.data,
  }));
  let localSize = 0;
  let centralSize = 0;
  for (const entry of entries) {
    if (entry.name.length > 0xffff || entry.data.length > limit) throw new RangeError("ZIP entry too large");
    localSize += 30 + entry.name.length + entry.data.length;
    centralSize += 46 + entry.name.length;
    if (localSize > limit || centralSize > limit || localSize + centralSize > limit) throw new RangeError("ZIP archive too large");
  }
  const bytes = new Uint8Array(localSize + centralSize + 22);
  const view = new DataView(bytes.buffer);
  let local = 0;
  let central = localSize;
  for (const { name, data } of entries) {
    const crc = crc32(data);
    view.setUint32(local, 0x04034b50, true);
    view.setUint16(local + 4, 20, true);
    view.setUint16(local + 6, 0x800, true);
    view.setUint16(local + 12, 33, true);
    view.setUint32(local + 14, crc, true);
    view.setUint32(local + 18, data.length, true);
    view.setUint32(local + 22, data.length, true);
    view.setUint16(local + 26, name.length, true);
    bytes.set(name, local + 30);
    bytes.set(data, local + 30 + name.length);
    view.setUint32(central, 0x02014b50, true);
    view.setUint16(central + 4, 20, true);
    view.setUint16(central + 6, 20, true);
    view.setUint16(central + 8, 0x800, true);
    view.setUint16(central + 14, 33, true);
    view.setUint32(central + 16, crc, true);
    view.setUint32(central + 20, data.length, true);
    view.setUint32(central + 24, data.length, true);
    view.setUint16(central + 28, name.length, true);
    view.setUint32(central + 42, local, true);
    bytes.set(name, central + 46);
    local += 30 + name.length + data.length;
    central += 46 + name.length;
  }
  view.setUint32(central, 0x06054b50, true);
  view.setUint16(central + 8, entries.length, true);
  view.setUint16(central + 10, entries.length, true);
  view.setUint32(central + 12, centralSize, true);
  view.setUint32(central + 16, localSize, true);
  return bytes;
}
