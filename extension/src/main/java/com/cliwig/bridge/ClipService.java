package com.cliwig.bridge;

import com.bitwig.extension.controller.api.Clip;
import com.bitwig.extension.controller.api.ClipLauncherSlot;
import com.bitwig.extension.controller.api.ClipLauncherSlotBank;
import com.bitwig.extension.controller.api.SettableStringValue;
import com.bitwig.extension.controller.api.StringValue;
import com.bitwig.extension.controller.api.Track;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;

import java.util.List;

/**
 * Clip Launcher slots — live/performance: empty clips, launch, list, notes.
 * Note editing goes through the launcher cursor clip (follows the selected slot).
 * Note-name parsing / scales / pattern sugar lives in the CLI, not here.
 */
public final class ClipService {
    private final TrackService tracks;
    private final Clip cursorClip;

    /** One note to write: step (16th grid), key (MIDI 0..127), velocity 1..127, duration in steps. */
    public record NoteSpec(int step, int key, int vel, double dur) {
    }

    public ClipService(final TrackService tracks, final Clip cursorClip) {
        this.tracks = tracks;
        this.cursorClip = cursorClip;

        // Mark clip launcher interests on bank tracks
        for (int ti = 0; ti < TrackService.BANK_SIZE; ti++) {
            final Track t = tracks.getTrackBank().getItemAt(ti);
            final ClipLauncherSlotBank slots = t.clipLauncherSlotBank();
            for (int s = 0; s < TrackService.SCENE_SLOTS; s++) {
                final ClipLauncherSlot slot = slots.getItemAt(s);
                slot.exists().markInterested();
                slot.hasContent().markInterested();
                slot.name().markInterested();
                slot.isPlaying().markInterested();
                slot.isPlaybackQueued().markInterested();
            }
        }
    }

    /**
     * Create empty clip on track.
     *
     * @param trackRef track name or index
     * @param slot     slot index, or -1 for first empty
     * @param beats    length in beats (default 4)
     * @param name     optional clip name
     */
    public JsonObject createEmpty(final String trackRef, final int slot, final int beats, final String name) {
        if (beats < 1 || beats > 512) {
            throw new IllegalArgumentException("beats must be 1..512");
        }
        final Track t = tracks.resolve(trackRef);
        final ClipLauncherSlotBank bank = t.clipLauncherSlotBank();

        int target = slot;
        if (target < 0) {
            target = firstEmptySlot(bank);
            if (target < 0) {
                throw new IllegalArgumentException("no empty clip slot on track (0.." + (TrackService.SCENE_SLOTS - 1) + ")");
            }
        }
        if (target >= TrackService.SCENE_SLOTS) {
            throw new IllegalArgumentException("slot out of range 0.." + (TrackService.SCENE_SLOTS - 1));
        }

        bank.createEmptyClip(target, beats);
        final ClipLauncherSlot created = bank.getItemAt(target);
        boolean named = false;
        if (name != null && !name.isBlank()) {
            // Slot.name() is typed as StringValue; runtime may be settable
            final StringValue nv = created.name();
            if (nv instanceof SettableStringValue) {
                ((SettableStringValue) nv).set(name.trim());
                named = true;
            }
        }

        final JsonObject result = new JsonObject();
        result.addProperty("track", t.name().get());
        result.addProperty("trackIndex", t.position().get());
        result.addProperty("slot", target);
        result.addProperty("beats", beats);
        if (name != null && !name.isBlank()) {
            result.addProperty("name", name.trim());
            result.addProperty("nameApplied", named);
        }
        return result;
    }

    public JsonObject list(final String trackRef) {
        final Track t = tracks.resolve(trackRef);
        final ClipLauncherSlotBank bank = t.clipLauncherSlotBank();
        final JsonArray clips = new JsonArray();
        for (int s = 0; s < TrackService.SCENE_SLOTS; s++) {
            final ClipLauncherSlot slot = bank.getItemAt(s);
            final JsonObject o = new JsonObject();
            o.addProperty("slot", s);
            o.addProperty("hasContent", slot.hasContent().get());
            o.addProperty("name", slot.name().get());
            o.addProperty("playing", slot.isPlaying().get());
            o.addProperty("queued", slot.isPlaybackQueued().get());
            clips.add(o);
        }
        final JsonObject result = new JsonObject();
        result.addProperty("track", t.name().get());
        result.addProperty("trackIndex", t.position().get());
        result.add("clips", clips);
        return result;
    }

    public JsonObject launch(final String trackRef, final int slot) {
        final Track t = tracks.resolve(trackRef);
        if (slot < 0 || slot >= TrackService.SCENE_SLOTS) {
            throw new IllegalArgumentException("slot out of range 0.." + (TrackService.SCENE_SLOTS - 1));
        }
        final ClipLauncherSlot s = t.clipLauncherSlotBank().getItemAt(slot);
        if (!s.hasContent().get()) {
            throw new IllegalArgumentException("slot " + slot + " is empty — create a clip first");
        }
        s.launch();
        final JsonObject result = new JsonObject();
        result.addProperty("track", t.name().get());
        result.addProperty("slot", slot);
        result.addProperty("name", s.name().get());
        result.addProperty("launched", true);
        return result;
    }

    public JsonObject stopTrack(final String trackRef) {
        final Track t = tracks.resolve(trackRef);
        t.stop();
        final JsonObject result = new JsonObject();
        result.addProperty("track", t.name().get());
        result.addProperty("stopped", true);
        return result;
    }

    /**
     * Write notes into a launcher clip via the cursor clip.
     * Slot must have content; each note is validated before writing.
     * Does not clear existing steps — use {@link #replaceNotes} for full patterns.
     */
    public JsonObject setNotes(final String trackRef, final int slot, final List<NoteSpec> notes) {
        if (notes.isEmpty()) {
            throw new IllegalArgumentException("notes array empty");
        }
        final ClipLauncherSlot s = resolveSlotWithContent(trackRef, slot);
        s.select();
        writeSteps(notes);
        final JsonObject result = new JsonObject();
        result.addProperty("track", trackRef);
        result.addProperty("slot", slot);
        result.addProperty("written", notes.size());
        return result;
    }

    /**
     * Clear all steps then write notes in one round-trip (one slot select).
     * Empty notes = clear only. Primary path for live pattern rewrite.
     */
    public JsonObject replaceNotes(final String trackRef, final int slot, final List<NoteSpec> notes) {
        final ClipLauncherSlot s = resolveSlotWithContent(trackRef, slot);
        s.select();
        cursorClip.clearSteps();
        if (!notes.isEmpty()) {
            writeSteps(notes);
        }
        final JsonObject result = new JsonObject();
        result.addProperty("track", trackRef);
        result.addProperty("slot", slot);
        result.addProperty("cleared", "all");
        result.addProperty("written", notes.size());
        return result;
    }

    /**
     * Clear notes: whole clip, or one cell when step+key are given.
     */
    public JsonObject clearNotes(final String trackRef, final int slot, final Integer step, final Integer key) {
        final ClipLauncherSlot s = resolveSlotWithContent(trackRef, slot);
        s.select();
        final JsonObject result = new JsonObject();
        result.addProperty("track", trackRef);
        result.addProperty("slot", slot);
        if (step != null && key != null) {
            cursorClip.scrollToKey(key);
            cursorClip.scrollToStep(step);
            cursorClip.clearStep(step, key);
            result.addProperty("cleared", 1);
        } else {
            cursorClip.clearSteps();
            result.addProperty("cleared", "all");
        }
        return result;
    }

    private void writeSteps(final List<NoteSpec> notes) {
        // Align grid: 1 step = 1/16 note (0.25 beats). NoteSpec.dur is length in steps.
        cursorClip.setStepSize(0.25);
        for (final NoteSpec n : notes) {
            if (n.step() < 0) {
                throw new IllegalArgumentException("step must be >= 0, got " + n.step());
            }
            if (n.key() < 0 || n.key() > 127) {
                throw new IllegalArgumentException("key must be 0..127, got " + n.key());
            }
            if (n.vel() < 1 || n.vel() > 127) {
                // rest (vel 0) — skip write
                continue;
            }
            if (n.dur() <= 0) {
                throw new IllegalArgumentException("dur must be > 0, got " + n.dur());
            }
            // Writes outside the cursor viewport are dropped silently — scroll first
            cursorClip.scrollToKey(n.key());
            cursorClip.scrollToStep(n.step());
            cursorClip.setStep(n.step(), n.key(), n.vel(), n.dur());
        }
    }

    private ClipLauncherSlot resolveSlotWithContent(final String trackRef, final int slot) {
        final Track t = tracks.resolve(trackRef);
        if (slot < 0 || slot >= TrackService.SCENE_SLOTS) {
            throw new IllegalArgumentException("slot out of range 0.." + (TrackService.SCENE_SLOTS - 1));
        }
        final ClipLauncherSlotBank bank = t.clipLauncherSlotBank();
        ClipLauncherSlot s = bank.getItemAt(slot);
        if (!s.hasContent().get()) {
            // Auto-create empty clip at this scene row (Bitwig cell track×scene)
            bank.createEmptyClip(slot, 4);
            s = bank.getItemAt(slot);
        }
        if (!s.hasContent().get()) {
            throw new IllegalArgumentException(
                    "slot " + slot + " is empty — create a clip first "
                            + "(e.g. s(" + slot + ").t(" + trackRef + ").c(new) or clip.new)");
        }
        return s;
    }

    private static int firstEmptySlot(final ClipLauncherSlotBank bank) {
        for (int s = 0; s < TrackService.SCENE_SLOTS; s++) {
            if (!bank.getItemAt(s).hasContent().get()) {
                return s;
            }
        }
        return -1;
    }
}
