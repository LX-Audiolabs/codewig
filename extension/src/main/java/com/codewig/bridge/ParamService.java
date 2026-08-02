package com.codewig.bridge;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

import com.bitwig.extension.controller.api.CursorDevice;
import com.bitwig.extension.controller.api.CursorTrack;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

/**
 * Direct Parameters on the cursor device.
 * Stable ids (not remote-control slots) — required for fluent param chains.
 *
 * Wire:
 *   param.list
 *   param.set  { id|name, v }  or  { sets: [ {id|name, v}, ... ] }  // batch for fluent expand
 */
public final class ParamService {
    /** Resolution for setDirectParameterValueNormalized (value mapped 0..1 → 0..RES-1). */
    private static final int RESOLUTION = 128;

    private final CursorTrack cursorTrack;
    private final CursorDevice cursorDevice;

    /** id → name */
    private final Map<String, String> namesById = new ConcurrentHashMap<>();
    /** id → normalized 0..1 */
    private final Map<String, Double> valuesById = new ConcurrentHashMap<>();
    private volatile String[] ids = new String[0];

    public ParamService(final CursorTrack cursorTrack, final CursorDevice cursorDevice) {
        this.cursorTrack = cursorTrack;
        this.cursorDevice = cursorDevice;

        cursorDevice.addDirectParameterIdObserver(newIds -> {
            ids = newIds != null ? newIds.clone() : new String[0];
            // drop stale
            namesById.keySet().retainAll(java.util.Arrays.asList(ids));
            valuesById.keySet().retainAll(java.util.Arrays.asList(ids));
        });

        cursorDevice.addDirectParameterNameObserver(64, (id, name) -> {
            if (id != null) {
                namesById.put(id, name != null ? name : "");
            }
        });

        cursorDevice.addDirectParameterNormalizedValueObserver((id, value) -> {
            if (id != null) {
                if (Double.isNaN(value)) {
                    valuesById.remove(id);
                } else {
                    valuesById.put(id, (double) value);
                }
            }
        });
    }

    public JsonObject list() {
        requireDevice();
        final JsonArray params = new JsonArray();
        for (final String id : ids) {
            final JsonObject o = new JsonObject();
            o.addProperty("id", id);
            o.addProperty("name", namesById.getOrDefault(id, ""));
            if (valuesById.containsKey(id)) {
                o.addProperty("value", valuesById.get(id));
            }
            params.add(o);
        }
        final JsonObject result = new JsonObject();
        result.add("params", params);
        result.addProperty("count", params.size());
        result.addProperty("device", cursorDevice.name().get());
        result.addProperty("track", cursorTrack.name().get());
        return result;
    }

    /**
     * Single set: name or id + v in 0..1.
     * Batch: sets array of {name|id, v}.
     */
    public JsonObject set(final JsonObject req) {
        requireDevice();

        final List<JsonObject> applied = new ArrayList<>();

        if (req.has("sets") && req.get("sets").isJsonArray()) {
            for (final JsonElement el : req.getAsJsonArray("sets")) {
                if (!el.isJsonObject()) {
                    throw new IllegalArgumentException("sets entries must be objects");
                }
                applied.add(applyOne(el.getAsJsonObject()));
            }
        } else {
            applied.add(applyOne(req));
        }

        final JsonObject result = new JsonObject();
        final JsonArray arr = new JsonArray();
        for (final JsonObject a : applied) {
            arr.add(a);
        }
        result.add("set", arr);
        result.addProperty("device", cursorDevice.name().get());
        return result;
    }

    private JsonObject applyOne(final JsonObject spec) {
        final String id = resolveId(spec);
        if (!spec.has("v")) {
            throw new IllegalArgumentException("param set requires 'v' (0..1 normalized)");
        }
        final double v = spec.get("v").getAsDouble();
        if (v < 0.0 || v > 1.0) {
            throw new IllegalArgumentException("v must be in 0..1, got " + v);
        }
        // API: value in [0 .. resolution-1]
        final int discrete = (int) Math.round(v * (RESOLUTION - 1));
        cursorDevice.setDirectParameterValueNormalized(id, discrete, RESOLUTION);

        final JsonObject done = new JsonObject();
        done.addProperty("id", id);
        done.addProperty("name", namesById.getOrDefault(id, ""));
        done.addProperty("v", v);
        return done;
    }

    private String resolveId(final JsonObject spec) {
        if (spec.has("id") && !spec.get("id").isJsonNull()) {
            final String id = spec.get("id").getAsString();
            if (id.isBlank()) {
                throw new IllegalArgumentException("empty id");
            }
            return id;
        }
        if (spec.has("name") && !spec.get("name").isJsonNull()) {
            final String want = spec.get("name").getAsString().trim();
            final String found = findIdByName(want);
            if (found == null) {
                throw new IllegalArgumentException("param not found by name: " + want + " (try param list)");
            }
            return found;
        }
        throw new IllegalArgumentException("param set requires 'id' or 'name'");
    }

    private String findIdByName(final String want) {
        final String key = want.toLowerCase(Locale.ROOT);
        // exact
        for (final Map.Entry<String, String> e : namesById.entrySet()) {
            if (e.getValue() != null && e.getValue().equalsIgnoreCase(want)) {
                return e.getKey();
            }
        }
        // contains (cutoff matches "Filter Cutoff" etc.)
        String best = null;
        int bestLen = Integer.MAX_VALUE;
        for (final Map.Entry<String, String> e : namesById.entrySet()) {
            final String n = e.getValue();
            if (n == null) {
                continue;
            }
            final String nl = n.toLowerCase(Locale.ROOT);
            if (nl.contains(key) && n.length() < bestLen) {
                best = e.getKey();
                bestLen = n.length();
            }
        }
        return best;
    }

    private void requireDevice() {
        if (!cursorTrack.exists().get()) {
            throw new IllegalArgumentException("no track selected");
        }
        if (!cursorDevice.exists().get()) {
            throw new IllegalArgumentException("no device selected (add/select a device first)");
        }
    }
}
