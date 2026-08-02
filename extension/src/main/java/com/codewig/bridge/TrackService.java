package com.codewig.bridge;

import com.bitwig.extension.controller.api.Application;
import com.bitwig.extension.controller.api.ControllerHost;
import com.bitwig.extension.controller.api.CursorTrack;
import com.bitwig.extension.controller.api.Track;
import com.bitwig.extension.controller.api.TrackBank;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;

import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;

/**
 * Tracks: create (with insert index), list, select, delete, move, multi mute/solo.
 * Clip-launcher slots: {@link #SCENE_SLOTS} scenes per track (live/performance).
 * Timed mute: optional quantize to next bar + auto-invert after N bars.
 */
public final class TrackService {
    /** Bank window large enough for typical projects (flat list). */
    public static final int BANK_SIZE = 128;
    /** Clip launcher slots (scenes) visible per track. */
    public static final int SCENE_SLOTS = 16;

    private final ControllerHost host;
    private final Application application;
    private final TrackBank trackBank;
    private final CursorTrack cursorTrack;
    private final TransportService transport;
    private final List<ScheduledMute> muteQueue = new ArrayList<>();

    private volatile int trackCount;

    public TrackService(final ControllerHost host, final TransportService transport) {
        this.host = host;
        this.transport = transport;
        this.application = host.createApplication();
        // numSends=0, numScenes=SCENE_SLOTS for clip launcher
        this.trackBank = host.createTrackBank(BANK_SIZE, 0, SCENE_SLOTS, true);
        this.cursorTrack = host.createCursorTrack("CODEWIG_CURSOR", "Codewig Cursor", 0, SCENE_SLOTS, true);

        trackBank.itemCount().markInterested();
        trackBank.itemCount().addValueObserver(c -> trackCount = c);

        cursorTrack.name().markInterested();
        cursorTrack.position().markInterested();
        cursorTrack.exists().markInterested();

        for (int i = 0; i < BANK_SIZE; i++) {
            final Track t = trackBank.getItemAt(i);
            t.exists().markInterested();
            t.name().markInterested();
            t.position().markInterested();
            t.trackType().markInterested();
            t.mute().markInterested();
            t.solo().markInterested();
            // Parameter: interest/observe via .value(), not Parameter itself
            t.volume().value().markInterested();
        }

        // Beat-time mute queue (quantize / auto-unmute)
        transport.addPositionListener(this::processMuteQueue);
    }

    public CursorTrack getCursorTrack() {
        return cursorTrack;
    }

    public TrackBank getTrackBank() {
        return trackBank;
    }

    public JsonObject list() {
        ensureScrolledToStart();
        final JsonArray tracks = new JsonArray();
        for (int i = 0; i < BANK_SIZE; i++) {
            final Track t = trackBank.getItemAt(i);
            if (!t.exists().get()) {
                continue;
            }
            tracks.add(trackJson(t, t.position().get()));
        }
        final JsonObject result = new JsonObject();
        result.add("tracks", tracks);
        result.addProperty("count", tracks.size());
        result.addProperty("selected", cursorTrack.exists().get() ? cursorTrack.name().get() : "");
        result.addProperty("selectedIndex", cursorTrack.exists().get() ? cursorTrack.position().get() : -1);
        return result;
    }

    /**
     * @param type instrument | audio | effect
     * @param at   insert index, or -1 for end
     * @param name optional display name
     */
    public JsonObject create(final String type, final int at, final String name) {
        final int before = countExistingTracks();
        final String kind = type == null ? "instrument" : type.toLowerCase();

        switch (kind) {
            case "instrument":
            case "inst":
                application.createInstrumentTrack(at);
                break;
            case "audio":
                application.createAudioTrack(at);
                break;
            case "effect":
            case "fx":
                application.createEffectTrack(at);
                break;
            default:
                throw new IllegalArgumentException("unknown track type: " + type + " (instrument|audio|effect)");
        }

        // Bitwig updates the track bank asynchronously, so we poll until the new
        // track appears at the expected index. For now naming at creation time is
        // best-effort; use `track rename` if it does not stick.
        final int insertIndex = at < 0 ? before : Math.min(at, before);
        host.scheduleTask(() -> applyToCreatedTrack(insertIndex, name, 30), 50);

        final JsonObject result = new JsonObject();
        result.addProperty("type", kind);
        result.addProperty("at", at);
        if (name != null && !name.isBlank()) {
            result.addProperty("name", name.trim());
        }
        result.addProperty("note", "track created; name/select best-effort via polling");
        return result;
    }

    private void applyToCreatedTrack(final int insertIndex, final String name, final int attemptsLeft) {
        ensureScrolledToStart();
        final Track t = findByAbsolutePosition(insertIndex);
        if (t != null) {
            // Wait until Bitwig has assigned the default name (e.g. "Inst 3"). Setting the
            // name too early can be overwritten by Bitwig's own initialization.
            final String currentName = t.name().get();
            if (currentName == null || currentName.isBlank()) {
                if (attemptsLeft > 0) {
                    host.scheduleTask(() -> applyToCreatedTrack(insertIndex, name, attemptsLeft - 1), 50);
                }
                return;
            }

            // Make the track visible and selected in all relevant views before renaming.
            // This matches the pattern used by DrivenByMoss and avoids setting the name
            // on a track that is not yet fully realized in the UI.
            t.selectInEditor();
            t.selectInMixer();
            t.makeVisibleInArranger();
            t.makeVisibleInMixer();

            if (name != null && !name.isBlank()) {
                t.name().set(name.trim());
            }
            return;
        }
        if (attemptsLeft > 0) {
            host.scheduleTask(() -> applyToCreatedTrack(insertIndex, name, attemptsLeft - 1), 50);
        }
    }

    public JsonObject select(final String ref) {
        final Track t = resolve(ref);
        t.selectInMixer();
        final JsonObject result = new JsonObject();
        result.addProperty("name", t.name().get());
        result.addProperty("index", t.position().get());
        return result;
    }

    public JsonObject rename(final String ref, final String name) {
        if (name == null || name.isBlank()) {
            throw new IllegalArgumentException("new track name required");
        }
        final Track t = resolve(ref);
        final String oldName = t.name().get();
        final int index = t.position().get();
        t.name().set(name.trim());
        final JsonObject result = new JsonObject();
        result.addProperty("oldName", oldName);
        result.addProperty("newName", name.trim());
        result.addProperty("index", index);
        return result;
    }

    public JsonObject delete(final String ref) {
        final Track t = resolve(ref);
        final String name = t.name().get();
        final int index = t.position().get();
        t.deleteObject();
        final JsonObject result = new JsonObject();
        result.addProperty("deleted", name);
        result.addProperty("index", index);
        return result;
    }

    /**
     * Mute several tracks at once: refs = names and/or indices.
     */
    public JsonObject muteMany(final String[] refs, final boolean on) {
        return muteMany(refs, on, null, null);
    }

    /**
     * Mute with optional timing.
     *
     * @param bars quantize duration: after primary mute state, invert after N bars (null = no invert)
     * @param q    {@code "bar"} / {@code "1"} = apply primary at next bar; else immediate
     */
    public JsonObject muteMany(final String[] refs, final boolean on, final Integer bars, final String q) {
        if (refs == null || refs.length == 0) {
            throw new IllegalArgumentException("need at least one track ref (index or name)");
        }
        if (bars != null && bars < 1) {
            throw new IllegalArgumentException("bars must be >= 1");
        }

        final boolean wantBar = q != null && ("bar".equalsIgnoreCase(q.trim()) || "1".equals(q.trim()));
        final boolean playing = transport.isPlaying();
        final double now = transport.getPositionBeats();
        final double bpb = transport.getBeatsPerBar();

        final JsonObject result = new JsonObject();
        result.addProperty("on", on);
        result.addProperty("playing", playing);
        result.addProperty("beatsPerBar", bpb);
        result.addProperty("positionBeats", now);

        // Resolve track display once for response
        final JsonArray targets = new JsonArray();
        for (final String ref : refs) {
            final Track t = resolve(ref);
            final JsonObject o = new JsonObject();
            o.addProperty("index", t.position().get());
            o.addProperty("name", t.name().get());
            targets.add(o);
        }
        result.add("tracks", targets);

        double primaryAt = now;
        boolean primaryScheduled = false;

        if (wantBar && playing) {
            primaryAt = transport.nextBarBeat(now);
            enqueueMute(refs, on, primaryAt);
            primaryScheduled = true;
            result.addProperty("primaryAtBeat", primaryAt);
            result.addProperty("q", "bar");
        } else {
            applyMuteRefs(refs, on);
            result.addProperty("primary", "now");
            if (wantBar && !playing) {
                result.addProperty("note", "transport not playing — mute applied immediately");
            }
        }

        if (bars != null) {
            result.addProperty("bars", bars);
            final double invertAt = primaryAt + bars * bpb;
            if (playing || primaryScheduled) {
                // Musical time: invert when playhead reaches invertAt
                // If primary was immediate but transport stopped later, wall-clock fallback:
                if (playing) {
                    enqueueMute(refs, !on, invertAt);
                    result.addProperty("invertAtBeat", invertAt);
                    result.addProperty("invert", !on);
                } else {
                    // not playing: primary already applied; schedule wall-clock invert
                    scheduleWallClockInvert(refs, !on, bars, bpb, result);
                }
            } else {
                scheduleWallClockInvert(refs, !on, bars, bpb, result);
            }
        }

        result.addProperty("queued", muteQueueSize());
        return result;
    }

    private void scheduleWallClockInvert(
            final String[] refs,
            final boolean invertOn,
            final int bars,
            final double bpb,
            final JsonObject result) {
        final double tempo = Math.max(20.0, transport.getTempo());
        final long ms = Math.max(1L, Math.round(bars * bpb * (60_000.0 / tempo)));
        final String[] refsCopy = refs.clone();
        host.scheduleTask(() -> applyMuteRefs(refsCopy, invertOn), ms);
        result.addProperty("invertMs", ms);
        result.addProperty("invert", invertOn);
        result.addProperty("invertMode", "wallclock");
    }

    private void enqueueMute(final String[] refs, final boolean on, final double atBeat) {
        synchronized (muteQueue) {
            muteQueue.add(new ScheduledMute(refs.clone(), on, atBeat));
        }
    }

    private int muteQueueSize() {
        synchronized (muteQueue) {
            return muteQueue.size();
        }
    }

    private void processMuteQueue(final double pos) {
        synchronized (muteQueue) {
            final Iterator<ScheduledMute> it = muteQueue.iterator();
            while (it.hasNext()) {
                final ScheduledMute job = it.next();
                if (pos + 1e-4 >= job.atBeat) {
                    applyMuteRefs(job.refs, job.on);
                    it.remove();
                }
            }
        }
    }

    private void applyMuteRefs(final String[] refs, final boolean on) {
        for (final String ref : refs) {
            try {
                resolve(ref).mute().set(on);
            } catch (final IllegalArgumentException e) {
                host.errorln("Codewig Bridge scheduled mute: " + e.getMessage());
            }
        }
    }

    private static final class ScheduledMute {
        final String[] refs;
        final boolean on;
        final double atBeat;

        ScheduledMute(final String[] refs, final boolean on, final double atBeat) {
            this.refs = refs;
            this.on = on;
            this.atBeat = atBeat;
        }
    }

    /**
     * Solo several tracks at once: refs = names and/or indices.
     */
    public JsonObject soloMany(final String[] refs, final boolean on) {
        return setBoolMany(refs, on, false);
    }

    private JsonObject setBoolMany(final String[] refs, final boolean on, final boolean muteNotSolo) {
        if (refs == null || refs.length == 0) {
            throw new IllegalArgumentException("need at least one track ref (index or name)");
        }
        final JsonArray applied = new JsonArray();
        for (final String ref : refs) {
            final Track t = resolve(ref);
            if (muteNotSolo) {
                t.mute().set(on);
            } else {
                t.solo().set(on);
            }
            final JsonObject o = new JsonObject();
            o.addProperty("index", t.position().get());
            o.addProperty("name", t.name().get());
            o.addProperty(muteNotSolo ? "mute" : "solo", on);
            applied.add(o);
        }
        final JsonObject result = new JsonObject();
        result.add(muteNotSolo ? "muted" : "soloed", applied);
        result.addProperty("on", on);
        return result;
    }

    /** volume 0..1 on one track */
    public JsonObject setVolume(final String ref, final double v) {
        if (v < 0.0 || v > 1.0) {
            throw new IllegalArgumentException("volume must be 0..1");
        }
        final Track t = resolve(ref);
        t.volume().value().setImmediately(v);
        final JsonObject result = new JsonObject();
        result.addProperty("name", t.name().get());
        result.addProperty("index", t.position().get());
        result.addProperty("volume", v);
        return result;
    }

    /**
     * Move track {@code ref} relative to another track or absolute index.
     * Exactly one of before/after/to should be set (to = absolute target index).
     */
    public JsonObject move(final String ref, final String before, final String after, final Integer to) {
        final Track moving = resolve(ref);
        int modes = 0;
        if (before != null) {
            modes++;
        }
        if (after != null) {
            modes++;
        }
        if (to != null) {
            modes++;
        }
        if (modes != 1) {
            throw new IllegalArgumentException("move requires exactly one of: before, after, to");
        }

        if (to != null) {
            final int target = to;
            if (target < 0) {
                throw new IllegalArgumentException("to must be >= 0");
            }
            ensureScrolledToStart();
            // to == 0 → before first; else after track at to-1 if exists, else before track at to
            if (target == 0) {
                final Track first = findByAbsolutePosition(0);
                if (first == null) {
                    throw new IllegalArgumentException("no tracks to move relative to");
                }
                first.beforeTrackInsertionPoint().moveTracks(moving);
            } else {
                final Track prev = findByAbsolutePosition(target - 1);
                if (prev != null) {
                    prev.afterTrackInsertionPoint().moveTracks(moving);
                } else {
                    final Track next = findByAbsolutePosition(target);
                    if (next == null) {
                        throw new IllegalArgumentException("target index out of range: " + target);
                    }
                    next.beforeTrackInsertionPoint().moveTracks(moving);
                }
            }
        } else if (before != null) {
            final Track anchor = resolve(before);
            anchor.beforeTrackInsertionPoint().moveTracks(moving);
        } else {
            final Track anchor = resolve(after);
            anchor.afterTrackInsertionPoint().moveTracks(moving);
        }

        final JsonObject result = new JsonObject();
        result.addProperty("moved", moving.name().get());
        result.addProperty("index", moving.position().get());
        return result;
    }

    public Track resolve(final String ref) {
        if (ref == null || ref.isBlank()) {
            throw new IllegalArgumentException("track ref required (name or index)");
        }
        final String r = ref.trim();
        ensureScrolledToStart();

        // numeric index
        try {
            final int idx = Integer.parseInt(r);
            final Track t = findByAbsolutePosition(idx);
            if (t == null) {
                throw new IllegalArgumentException("no track at index " + idx);
            }
            return t;
        } catch (final NumberFormatException ignored) {
            // name match
        }

        for (int i = 0; i < BANK_SIZE; i++) {
            final Track t = trackBank.getItemAt(i);
            if (t.exists().get() && r.equalsIgnoreCase(t.name().get())) {
                return t;
            }
        }
        throw new IllegalArgumentException("track not found: " + r);
    }

    private Track findByAbsolutePosition(final int absolutePos) {
        ensureScrolledToStart();
        for (int i = 0; i < BANK_SIZE; i++) {
            final Track t = trackBank.getItemAt(i);
            if (t.exists().get() && t.position().get() == absolutePos) {
                return t;
            }
        }
        return null;
    }

    private int countExistingTracks() {
        ensureScrolledToStart();
        int count = 0;
        for (int i = 0; i < BANK_SIZE; i++) {
            if (trackBank.getItemAt(i).exists().get()) {
                count++;
            }
        }
        return count;
    }

    private void ensureScrolledToStart() {
        trackBank.scrollPosition().set(0);
    }

    private static JsonObject trackJson(final Track t, final int index) {
        final JsonObject o = new JsonObject();
        o.addProperty("index", index);
        o.addProperty("name", t.name().get());
        o.addProperty("type", t.trackType().get());
        o.addProperty("mute", t.mute().get());
        o.addProperty("solo", t.solo().get());
        try {
            o.addProperty("volume", t.volume().value().get());
        } catch (final Exception ignored) {
            // not yet available
        }
        return o;
    }
}
