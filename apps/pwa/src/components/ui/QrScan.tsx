import { useCallback, useEffect, useRef, useState } from 'react';
import jsQR from 'jsqr';
import { Camera } from 'lucide-react';
import { haptic } from '../../companion/session';

type Props = {
  active: boolean;
  onScan: (value: string) => void;
};

type Detector = {
  detect: (source: ImageBitmapSource) => Promise<{ rawValue: string }[]>;
};

function barcodeDetector(): Detector | null {
  const Ctor = window.BarcodeDetector;
  if (!Ctor) return null;
  try {
    return new Ctor({ formats: ['qr_code'] });
  } catch {
    return null;
  }
}

export function QrScan({ active, onScan }: Props) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const detectorRef = useRef<Detector | null>(null);
  const rafRef = useRef(0);
  const genRef = useRef(0);
  const liveRef = useRef(false);
  const onScanRef = useRef(onScan);
  onScanRef.current = onScan;

  const tappedRef = useRef(false);
  const startingRef = useRef(false);
  const [live, setLive] = useState(false);
  const [prompt, setPrompt] = useState(false);
  const [denied, setDenied] = useState(false);

  const stop = useCallback(() => {
    genRef.current += 1;
    liveRef.current = false;
    cancelAnimationFrame(rafRef.current);
    streamRef.current?.getTracks().forEach((track) => track.stop());
    streamRef.current = null;
    const video = videoRef.current;
    if (video) video.srcObject = null;
    setLive(false);
  }, []);

  const readFrame = useCallback((): string | null => {
    const video = videoRef.current;
    const canvas = canvasRef.current;
    if (!video || !canvas || video.readyState < 2) return null;
    const srcW = video.videoWidth;
    const srcH = video.videoHeight;
    if (!srcW || !srcH) return null;
    const scale = Math.min(1, 480 / Math.max(srcW, srcH));
    const w = Math.max(1, Math.floor(srcW * scale));
    const h = Math.max(1, Math.floor(srcH * scale));
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    if (!ctx) return null;
    ctx.drawImage(video, 0, 0, w, h);
    const image = ctx.getImageData(0, 0, w, h);
    return jsQR(image.data, image.width, image.height)?.data?.trim() || null;
  }, []);

  const loop = useCallback(async () => {
    if (!liveRef.current) return;
    const video = videoRef.current;
    let value: string | null = null;
    const detector = detectorRef.current;
    if (detector && video && video.readyState >= 2) {
      try {
        const codes = await detector.detect(video);
        value = codes[0]?.rawValue?.trim() || null;
      } catch {
        value = readFrame();
      }
    } else {
      value = readFrame();
    }
    if (!liveRef.current) return;
    if (value) {
      haptic('success');
      onScanRef.current(value);
      stop();
      return;
    }
    rafRef.current = requestAnimationFrame(() => {
      void loop();
    });
  }, [readFrame, stop]);

  const start = useCallback(async () => {
    if (liveRef.current || startingRef.current) return;
    const gen = genRef.current;
    startingRef.current = true;
    setDenied(false);
    detectorRef.current = barcodeDetector();
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: false,
        video: { facingMode: { ideal: 'environment' } },
      });
      if (gen !== genRef.current) {
        stream.getTracks().forEach((track) => track.stop());
        return;
      }
      const video = videoRef.current;
      if (!video) {
        stream.getTracks().forEach((track) => track.stop());
        return;
      }
      streamRef.current = stream;
      video.srcObject = stream;
      await video.play();
      if (gen !== genRef.current) {
        stream.getTracks().forEach((track) => track.stop());
        return;
      }
      liveRef.current = true;
      setLive(true);
      setPrompt(false);
      rafRef.current = requestAnimationFrame(() => {
        void loop();
      });
    } catch {
      if (gen !== genRef.current) return;
      setPrompt(true);
      if (tappedRef.current) setDenied(true);
    } finally {
      startingRef.current = false;
    }
  }, [loop]);

  useEffect(() => {
    if (!active) {
      stop();
      return;
    }
    void start();
    const onHide = () => {
      if (document.visibilityState === 'hidden') stop();
    };
    document.addEventListener('visibilitychange', onHide);
    return () => {
      document.removeEventListener('visibilitychange', onHide);
      stop();
    };
  }, [active, start, stop]);

  return (
    <div className="paper-well relative aspect-square w-full overflow-hidden">
      <video
        ref={videoRef}
        className="h-full w-full object-cover"
        playsInline
        muted
        autoPlay
      />
      <canvas ref={canvasRef} className="hidden" />
      {!live ? (
        <button
          type="button"
          className="absolute inset-0 flex min-h-touch flex-col items-center justify-center gap-2 bg-app-well/90 px-4"
          onClick={() => {
            tappedRef.current = true;
            void start();
          }}
        >
          <Camera className="h-8 w-8 text-lcd" strokeWidth={1.8} />
          <span className="type-secondary text-app-text">Scan</span>
          {!window.isSecureContext ? (
            <span className="type-meta text-center text-app-muted">
              Camera needs HTTPS. Enter the code below instead.
            </span>
          ) : denied ? (
            <span className="type-meta text-center text-app-muted">
              Allow camera, then tap again
            </span>
          ) : prompt ? (
            <span className="type-meta text-app-muted">Tap to open the camera</span>
          ) : null}
        </button>
      ) : null}
    </div>
  );
}
