package com.cliwig.protocol;

import com.cliwig.bridge.ClipService;
import com.cliwig.bridge.DeviceService;
import com.cliwig.bridge.ParamService;
import com.cliwig.bridge.SceneService;
import com.cliwig.bridge.TrackService;
import com.cliwig.bridge.TransportService;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

/**
 * Maps wire commands to bridge services.
 */
public final class CommandRouter {
    private final TransportService transport;
    private final TrackService tracks;
    private final DeviceService devices;
    private final ParamService params;
    private final ClipService clips;
    private final SceneService scenes;
    private final int port;

    public CommandRouter(
            final TransportService transport,
            final TrackService tracks,
            final DeviceService devices,
            final ParamService params,
            final ClipService clips,
            final SceneService scenes,
            final int port) {
        this.transport = transport;
        this.tracks = tracks;
        this.devices = devices;
        this.params = params;
        this.clips = clips;
        this.scenes = scenes;
        this.port = port;
    }

    public String handle(final JsonObject req) {
        final JsonElement id = req.has("id") ? req.get("id") : null;

        if (!req.has("c") || !req.get("c").isJsonPrimitive()) {
            return Messages.error(id, "BAD_REQUEST", "missing string field 'c' (command)");
        }

        final String cmd = req.get("c").getAsString();

        try {
            switch (cmd) {
                case "ping":
                    return Messages.ok(id);

                case "status":
                    return Messages.ok(id, transport.status(port));

                case "play":
                    transport.play();
                    return Messages.ok(id);

                case "stop":
                    transport.stop();
                    return Messages.ok(id);

                case "set":
                    return handleSet(id, req);

                case "track.new":
                    return Messages.ok(id, tracks.create(
                            str(req, "type", "instrument"),
                            intOr(req, "at", -1),
                            str(req, "name", null)));

                case "track.list":
                    return Messages.ok(id, tracks.list());

                case "track.select":
                    return Messages.ok(id, tracks.select(requireStr(req, "ref")));

                case "track.rename":
                    return Messages.ok(id, tracks.rename(requireStr(req, "ref"), requireStr(req, "name")));

                case "track.delete":
                    return Messages.ok(id, tracks.delete(requireStr(req, "ref")));

                case "track.move":
                    return Messages.ok(id, tracks.move(
                            requireStr(req, "ref"),
                            str(req, "before", null),
                            str(req, "after", null),
                            req.has("to") && !req.get("to").isJsonNull() ? req.get("to").getAsInt() : null));

                case "track.mute":
                    return Messages.ok(id, tracks.muteMany(
                            requireRefs(req),
                            boolOr(req, "on", true),
                            optPositiveInt(req, "bars"),
                            str(req, "q", null)));

                case "track.solo":
                    return Messages.ok(id, tracks.soloMany(requireRefs(req), boolOr(req, "on", true)));

                case "track.volume":
                    return Messages.ok(id, tracks.setVolume(requireStr(req, "ref"), requireDouble(req, "v")));

                case "device.add":
                    return Messages.ok(id, devices.add(requireStr(req, "name")));

                case "device.list":
                    return Messages.ok(id, devices.list());

                case "device.select":
                    return Messages.ok(id, devices.select(requireInt(req, "index")));

                case "device.delete":
                    return Messages.ok(id, devices.delete(requireInt(req, "index")));

                case "param.list":
                    return Messages.ok(id, params.list());

                case "param.set":
                    return Messages.ok(id, params.set(req));

                case "clip.new":
                    return Messages.ok(id, clips.createEmpty(
                            requireStr(req, "track"),
                            intOr(req, "slot", -1),
                            intOr(req, "beats", 4),
                            str(req, "name", null)));

                case "clip.list":
                    return Messages.ok(id, clips.list(requireStr(req, "track")));

                case "clip.launch":
                    return Messages.ok(id, clips.launch(requireStr(req, "track"), requireInt(req, "slot")));

                case "clip.stop":
                    return Messages.ok(id, clips.stopTrack(requireStr(req, "track")));

                case "clip.set-notes":
                    return Messages.ok(id, clips.setNotes(
                            requireStr(req, "track"),
                            requireInt(req, "slot"),
                            parseNotes(req)));

                case "clip.replace-notes":
                    // clear + write in one round-trip (live pattern rewrite)
                    return Messages.ok(id, clips.replaceNotes(
                            requireStr(req, "track"),
                            requireInt(req, "slot"),
                            parseNotesAllowEmpty(req)));

                case "clip.clear-notes":
                    return Messages.ok(id, clips.clearNotes(
                            requireStr(req, "track"),
                            requireInt(req, "slot"),
                            optInt(req, "step"),
                            optInt(req, "key")));

                case "scene.list":
                    return Messages.ok(id, scenes.list());

                case "scene.launch":
                    return Messages.ok(id, scenes.launch(requireSceneRef(req)));

                case "scene.stop":
                    return Messages.ok(id, scenes.stop(requireSceneRef(req)));

                default:
                    return Messages.error(id, "UNKNOWN_COMMAND", "unknown command: " + cmd);
            }
        } catch (final IllegalArgumentException e) {
            return Messages.error(id, "BAD_REQUEST", e.getMessage());
        } catch (final Exception e) {
            return Messages.error(id, "INTERNAL", e.getMessage() != null ? e.getMessage() : e.getClass().getSimpleName());
        }
    }

    private String handleSet(final JsonElement id, final JsonObject req) {
        if (!req.has("k") || !req.get("k").isJsonPrimitive()) {
            return Messages.error(id, "BAD_REQUEST", "set requires string field 'k'");
        }
        final String key = req.get("k").getAsString();

        if ("tempo".equals(key)) {
            if (!req.has("v")) {
                return Messages.error(id, "BAD_REQUEST", "set tempo requires 'v' (bpm)");
            }
            transport.setTempo(req.get("v").getAsDouble());
            return Messages.ok(id);
        }
        return Messages.error(id, "UNKNOWN_KEY", "unknown set key: " + key);
    }

    /** refs: JSON array, or comma-separated string in "refs" */
    private static String[] requireRefs(final JsonObject req) {
        if (req.has("refs") && req.get("refs").isJsonArray()) {
            final JsonArray arr = req.getAsJsonArray("refs");
            if (arr.size() == 0) {
                throw new IllegalArgumentException("refs array empty");
            }
            final String[] out = new String[arr.size()];
            for (int i = 0; i < arr.size(); i++) {
                out[i] = arr.get(i).getAsString();
            }
            return out;
        }
        if (req.has("refs") && req.get("refs").isJsonPrimitive()) {
            final String raw = req.get("refs").getAsString().trim();
            if (raw.isEmpty()) {
                throw new IllegalArgumentException("refs empty");
            }
            return raw.split("\\s*,\\s*");
        }
        throw new IllegalArgumentException("missing refs (array of track names/indices)");
    }

    private static String str(final JsonObject req, final String key, final String def) {
        if (!req.has(key) || req.get(key).isJsonNull()) {
            return def;
        }
        return req.get(key).getAsString();
    }

    private static String requireStr(final JsonObject req, final String key) {
        final String v = str(req, key, null);
        if (v == null || v.isBlank()) {
            throw new IllegalArgumentException("missing '" + key + "'");
        }
        return v;
    }

    /** `ref` as string or number (index primary, name secondary). */
    private static String requireSceneRef(final JsonObject req) {
        if (!req.has("ref") || req.get("ref").isJsonNull()) {
            throw new IllegalArgumentException("missing 'ref' (scene index or name)");
        }
        final JsonElement el = req.get("ref");
        if (el.isJsonPrimitive() && el.getAsJsonPrimitive().isNumber()) {
            return Integer.toString(el.getAsInt());
        }
        final String s = el.getAsString();
        if (s == null || s.isBlank()) {
            throw new IllegalArgumentException("empty scene ref");
        }
        return s.trim();
    }

    private static int intOr(final JsonObject req, final String key, final int def) {
        if (!req.has(key) || req.get(key).isJsonNull()) {
            return def;
        }
        return req.get(key).getAsInt();
    }

    private static int requireInt(final JsonObject req, final String key) {
        if (!req.has(key) || req.get(key).isJsonNull()) {
            throw new IllegalArgumentException("missing '" + key + "'");
        }
        return req.get(key).getAsInt();
    }

    private static Integer optInt(final JsonObject req, final String key) {
        if (!req.has(key) || req.get(key).isJsonNull()) {
            return null;
        }
        return req.get(key).getAsInt();
    }

    /** Optional positive int (>=1); null if missing. */
    private static Integer optPositiveInt(final JsonObject req, final String key) {
        final Integer v = optInt(req, key);
        if (v == null) {
            return null;
        }
        if (v < 1) {
            throw new IllegalArgumentException(key + " must be >= 1");
        }
        return v;
    }

    /** notes: array of {step:int, key:int (MIDI), vel:int 1..127 = 100, dur:double steps = 1.0} */
    private static java.util.List<com.cliwig.bridge.ClipService.NoteSpec> parseNotes(final JsonObject req) {
        final java.util.List<com.cliwig.bridge.ClipService.NoteSpec> out = parseNotesAllowEmpty(req);
        if (out.isEmpty()) {
            throw new IllegalArgumentException("notes array empty");
        }
        return out;
    }

    /** Like parseNotes but allows empty array (replace-notes = clear only). */
    private static java.util.List<com.cliwig.bridge.ClipService.NoteSpec> parseNotesAllowEmpty(final JsonObject req) {
        if (!req.has("notes") || !req.get("notes").isJsonArray()) {
            throw new IllegalArgumentException("missing 'notes' (array of {step,key,vel,dur})");
        }
        final JsonArray arr = req.getAsJsonArray("notes");
        final java.util.List<com.cliwig.bridge.ClipService.NoteSpec> out = new java.util.ArrayList<>(arr.size());
        for (final JsonElement el : arr) {
            if (!el.isJsonObject()) {
                throw new IllegalArgumentException("note must be an object {step,key,vel,dur}");
            }
            final JsonObject n = el.getAsJsonObject();
            out.add(new com.cliwig.bridge.ClipService.NoteSpec(
                    requireInt(n, "step"),
                    requireInt(n, "key"),
                    intOr(n, "vel", 100),
                    n.has("dur") && !n.get("dur").isJsonNull() ? n.get("dur").getAsDouble() : 1.0));
        }
        return out;
    }

    private static double requireDouble(final JsonObject req, final String key) {
        if (!req.has(key) || req.get(key).isJsonNull()) {
            throw new IllegalArgumentException("missing '" + key + "'");
        }
        return req.get(key).getAsDouble();
    }

    private static boolean boolOr(final JsonObject req, final String key, final boolean def) {
        if (!req.has(key) || req.get(key).isJsonNull()) {
            return def;
        }
        final JsonElement el = req.get(key);
        if (el.isJsonPrimitive() && el.getAsJsonPrimitive().isBoolean()) {
            return el.getAsBoolean();
        }
        final String s = el.getAsString();
        if ("on".equalsIgnoreCase(s) || "true".equalsIgnoreCase(s) || "1".equals(s)) {
            return true;
        }
        if ("off".equalsIgnoreCase(s) || "false".equalsIgnoreCase(s) || "0".equals(s)) {
            return false;
        }
        return def;
    }
}
