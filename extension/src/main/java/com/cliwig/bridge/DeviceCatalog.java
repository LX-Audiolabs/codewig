package com.cliwig.bridge;

import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Locale;
import java.util.Map;
import java.util.UUID;

/**
 * Curated native Bitwig devices that {@code device.add} may insert.
 * <p>
 * Not a full Bitwig browser. Live setup is mostly manual (drum pads, extra FX);
 * codewig-live focuses on clips / scenes / mute after tracks exist.
 * <p>
 * Out of scope (user places manually): Sampler, Drum Machine, Grids, all v* drum
 * devices, VSTs, and any FX not listed here.
 * <p>
 * Case-insensitive lookup; spaces / hyphens / plus normalized. Raw UUID always allowed.
 */
public final class DeviceCatalog {
    private static final Map<String, UUID> BY_KEY;

    static {
        final Map<String, UUID> m = new LinkedHashMap<>();

        // Synths (WIGSCRIPT n / fluent synth tracks)
        put(m, "Polymer", "8f58138b-03aa-4e9d-83bd-a038c99a4ed5");
        put(m, "Polysynth", "a9ffacb5-33e9-4fc7-8621-b1af31e410ef");
        put(m, "Organ", "f2dcfe9a-7b66-4c84-984a-b25685a1c21a");

        // Drum kit shell only — pads (v0Kick, v9Hat, …) are manual inside the layer
        put(m, "Instrument Layer", "5024be2e-65d6-4d40-bbfe-8b2ea993c445");

        // Small FX set for fluent/chain setup
        put(m, "Filter", "4ccfc70e-59bd-4e97-a8a7-d8cdce88bf42");
        put(m, "Reverb", "5a1cb339-1c4a-4cc7-9cae-bd7a2058153d");
        put(m, "Delay+", "f2baa2a8-36c5-4a79-b1d9-a4e461c45ee9");
        put(m, "Chorus+", "1b8f2226-c432-4a0a-9830-69bc76d1a276");
        put(m, "Saturator", "93d11348-86ae-4ead-9fe7-84ac03b9369c");

        BY_KEY = Collections.unmodifiableMap(m);
    }

    private DeviceCatalog() {
    }

    private static void put(final Map<String, UUID> m, final String name, final String uuid) {
        m.put(key(name), UUID.fromString(uuid));
    }

    private static String key(final String name) {
        return name.toLowerCase(Locale.ROOT).replace(" ", "").replace("-", "").replace("+", "plus");
    }

    public static UUID resolve(final String name) {
        if (name == null || name.isBlank()) {
            return null;
        }
        final String trimmed = name.trim();
        try {
            return UUID.fromString(trimmed);
        } catch (final IllegalArgumentException ignored) {
            // fall through
        }
        // Aliases used in WIGSCRIPT / older docs
        final String k = key(trimmed);
        if ("delay2".equals(k) || "delay1".equals(k) || "dly2".equals(k) || "dly1".equals(k)) {
            return BY_KEY.get(key("Delay+"));
        }
        if ("chorus".equals(k) || "chor".equals(k)) {
            return BY_KEY.get(key("Chorus+"));
        }
        if ("dist".equals(k) || "distortion".equals(k)) {
            return BY_KEY.get(key("Saturator"));
        }
        if ("layer".equals(k) || "instrumentlayer".equals(k)) {
            return BY_KEY.get(key("Instrument Layer"));
        }
        return BY_KEY.get(k);
    }

    public static Map<String, UUID> all() {
        return BY_KEY;
    }
}
