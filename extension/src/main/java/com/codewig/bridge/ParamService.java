package com.codewig.bridge;

import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

import com.bitwig.extension.controller.api.CursorDevice;
import com.bitwig.extension.controller.api.CursorRemoteControlsPage;
import com.bitwig.extension.controller.api.CursorTrack;
import com.bitwig.extension.controller.api.Parameter;
import com.bitwig.extension.controller.api.RemoteControl;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

/**
 * Device parameters on the cursor device.
 * <ul>
 *   <li><b>direct</b> — full plugin param dump ({@code setDirectParameterValueNormalized})</li>
 *   <li><b>remote</b> — Bitwig Remote Controls pages (8 knobs × N pages) — preferred for WIGSCRIPT YAML</li>
 * </ul>
 *
 * Wire:
 *   param.list           → direct (default)
 *   param.list {source:"remote"} → remote controls only
 *   param.set  { id|name, v }  or  { sets: [ {id|name, v}, ... ] }
 */
public final class ParamService {
    /** Resolution for setDirectParameterValueNormalized (value mapped 0..1 → 0..RES-1). */
    private static final int RESOLUTION = 128;
    private static final int REMOTE_SLOTS = 8;

    private final CursorTrack cursorTrack;
    private final CursorDevice cursorDevice;
    private final CursorRemoteControlsPage remoteControls;

    /** id → name (direct params) */
    private final Map<String, String> namesById = new ConcurrentHashMap<>();
    /** id → normalized 0..1 */
    private final Map<String, Double> valuesById = new ConcurrentHashMap<>();
    private volatile String[] ids = new String[0];

    public ParamService(final CursorTrack cursorTrack, final CursorDevice cursorDevice) {
        this.cursorTrack = cursorTrack;
        this.cursorDevice = cursorDevice;

        cursorDevice.exists().markInterested();
        cursorDevice.name().markInterested();

        // ── Direct params ──────────────────────────────────────────
        cursorDevice.addDirectParameterIdObserver(newIds -> {
            ids = newIds != null ? newIds.clone() : new String[0];
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

        // ── Remote Controls (8 slots × pages) ──────────────────────
        this.remoteControls = cursorDevice.createCursorRemoteControlsPage(REMOTE_SLOTS);
        remoteControls.pageCount().markInterested();
        remoteControls.selectedPageIndex().markInterested();
        for (int i = 0; i < REMOTE_SLOTS; i++) {
            final RemoteControl p = remoteControls.getParameter(i);
            p.exists().markInterested();
            p.name().markInterested();
            p.value().markInterested();
        }
    }

    /** Default = direct params (full dump). */
    public JsonObject list() {
        return listDirect();
    }

    /**
     * @param source {@code direct} | {@code remote} | {@code all}
     */
    public JsonObject list(final String source) {
        if (source == null || source.isBlank() || "direct".equalsIgnoreCase(source)) {
            return listDirect();
        }
        if ("remote".equalsIgnoreCase(source)) {
            return listRemote();
        }
        if ("all".equalsIgnoreCase(source)) {
            final JsonObject result = new JsonObject();
            result.add("direct", listDirect());
            result.add("remote", listRemote());
            result.addProperty("device", cursorDevice.name().get());
            result.addProperty("track", cursorTrack.name().get());
            return result;
        }
        throw new IllegalArgumentException("param.list source must be direct|remote|all, got: " + source);
    }

    public JsonObject listDirect() {
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
        result.addProperty("source", "direct");
        result.addProperty("device", cursorDevice.name().get());
        result.addProperty("track", cursorTrack.name().get());
        return result;
    }

    /**
     * Walk all Remote Control pages and list the 8 slots per page.
     * Names map to plugin params (usable with param.set by name when direct ids exist).
     */
    public JsonObject listRemote() {
        requireDevice();
        final int pageCount = Math.max(1, remoteControls.pageCount().get());
        final int savedPage = remoteControls.selectedPageIndex().get();

        final JsonArray pages = new JsonArray();
        final JsonArray flat = new JsonArray();

        for (int page = 0; page < pageCount; page++) {
            remoteControls.selectedPageIndex().set(page);
            // Same-tick read often works after markInterested; page switch is best-effort.
            final JsonObject pageObj = new JsonObject();
            pageObj.addProperty("index", page);
            final JsonArray pageParams = new JsonArray();
            for (int slot = 0; slot < REMOTE_SLOTS; slot++) {
                final RemoteControl p = remoteControls.getParameter(slot);
                final boolean exists = p.exists().get();
                final String name = p.name().get() != null ? p.name().get() : "";
                // Always report slot so we can debug empty mappings; skip fully empty
                if (!exists && name.isBlank()) {
                    continue;
                }
                final JsonObject o = new JsonObject();
                o.addProperty("page", page);
                o.addProperty("slot", slot);
                o.addProperty("exists", exists);
                o.addProperty("name", name.isBlank() ? ("slot" + slot) : name);
                o.addProperty("value", p.value().get());
                final String directId = name.isBlank() ? null : findIdByName(name);
                if (directId != null) {
                    o.addProperty("id", directId);
                }
                pageParams.add(o);
                flat.add(o);
            }
            pageObj.add("params", pageParams);
            pageObj.addProperty("count", pageParams.size());
            pages.add(pageObj);
        }

        // restore page
        if (savedPage >= 0 && savedPage < pageCount) {
            remoteControls.selectedPageIndex().set(savedPage);
        }

        final JsonObject result = new JsonObject();
        result.add("pages", pages);
        result.add("params", flat);
        result.addProperty("count", flat.size());
        result.addProperty("pageCount", pageCount);
        result.addProperty("source", "remote");
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
        for (final Map.Entry<String, String> e : namesById.entrySet()) {
            if (e.getValue() != null && e.getValue().equalsIgnoreCase(want)) {
                return e.getKey();
            }
        }
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
