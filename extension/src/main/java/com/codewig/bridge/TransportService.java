package com.codewig.bridge;

import com.bitwig.extension.controller.api.ControllerHost;
import com.bitwig.extension.controller.api.Parameter;
import com.bitwig.extension.controller.api.Transport;
import com.google.gson.JsonObject;

import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.function.DoubleConsumer;

/**
 * Thin adapter around Bitwig Transport (play / stop / tempo / position / status).
 * No metronome — we don't live-record from Codewig.
 *
 * <p>Bitwig Parameter observer rules (host throws otherwise):
 * <ul>
 *   <li>Never {@code parameter.addValueObserver} / {@code addRawValueObserver} on Parameter itself
 *       → use {@code parameter.value().addValueObserver(...)} if you need a callback</li>
 *   <li>Or just {@code markInterested()} + {@code getRaw()} (tempo)</li>
 *   <li>BeatTimeValue: {@code addValueObserver}, not raw</li>
 * </ul>
 */
public final class TransportService {
    private final Transport transport;
    private final ControllerHost host;
    private final Parameter tempoParam;
    private final List<DoubleConsumer> positionListeners = new CopyOnWriteArrayList<>();

    private volatile boolean playing;
    /** Fallback BPM if getRaw not ready yet. */
    private volatile double tempo = 120.0;
    /** Playhead in quarter-note beats. */
    private volatile double positionBeats;
    private volatile int tsNumerator = 4;
    private volatile int tsDenominator = 4;

    public TransportService(final ControllerHost host) {
        this.host = host;
        this.transport = host.createTransport();
        this.tempoParam = transport.tempo();

        transport.isPlaying().markInterested();
        transport.isPlaying().addValueObserver(v -> playing = v);

        // Tempo Parameter: do NOT observe Parameter itself (host: use value().addObserver).
        // markInterested + getRaw is enough; optional value() observer only for cache.
        tempoParam.markInterested();
        tempoParam.value().markInterested();
        tempoParam.value().addValueObserver(ignored -> tempo = safeTempoRaw());

        // Beat time (not a Parameter) — addValueObserver only
        transport.playPosition().markInterested();
        transport.playPosition().addValueObserver(v -> {
            positionBeats = v;
            for (final DoubleConsumer listener : positionListeners) {
                listener.accept(v);
            }
        });

        transport.timeSignature().numerator().markInterested();
        transport.timeSignature().numerator().addValueObserver(v -> tsNumerator = Math.max(1, v));
        transport.timeSignature().denominator().markInterested();
        transport.timeSignature().denominator().addValueObserver(v -> tsDenominator = Math.max(1, v));
    }

    private double safeTempoRaw() {
        final double raw = tempoParam.getRaw();
        if (raw >= 20.0 && raw <= 999.0) {
            return raw;
        }
        return tempo;
    }

    public Transport getTransport() {
        return transport;
    }

    public boolean isPlaying() {
        return playing;
    }

    public double getTempo() {
        tempo = safeTempoRaw();
        return tempo;
    }

    public double getPositionBeats() {
        return positionBeats;
    }

    /**
     * Quarter-note beats per bar from project time signature
     * (e.g. 4/4 → 4.0, 6/8 → 3.0).
     */
    public double getBeatsPerBar() {
        return tsNumerator * (4.0 / tsDenominator);
    }

    /** Next bar boundary strictly after {@code pos} (never "this bar"). */
    public double nextBarBeat(final double pos) {
        final double bpb = getBeatsPerBar();
        if (bpb <= 0) {
            return pos + 4.0;
        }
        return Math.floor(pos / bpb + 1e-6) * bpb + bpb;
    }

    public void addPositionListener(final DoubleConsumer listener) {
        if (listener != null) {
            positionListeners.add(listener);
        }
    }

    public void play() {
        transport.play();
    }

    public void stop() {
        transport.stop();
    }

    public void setTempo(final double bpm) {
        if (bpm < 20.0 || bpm > 999.0) {
            throw new IllegalArgumentException("tempo out of range (20–999): " + bpm);
        }
        tempoParam.setRaw(bpm);
        tempo = bpm;
    }

    public JsonObject status(final int port) {
        final JsonObject result = new JsonObject();
        result.addProperty("bitwig", "connected");
        result.addProperty("playing", playing);
        result.addProperty("tempo", getTempo());
        result.addProperty("positionBeats", positionBeats);
        result.addProperty("beatsPerBar", getBeatsPerBar());
        result.addProperty("port", port);
        result.addProperty("hostProduct", host.getHostProduct());
        result.addProperty("hostVersion", host.getHostVersion());
        return result;
    }
}
