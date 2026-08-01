package com.cliwig.bridge;

import com.bitwig.extension.controller.api.ControllerHost;
import com.bitwig.extension.controller.api.Scene;
import com.bitwig.extension.controller.api.SceneBank;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;

/**
 * Clip-launcher scenes: list, resolve by index (primary) or name, launch/stop.
 * Names are secondary — same idea as clip @name → slot index.
 */
public final class SceneService {
    private final SceneBank sceneBank;
    private final int size;

    public SceneService(final ControllerHost host, final int size) {
        this.size = size;
        this.sceneBank = host.createSceneBank(size);
        for (int i = 0; i < size; i++) {
            final Scene s = sceneBank.getScene(i);
            s.exists().markInterested();
            s.name().markInterested();
            s.sceneIndex().markInterested();
            s.clipCount().markInterested();
        }
        sceneBank.scrollPosition().markInterested();
        sceneBank.itemCount().markInterested();
    }

    public JsonObject list() {
        sceneBank.scrollPosition().set(0);
        final JsonArray scenes = new JsonArray();
        for (int i = 0; i < size; i++) {
            final Scene s = sceneBank.getScene(i);
            if (!s.exists().get()) {
                continue;
            }
            final JsonObject o = new JsonObject();
            o.addProperty("index", i);
            o.addProperty("name", s.name().get());
            o.addProperty("clipCount", s.clipCount().get());
            scenes.add(o);
        }
        final JsonObject result = new JsonObject();
        result.add("scenes", scenes);
        result.addProperty("count", scenes.size());
        return result;
    }

    /**
     * Resolve ref: bare integer string → index; else case-insensitive name (first match).
     */
    public int resolve(final String ref) {
        if (ref == null || ref.isBlank()) {
            throw new IllegalArgumentException("scene ref empty");
        }
        final String t = ref.trim();
        try {
            final int idx = Integer.parseInt(t);
            if (idx < 0 || idx >= size) {
                throw new IllegalArgumentException(
                        "scene index out of range 0.." + (size - 1) + ", got " + idx);
            }
            return idx;
        } catch (final NumberFormatException ignored) {
            // name path
        }
        sceneBank.scrollPosition().set(0);
        final String want = t.toLowerCase();
        for (int i = 0; i < size; i++) {
            final Scene s = sceneBank.getScene(i);
            if (!s.exists().get()) {
                continue;
            }
            final String n = s.name().get();
            if (n != null && n.equalsIgnoreCase(want)) {
                return i;
            }
        }
        throw new IllegalArgumentException(
                "scene '" + t + "' not found (name or index 0.." + (size - 1) + ")");
    }

    public JsonObject launch(final String ref) {
        final int idx = resolve(ref);
        final Scene s = sceneBank.getScene(idx);
        s.launch();
        final JsonObject result = new JsonObject();
        result.addProperty("index", idx);
        result.addProperty("name", s.name().get());
        result.addProperty("launched", true);
        return result;
    }

    /** Stop clip launcher playback (all tracks). */
    public JsonObject stop(final String ref) {
        final int idx = resolve(ref); // validate ref even if stop is global
        sceneBank.stop();
        final Scene s = sceneBank.getScene(idx);
        final JsonObject result = new JsonObject();
        result.addProperty("index", idx);
        result.addProperty("name", s.name().get());
        result.addProperty("stopped", true);
        return result;
    }
}
