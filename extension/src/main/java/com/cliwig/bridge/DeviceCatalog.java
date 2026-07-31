package com.cliwig.bridge;

import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Locale;
import java.util.Map;
import java.util.UUID;

/**
 * Minimal native Bitwig device name → UUID map (from community device lists).
 * Case-insensitive lookup; spaces/hyphens/plus normalized.
 */
public final class DeviceCatalog {
    private static final Map<String, UUID> BY_KEY;

    static {
        final Map<String, UUID> m = new LinkedHashMap<>();
        // Instruments
        put(m, "Polymer", "8f58138b-03aa-4e9d-83bd-a038c99a4ed5");
        put(m, "Polysynth", "a9ffacb5-33e9-4fc7-8621-b1af31e410ef");
        put(m, "Sampler", "468bc14b-b2e7-45a1-9666-e83117fe404e");
        put(m, "Drum Machine", "8ea97e45-0255-40fd-bc7e-94419741e9d1");
        put(m, "FM-4", "7a0a94df-3aa4-4bb5-8e24-2511999871ad");
        put(m, "Phase-4", "252723bf-68a6-4ee6-81f8-95ba4d0fb467");
        put(m, "Organ", "f2dcfe9a-7b66-4c84-984a-b25685a1c21a");
        put(m, "Poly Grid", "a33bba66-8cd4-4f89-aee5-68bf67f70a54");
        put(m, "Instrument Layer", "5024be2e-65d6-4d40-bbfe-8b2ea993c445");
        put(m, "Instrument Selector", "9588fbcf-721a-438b-8555-97e4231f7d2c");
        // FX (common)
        put(m, "Dynamics", "22e785a2-a187-41e9-a0f2-66343694014c");
        put(m, "Compressor", "2b1b4787-8d74-4138-877b-9197209eef0f");
        put(m, "EQ+", "e4815188-ba6f-4d14-bcfc-2dcb8f778ccb");
        put(m, "EQ-5", "227e2e3c-75d5-46f3-960d-8fb5529fe29f");
        put(m, "EQ-2", "01af068e-1e49-4777-a6e6-7f1dc679227a");
        put(m, "Reverb", "5a1cb339-1c4a-4cc7-9cae-bd7a2058153d");
        put(m, "Delay+", "f2baa2a8-36c5-4a79-b1d9-a4e461c45ee9");
        put(m, "Chorus+", "1b8f2226-c432-4a0a-9830-69bc76d1a276");
        put(m, "Filter", "4ccfc70e-59bd-4e97-a8a7-d8cdce88bf42");
        put(m, "Saturator", "93d11348-86ae-4ead-9fe7-84ac03b9369c");
        put(m, "Tool", "e67b9c56-838d-4fba-8e3e-ae4e02cccbcb");
        put(m, "Amp", "41be8f3a-6d24-4442-9508-8548dbe62d47");
        put(m, "Gate", "556300ac-3a6e-4423-966a-5d5dde459a1b");
        put(m, "Peak Limiter", "8da7251e-2578-4bcc-b3c4-8f4ec2e115d0");
        put(m, "FX Grid", "d641f61b-d4db-4006-930e-cdd7aeb3e9d7");
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
        // raw UUID allowed
        try {
            return UUID.fromString(trimmed);
        } catch (final IllegalArgumentException ignored) {
            // fall through
        }
        return BY_KEY.get(key(trimmed));
    }

    public static Map<String, UUID> all() {
        return BY_KEY;
    }
}
