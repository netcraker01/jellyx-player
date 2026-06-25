/**
 * Segment-based player for YouTube fMP4 audio.
 *
 * YouTube serves audio as fragmented MP4 (m4a) streams. When loaded directly
 * into an HTMLAudioElement, `audio.duration` returns Infinity and seeking via
 * `currentTime` does not work.
 *
 * This module uses a Blob-based approach:
 *   1. Fetch the first 8KB to parse ftyp + moov + sidx
 *   2. Extract segment references (byte ranges + time ranges)
 *   3. Fetch init segment + media segments, combine into a Blob, create URL
 *   4. For seek: fetch init + target segment, create new Blob URL
 *
 * SoundCloud does NOT go through this - its MP3 streams work fine with direct
 * audio.src + currentTime seeking.
 */

import { parseInitSegment, type SegmentReference } from './fmp4Parser';

const HEADER_FETCH_SIZE = 8192;
const SEEK_PREFETCH_COUNT = 3;

interface SegmentPlayerState {
  segments: SegmentReference[];
  initSegment: Uint8Array;
  url: string;
  currentBlobUrl: string | null;
}

const playerStates = new WeakMap<HTMLAudioElement, SegmentPlayerState>();

function findSegmentForTime(time: number, segments: SegmentReference[]): number {
  for (let i = 0; i < segments.length; i++) {
    if (time >= segments[i].startTime && time < segments[i].endTime) {
      return i;
    }
  }
  if (segments.length > 0 && time >= segments[segments.length - 1].endTime) {
    return segments.length - 1;
  }
  return 0;
}

async function fetchRange(url: string, start: number, end: number): Promise<Uint8Array> {
  const response = await fetch(url, {
    headers: { Range: `bytes=${start}-${end}` },
  });
  if (!response.ok && response.status !== 206) {
    throw new Error(`Fetch failed: ${response.status}`);
  }
  const buffer = await response.arrayBuffer();
  return new Uint8Array(buffer);
}

/**
 * Initialize segment-based playback for YouTube m4a.
 * Fetches header, parses sidx, fetches first segment, creates Blob URL.
 */
export async function initSegmentPlayer(
  audio: HTMLAudioElement,
  url: string,
): Promise<void> {
  destroySegmentPlayer(audio);

  const headerBytes = await fetchRange(url, 0, HEADER_FETCH_SIZE - 1);
  const index = parseInitSegment(headerBytes);

  if (index.segments.length === 0) {
    throw new Error('No media segments found');
  }

  const initSegment = headerBytes.slice(0, index.initSegmentEnd);
  const segments = index.segments;

  // Fetch first segment
  const firstSeg = segments[0];
  const firstSegData = await fetchRange(url, firstSeg.startByte, firstSeg.endByte);

  // Combine init + first segment into a Blob
  const combined = new Uint8Array(initSegment.length + firstSegData.length);
  combined.set(initSegment, 0);
  combined.set(firstSegData, initSegment.length);

  const blob = new Blob([combined], { type: 'audio/mp4' });
  const blobUrl = URL.createObjectURL(blob);
  audio.src = blobUrl;

  playerStates.set(audio, {
    segments,
    initSegment,
    url,
    currentBlobUrl: blobUrl,
  });
}

/**
 * Seek to a position by fetching the right segment and creating a new Blob URL.
 */
export async function handleSegmentSeek(
  audio: HTMLAudioElement,
  position: number,
): Promise<void> {
  const state = playerStates.get(audio);
  if (!state) return;

  const targetIndex = findSegmentForTime(position, state.segments);
  if (targetIndex < 0 || targetIndex >= state.segments.length) return;

  // Revoke old blob URL
  if (state.currentBlobUrl) {
    URL.revokeObjectURL(state.currentBlobUrl);
  }

  // Fetch init + target segment + a few lookahead segments
  const endIndex = Math.min(targetIndex + SEEK_PREFETCH_COUNT, state.segments.length);
  const segmentsToFetch = state.segments.slice(targetIndex, endIndex);

  // Calculate total size
  let totalSize = state.initSegment.length;
  for (const seg of segmentsToFetch) {
    totalSize += seg.endByte - seg.startByte + 1;
  }

  // Combine all into one buffer
  const combined = new Uint8Array(totalSize);
  let offset = 0;
  combined.set(state.initSegment, offset);
  offset += state.initSegment.length;

  for (const seg of segmentsToFetch) {
    const data = await fetchRange(state.url, seg.startByte, seg.endByte);
    combined.set(data, offset);
    offset += data.length;
  }

  const blob = new Blob([combined], { type: 'audio/mp4' });
  const blobUrl = URL.createObjectURL(blob);

  state.currentBlobUrl = blobUrl;
  audio.src = blobUrl;
  // Set currentTime to the position within this segment
  audio.currentTime = position - segmentsToFetch[0].startTime;
  await audio.play();
}

export function isSegmentPlayerActive(audio: HTMLAudioElement): boolean {
  return playerStates.has(audio);
}

export function destroySegmentPlayer(audio: HTMLAudioElement): void {
  const state = playerStates.get(audio);
  if (state) {
    if (state.currentBlobUrl) {
      URL.revokeObjectURL(state.currentBlobUrl);
    }
    playerStates.delete(audio);
  }
}