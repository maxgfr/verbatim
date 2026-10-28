// The edited media, rendered in the page with WebCodecs (Mediabunny): every
// kept range is decoded, moved up the timeline and encoded again, so cuts are
// exact to the frame. Nothing is uploaded. Video becomes MP4 (H.264, or VP9 /
// AV1 when the browser cannot encode H.264); audio alone becomes M4A.

import {
  ALL_FORMATS,
  AudioSampleSink,
  AudioSampleSource,
  BlobSource,
  BufferTarget,
  getFirstEncodableAudioCodec,
  getFirstEncodableVideoCodec,
  Input,
  Mp4OutputFormat,
  Output,
  QUALITY_HIGH,
  VideoSampleSink,
  VideoSampleSource,
} from "mediabunny";
import type { Range } from "../edl/index.ts";
import { tr } from "../i18n.ts";

export interface Rendered {
  blob: Blob;
  ext: "mp4" | "m4a";
}

export async function renderCut(file: File, keeps: Range[], onProgress: (ratio: number) => void, signal: AbortSignal): Promise<Rendered> {
  if (!keeps.length) throw new Error(tr("Nothing would be left: everything is cut.", "Il ne resterait rien : tout est coupé."));
  const input = new Input({ source: new BlobSource(file), formats: ALL_FORMATS });
  const vt = await input.getPrimaryVideoTrack();
  const at = await input.getPrimaryAudioTrack();
  if (!vt && !at) throw new Error(tr("This file has no track the browser can read.", "Ce fichier n'a aucune piste lisible par le navigateur."));
  const target = new BufferTarget();
  const output = new Output({ format: new Mp4OutputFormat({ fastStart: "in-memory" }), target });
  let video: VideoSampleSource | null = null;
  let audio: AudioSampleSource | null = null;
  if (vt) {
    const codec = await getFirstEncodableVideoCodec(["avc", "vp9", "av1"], { width: vt.displayWidth, height: vt.displayHeight });
    if (!codec) throw new Error(tr("This browser cannot encode video. Try Chrome, Edge or Safari, or use the CLI command below.", "Ce navigateur ne sait pas encoder de vidéo. Essayez Chrome, Edge ou Safari, ou la commande CLI ci-dessous."));
    video = new VideoSampleSource({ codec, bitrate: QUALITY_HIGH });
    output.addVideoTrack(video);
  }
  if (at) {
    const codec = await getFirstEncodableAudioCodec(["aac", "opus"], { numberOfChannels: at.numberOfChannels, sampleRate: at.sampleRate });
    if (codec) {
      audio = new AudioSampleSource({ codec, bitrate: QUALITY_HIGH });
      output.addAudioTrack(audio);
    } else if (!video) {
      throw new Error(tr("This browser cannot encode audio.", "Ce navigateur ne sait pas encoder d'audio."));
    }
  }
  await output.start();
  const total = keeps.reduce((n, [a, b]) => n + b - a, 0);
  const done = { video: 0, audio: 0 };
  const report = () => onProgress(Math.min(1, (video ? done.video : done.audio) / total));

  async function copy(sink: VideoSampleSink | AudioSampleSink, source: VideoSampleSource | AudioSampleSource, key: "video" | "audio") {
    let offset = 0;
    for (const [a, b] of keeps) {
      for await (const sample of sink.samples(a, b)) {
        if (signal.aborted) {
          sample.close();
          throw new DOMException("cancelled", "AbortError");
        }
        // A sample that started before the range keeps its place at the range's start.
        sample.setTimestamp(offset + Math.max(0, sample.timestamp - a));
        await (source as VideoSampleSource).add(sample as never);
        sample.close();
        done[key] = offset + Math.min(b - a, Math.max(0, sample.timestamp - offset));
        report();
      }
      offset += b - a;
      done[key] = offset;
    }
    source.close();
  }

  try {
    // Both tracks at once, so the muxer can interleave without holding everything.
    await Promise.all([vt && video ? copy(new VideoSampleSink(vt), video, "video") : null, at && audio ? copy(new AudioSampleSink(at), audio, "audio") : null]);
    await output.finalize();
  } catch (e) {
    await output.cancel().catch(() => {});
    throw e;
  }
  onProgress(1);
  return { blob: new Blob([target.buffer!], { type: video ? "video/mp4" : "audio/mp4" }), ext: video ? "mp4" : "m4a" };
}
