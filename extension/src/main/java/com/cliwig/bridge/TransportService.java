package com.cliwig.bridge;

import com.bitwig.extension.controller.api.ControllerHost;
import com.bitwig.extension.controller.api.SettableRangedValue;
import com.bitwig.extension.controller.api.Transport;
import com.google.gson.JsonObject;

/**
 * Thin adapter around Bitwig Transport (play / stop / tempo / status).
 * No metronome — we don't live-record from CLIwig.
 */
public final class TransportService {
    private final Transport transport;
    private final ControllerHost host;
    private final SettableRangedValue tempoValue;

    private volatile boolean playing;
    private volatile double tempo = 120.0;

    public TransportService(final ControllerHost host) {
        this.host = host;
        this.transport = host.createTransport();
        this.tempoValue = transport.tempo().value();

        transport.isPlaying().markInterested();
        transport.isPlaying().addValueObserver(v -> playing = v);

        tempoValue.markInterested();
        tempoValue.addRawValueObserver(v -> tempo = v);
    }

    public Transport getTransport() {
        return transport;
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
        tempoValue.setRaw(bpm);
    }

    public JsonObject status(final int port) {
        final JsonObject result = new JsonObject();
        result.addProperty("bitwig", "connected");
        result.addProperty("playing", playing);
        result.addProperty("tempo", tempo);
        result.addProperty("port", port);
        result.addProperty("hostProduct", host.getHostProduct());
        result.addProperty("hostVersion", host.getHostVersion());
        return result;
    }
}
