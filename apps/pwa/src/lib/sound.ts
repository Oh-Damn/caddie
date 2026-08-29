import { loadPreferences, PREFS_CHANGE } from './preferences';

let ctx: AudioContext | null = null;
let enabled: boolean | null = null;
let installed = false;

let clickBuffer: AudioBuffer | null = null;
let encoded: Promise<ArrayBuffer | null> | null = null;
let decoding = false;

const CLICK_URL = '/click.mp3';
const CLICK_GAIN = 0.9;

function wanted(): boolean {
  if (enabled === null) {
    try {
      enabled = loadPreferences().sound;
    } catch {
      enabled = false;
    }
  }
  return enabled;
}

function prefetch(): void {
  if (encoded) return;
  encoded = fetch(CLICK_URL)
    .then((res) => (res.ok ? res.arrayBuffer() : null))
    .catch(() => null);
}

function decodeClick(audio: AudioContext): void {
  if (clickBuffer || decoding || !encoded) return;
  decoding = true;
  void encoded
    .then((bytes) => (bytes ? audio.decodeAudioData(bytes.slice(0)) : null))
    .then((buffer) => {
      clickBuffer = buffer;
    })
    .catch(() => {})
    .finally(() => {
      decoding = false;
    });
}

function context(): AudioContext | null {
  if (ctx) {
    if (ctx.state === 'suspended') void ctx.resume();
    return ctx;
  }
  const Ctor = window.AudioContext ?? window.webkitAudioContext;
  if (!Ctor) return null;
  try {
    ctx = new Ctor();
    decodeClick(ctx);
    return ctx;
  } catch {
    return null;
  }
}

function tone(
  freq: number,
  duration: number,
  peak: number,
  at = 0,
  type: OscillatorType = 'sine',
): void {
  const audio = context();
  if (!audio) return;
  const start = audio.currentTime + at;
  const osc = audio.createOscillator();
  const gain = audio.createGain();
  osc.type = type;
  osc.frequency.setValueAtTime(freq, start);

  gain.gain.setValueAtTime(0, start);
  gain.gain.linearRampToValueAtTime(peak, start + 0.006);
  gain.gain.exponentialRampToValueAtTime(0.0001, start + duration);
  osc.connect(gain).connect(audio.destination);
  osc.start(start);
  osc.stop(start + duration + 0.02);
}

export function clickSound(): void {
  if (!wanted()) return;
  const audio = context();
  if (!audio) return;
  if (clickBuffer) {
    const source = audio.createBufferSource();
    source.buffer = clickBuffer;
    const gain = audio.createGain();
    gain.gain.value = CLICK_GAIN;
    source.connect(gain).connect(audio.destination);
    source.start();
    return;
  }
  synthClick(audio);
}

function synthClick(audio: AudioContext): void {
  const at = audio.currentTime;

  const osc = audio.createOscillator();
  const body = audio.createGain();
  osc.type = 'sine';
  osc.frequency.setValueAtTime(170, at);
  osc.frequency.exponentialRampToValueAtTime(62, at + 0.075);
  body.gain.setValueAtTime(0, at);
  body.gain.linearRampToValueAtTime(0.22, at + 0.004);
  body.gain.exponentialRampToValueAtTime(0.0001, at + 0.13);
  osc.connect(body).connect(audio.destination);
  osc.start(at);
  osc.stop(at + 0.15);

  const len = Math.floor(audio.sampleRate * 0.02);
  const buffer = audio.createBuffer(1, len, audio.sampleRate);
  const samples = buffer.getChannelData(0);

  for (let i = 0; i < len; i += 1) {
    samples[i] = (Math.random() * 2 - 1) * (1 - i / len);
  }
  const noise = audio.createBufferSource();
  noise.buffer = buffer;
  const band = audio.createBiquadFilter();
  band.type = 'bandpass';
  band.frequency.value = 1200;
  band.Q.value = 0.8;
  const edge = audio.createGain();
  edge.gain.setValueAtTime(0.05, at);
  edge.gain.exponentialRampToValueAtTime(0.0001, at + 0.03);
  noise.connect(band).connect(edge).connect(audio.destination);
  noise.start(at);
}

export function chimeSound(): void {
  if (!wanted()) return;
  tone(660, 0.18, 0.14);
  tone(990, 0.32, 0.12, 0.14);
}

export function installSound(selector: string): void {
  if (installed) return;
  installed = true;
  prefetch();
  window.addEventListener(PREFS_CHANGE, () => {
    enabled = null;
  });
  document.addEventListener(
    'pointerdown',
    (event) => {
      const target = event.target;
      if (!(target instanceof Element)) return;
      if (!target.closest(selector)) return;
      clickSound();
    },
    { capture: true, passive: true },
  );
}
