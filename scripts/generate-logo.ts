/**
 * Generates the netmeter logo procedurally and writes it to `docs/logo.svg`.
 *
 * The logo is a gauge: 24 hourly bars, each a stacked download/upload bar like the app's daily
 * chart, fanned over a 270° arc around a needle. Bar lengths follow a daily traffic curve (quiet at
 * night, busy in the afternoon) plus seeded noise, so every run produces the same file.
 *
 * Run with `pnpm logo`, which also regenerates the app icons from the SVG.
 */
import { writeFileSync } from "node:fs";

type Bar = {
  /** Clockwise from 12 o'clock, in degrees. */
  angle: number;
  /** Radial length of the download segment, in px. */
  download: number;
  /** Radial length of the upload segment, in px. */
  upload: number;
};

const OUTPUT_PATH: URL = new URL("../docs/logo.svg", import.meta.url);
const SEED: number = 20261111;

// macOS icon grid: an 824 px tile centred on a 1024 px canvas.
const CANVAS_SIZE: number = 1024;
const TILE_INSET: number = 100;
const TILE_RADIUS: number = 185;
const CENTER_X: number = 512;
/** Below the canvas centre: the arc is open at the bottom, so this balances it optically. */
const CENTER_Y: number = 536;

const HOURS: number = 24;
const ARC_DEGREES: number = 270;
const INNER_RADIUS: number = 160;
const MAX_BAR_LENGTH: number = 200;
const MIN_BAR_SHARE: number = 0.38;
const BAR_WIDTH: number = 24;
const SEGMENT_GAP: number = 8;
/** Hour with the least traffic; the curve peaks 12 hours later. */
const QUIETEST_HOUR: number = 4;
/** Weight of the daily curve against noise in a bar's length. */
const CURVE_WEIGHT: number = 0.7;
const MIN_UPLOAD_FRACTION: number = 0.18;
const UPLOAD_FRACTION_SPREAD: number = 0.14;

const NEEDLE_ANGLE: number = 40;
const NEEDLE_WIDTH: number = 22;
const NEEDLE_CLEARANCE: number = 34;
const HUB_RADIUS: number = 36;
const HUB_HOLE_RADIUS: number = 14;

// Series colours match the app's chart (src/App.css).
const DOWNLOAD_COLOR: string = "#3987e5";
const UPLOAD_COLOR: string = "#eb6834";
const NEEDLE_COLOR: string = "#f0efec";
const TILE_TOP_COLOR: string = "#232a38";
const TILE_BOTTOM_COLOR: string = "#0d1016";
const TILE_EDGE_COLOR: string = "rgba(255, 255, 255, 0.08)";
const TILE_EDGE_WIDTH: number = 2;

/** `count` numbers in [0, 1) from the mulberry32 generator seeded with `seed`. */
function randomSequence(seed: number, count: number): number[] {
  let state: number = seed >>> 0;
  return Array.from({ length: count }, (): number => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t: number = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  });
}

/** Relative traffic of an hour: 0 at `QUIETEST_HOUR`, 1 twelve hours later. */
function dailyCurve(hour: number): number {
  return 0.5 - 0.5 * Math.cos((2 * Math.PI * (hour - QUIETEST_HOUR)) / HOURS);
}

function hourlyBars(lengthNoise: number[], uploadNoise: number[]): Bar[] {
  return Array.from({ length: HOURS }, (_, hour): Bar => {
    const activity: number = CURVE_WEIGHT * dailyCurve(hour) + (1 - CURVE_WEIGHT) * lengthNoise[hour];
    // `activity` is in [0, 1), so `share` stays in [MIN_BAR_SHARE, 1).
    const share: number = MIN_BAR_SHARE + (1 - MIN_BAR_SHARE) * activity;
    const total: number = share * MAX_BAR_LENGTH - SEGMENT_GAP;
    const uploadFraction: number = MIN_UPLOAD_FRACTION + UPLOAD_FRACTION_SPREAD * uploadNoise[hour];
    const upload: number = Math.max(BAR_WIDTH, total * uploadFraction);
    return {
      angle: -ARC_DEGREES / 2 + (hour * ARC_DEGREES) / (HOURS - 1),
      download: total - upload,
      upload,
    };
  });
}

function round(value: number): string {
  return String(Math.round(value * 100) / 100);
}

/** Pill pointing away from the centre, covering radii `from`..`to`, rotated to `angle`. */
function radialPill(angle: number, from: number, to: number, color: string): string {
  return `<rect x="${round(-BAR_WIDTH / 2)}" y="${round(-to)}" width="${BAR_WIDTH}" height="${round(to - from)}" rx="${BAR_WIDTH / 2}" fill="${color}" transform="rotate(${round(angle)})"/>`;
}

function renderBar(bar: Bar): string {
  const downloadEnd: number = INNER_RADIUS + bar.download;
  const uploadStart: number = downloadEnd + SEGMENT_GAP;
  return [
    radialPill(bar.angle, INNER_RADIUS, downloadEnd, DOWNLOAD_COLOR),
    radialPill(bar.angle, uploadStart, uploadStart + bar.upload, UPLOAD_COLOR),
  ].join("\n    ");
}

function renderNeedle(): string {
  const tip: number = INNER_RADIUS - NEEDLE_CLEARANCE;
  return [
    `<line x1="0" y1="0" x2="0" y2="${-tip}" stroke="${NEEDLE_COLOR}" stroke-width="${NEEDLE_WIDTH}" stroke-linecap="round" transform="rotate(${NEEDLE_ANGLE})"/>`,
    `<circle r="${HUB_RADIUS}" fill="${NEEDLE_COLOR}"/>`,
    `<circle r="${HUB_HOLE_RADIUS}" fill="${TILE_BOTTOM_COLOR}"/>`,
  ].join("\n    ");
}

function renderLogo(bars: Bar[]): string {
  const tileSize: number = CANVAS_SIZE - 2 * TILE_INSET;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${CANVAS_SIZE}" height="${CANVAS_SIZE}" viewBox="0 0 ${CANVAS_SIZE} ${CANVAS_SIZE}">
  <title>netmeter</title>
  <defs>
    <linearGradient id="tile" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="${TILE_TOP_COLOR}"/>
      <stop offset="1" stop-color="${TILE_BOTTOM_COLOR}"/>
    </linearGradient>
  </defs>
  <rect x="${TILE_INSET}" y="${TILE_INSET}" width="${tileSize}" height="${tileSize}" rx="${TILE_RADIUS}" fill="url(#tile)" stroke="${TILE_EDGE_COLOR}" stroke-width="${TILE_EDGE_WIDTH}"/>
  <g transform="translate(${CENTER_X} ${CENTER_Y})">
    ${bars.map(renderBar).join("\n    ")}
    ${renderNeedle()}
  </g>
</svg>
`;
}

const noise: number[] = randomSequence(SEED, 2 * HOURS);
const svg: string = renderLogo(hourlyBars(noise.slice(0, HOURS), noise.slice(HOURS)));
writeFileSync(OUTPUT_PATH, svg);
console.log({ event: "logo_written", path: OUTPUT_PATH.pathname, bytes: svg.length });
