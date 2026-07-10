/**
 * fMP4 (fragmented MP4) init-segment parser.
 *
 * YouTube serves audio as fragmented MP4 (fMP4): a `ftyp` + `moov` init segment
 * followed by a `sidx` (Segment Index) box and one or more `mdat` fragments.
 * To drive Media Source Extensions (MSE) we need to extract, from the init
 * segment, the byte ranges and time ranges of every subsegment listed in the
 * sidx box. That lets us fetch arbitrary time ranges on demand and seek
 * correctly even when HTMLAudioElement.duration would otherwise report
 * Infinity (which is exactly what happens with YouTube m4a streams).
 *
 * Based on the Nuclear player's parser (MIT), adapted for Jellyx Player.
 *
 * Reference: ISO 14496-12:2015 §8.16.3 (Segment Index Box).
 */

/** One subsegment from the sidx box: a byte range plus a time range (seconds). */
export interface SegmentReference {
  /** Inclusive start byte offset within the overall stream. */
  startByte: number;
  /** Inclusive end byte offset within the overall stream. */
  endByte: number;
  /** Start time in seconds (presentation time of the first sample). */
  startTime: number;
  /** End time in seconds (presentation time just past the last sample). */
  endTime: number;
}

/** Parsed index of an fMP4 init segment. */
export interface Fmp4Index {
  /** Byte offset where the init segment ends (i.e. start of the first sidx-referenced
   *  subsegment, typically the byte right after the sidx box). */
  initSegmentEnd: number;
  /** One entry per subsegment listed in the sidx box, in order. */
  segments: SegmentReference[];
  /** Media timescale from the sidx box (ticks per second). */
  timescale: number;
}

/** A discovered top-level ISO BMFF box. */
interface Fmp4Box {
  type: string;
  offset: number;
  size: number;
}

const BOX_HEADER_SIZE = 8;
const EXTENDED_SIZE_MARKER = 1;

/** Little cursor over a Uint8Array reading big-endian ISO BMFF fields. */
class BinaryReader {
  private readonly view: DataView;
  private cursor: number;

  constructor(data: Uint8Array, offset = 0) {
    this.view = new DataView(data.buffer, data.byteOffset, data.byteLength);
    this.cursor = offset;
  }

  get position(): number {
    return this.cursor;
  }

  readUint8(): number {
    const value = this.view.getUint8(this.cursor);
    this.cursor += 1;
    return value;
  }

  readUint16(): number {
    const value = this.view.getUint16(this.cursor);
    this.cursor += 2;
    return value;
  }

  readUint32(): number {
    const value = this.view.getUint32(this.cursor);
    this.cursor += 4;
    return value;
  }

  readUint64(): number {
    const high = this.view.getUint32(this.cursor);
    const low = this.view.getUint32(this.cursor + 4);
    this.cursor += 8;
    // Safe for typical media durations; high * 2^32 + low.
    return high * 0x100000000 + low;
  }

  readAscii(length: number): string {
    let result = '';
    for (let index = 0; index < length; index++) {
      result += String.fromCharCode(this.view.getUint8(this.cursor + index));
    }
    this.cursor += length;
    return result;
  }

  skip(bytes: number): void {
    this.cursor += bytes;
  }

  hasRemaining(bytes: number): boolean {
    return this.cursor + bytes <= this.view.byteLength;
  }
}

/** Walk the byte buffer and return every top-level box (ftyp, moov, sidx, mdat...). */
export function findBoxes(data: Uint8Array): Fmp4Box[] {
  const boxes: Fmp4Box[] = [];
  const reader = new BinaryReader(data);

  while (reader.hasRemaining(BOX_HEADER_SIZE)) {
    const offset = reader.position;

    let size = reader.readUint32();
    const type = reader.readAscii(4);

    if (size === EXTENDED_SIZE_MARKER) {
      // 64-bit extended size follows the type field.
      size = reader.readUint64();
    } else if (size === 0) {
      // Box extends to end of file.
      size = data.length - offset;
    }

    boxes.push({ type, offset, size });

    const nextBox = offset + size;
    reader.skip(nextBox - reader.position);
  }

  return boxes;
}

/**
 * Parse an ISO 14496-12 Segment Index Box (sidx).
 *
 * Layout (§8.16.3):
 *   Box header (8 bytes): size (4) + type 'sidx' (4)
 *   version (1) + flags (3)
 *   reference_ID (4)
 *   timescale (4)
 *   if version == 0:
 *     earliest_presentation_time (4) + first_offset (4)
 *   else:
 *     earliest_presentation_time (8) + first_offset (8)
 *   reserved (2)
 *   reference_count (2)
 *   for each reference:
 *     reference_type(1 bit) + referenced_size(31 bits) (4)
 *     subsegment_duration (4)
 *     starts_with_SAP(1) + SAP_type(3) + SAP_delta_time(28) (4)
 */
export function parseSidx(
  data: Uint8Array,
  boxOffset: number,
  boxSize: number,
): { references: SegmentReference[]; timescale: number } {
  const reader = new BinaryReader(data, boxOffset + BOX_HEADER_SIZE);

  const version = reader.readUint8();
  reader.skip(3); // flags

  reader.skip(4); // reference_ID
  const timescale = reader.readUint32();

  let firstOffset: number;
  if (version === 0) {
    reader.skip(4); // earliest_presentation_time (32-bit)
    firstOffset = reader.readUint32();
  } else {
    reader.skip(8); // earliest_presentation_time (64-bit)
    firstOffset = reader.readUint64();
  }

  reader.skip(2); // reserved
  const referenceCount = reader.readUint16();

  // First subsegment starts right after the sidx box, offset by first_offset.
  let byteOffset = boxOffset + boxSize + firstOffset;
  let timeOffset = 0;
  const references: SegmentReference[] = [];

  for (let index = 0; index < referenceCount; index++) {
    const referencedSize = reader.readUint32() & 0x7fffffff; // bit 31 = type
    const subsegmentDuration = reader.readUint32();
    reader.skip(4); // starts_with_SAP + SAP_type + SAP_delta_time

    const startByte = byteOffset;
    const endByte = byteOffset + referencedSize - 1;
    const startTime = timeOffset / timescale;
    const endTime = (timeOffset + subsegmentDuration) / timescale;

    references.push({ startByte, endByte, startTime, endTime });

    byteOffset += referencedSize;
    timeOffset += subsegmentDuration;
  }

  return { references, timescale };
}

/**
 * Parse an fMP4 init segment buffer and return the segment index.
 *
 * Throws if no sidx box is present (the stream is not fMP4 or the header
 * fetch was too small to contain the sidx).
 */
export function parseInitSegment(data: Uint8Array): Fmp4Index {
  const boxes = findBoxes(data);
  const sidxBox = boxes.find((box) => box.type === 'sidx');

  if (!sidxBox) {
    throw new Error('No sidx box found in init segment');
  }

  const { references, timescale } = parseSidx(data, sidxBox.offset, sidxBox.size);

  return {
    // Init segment is everything up to (and including) the boxes before sidx.
    // sidx.offset is the byte where the sidx box starts, which is exactly
    // where the init segment (ftyp + moov) ends.
    initSegmentEnd: sidxBox.offset,
    segments: references,
    timescale,
  };
}