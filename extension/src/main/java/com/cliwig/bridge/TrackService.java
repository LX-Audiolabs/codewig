package com.cliwig.bridge;

import com.bitwig.extension.controller.api.Application;
import com.bitwig.extension.controller.api.ControllerHost;
import com.bitwig.extension.controller.api.CursorTrack;
import com.bitwig.extension.controller.api.Track;
import com.bitwig.extension.controller.api.TrackBank;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;

/**
 * Tracks: create (with insert index), list, select, delete, move, multi mute/solo.
 * Clip-launcher slots: {@link #SCENE_SLOTS} scenes per track (live/performance).
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

    private volatile int trackCount;

    public TrackService(final ControllerHost host) {
        this.host = host;
        this.application = host.createApplication();
        // numSends=0, numScenes=SCENE_SLOTS for clip launcher
        this.trackBank = host.createTrackBank(BANK_SIZE, 0, SCENE_SLOTS, true);
        this.cursorTrack = host.createCursorTrack("CLIWIG_CURSOR", "CLIwig Cursor", 0, SCENE_SLOTS, true);

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
            t.volume().markInterested();
            t.volume().value().markInterested();
        }
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
     * Fluent later: track.mute(1,3,6)
     */
    public JsonObject muteMany(final String[] refs, final boolean on) {
        return setBoolMany(refs, on, true);
    }

    /**
     * Solo several tracks at once: refs = names and/or indices.
     * Fluent later: track.solo(1,3,6)
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
