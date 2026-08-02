package com.codewig.bridge;

import java.io.File;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Locale;
import java.util.Map;
import java.util.UUID;

import com.bitwig.extension.controller.api.ControllerHost;

/**
 * Device name resolution for {@code device.add}.
 * <p>
 * <b>Insert model (open):</b> any Bitwig stock/library device the bridge can resolve —
 * known UUID map, known drum aliases, raw UUID, or a matching {@code .bwdevice} under
 * Bitwig {@code Library/devices}. Not a closed product allowlist.
 * <p>
 * <b>UI / params:</b> only devices with {@code devices/*.yaml} are listed and param-settable.
 * <p>
 * <b>Out of scope:</b> Sampler, Drum Machine (multi-pad / samples).
 * <p>
 * Case-insensitive lookup; spaces / hyphens / underscores / dots stripped,
 * plus → "plus" (mirrors Rust {@code device::norm}). Alias sets are the union
 * of both sides — keep in sync with Rust {@code device.rs}.
 */
public final class DeviceCatalog {
    private static final Map<String, UUID> BY_UUID;
    /** Normalized key → Bitwig library file name (without path), e.g. {@code v9 Kick.bwdevice}. */
    private static final Map<String, String> DRUM_FILES;

    static {
        final Map<String, UUID> m = new LinkedHashMap<>();

        // Synths
        putUuid(m, "Polymer", "8f58138b-03aa-4e9d-83bd-a038c99a4ed5");
        putUuid(m, "Polysynth", "a9ffacb5-33e9-4fc7-8621-b1af31e410ef");
        putUuid(m, "Organ", "f2dcfe9a-7b66-4c84-984a-b25685a1c21a");

        // Layer shell (multi-pad kits)
        putUuid(m, "Instrument Layer", "5024be2e-65d6-4d40-bbfe-8b2ea993c445");

        // FX
        putUuid(m, "Filter", "4ccfc70e-59bd-4e97-a8a7-d8cdce88bf42");
        putUuid(m, "Reverb", "5a1cb339-1c4a-4cc7-9cae-bd7a2058153d");
        putUuid(m, "Delay+", "f2baa2a8-36c5-4a79-b1d9-a4e461c45ee9");
        putUuid(m, "Chorus+", "1b8f2226-c432-4a0a-9830-69bc76d1a276");
        putUuid(m, "Saturator", "93d11348-86ae-4ead-9fe7-84ac03b9369c");

        BY_UUID = Collections.unmodifiableMap(m);

        final Map<String, String> d = new LinkedHashMap<>();
        // Exact Bitwig library filenames (Library/devices/*.bwdevice)
        putDrum(d, "v0 Cymbal", "v0 Cymbal.bwdevice");
        putDrum(d, "v0 Hat", "v0 Hat.bwdevice");
        putDrum(d, "v0 Kick", "v0 Kick.bwdevice");
        putDrum(d, "v0 Snare", "v0 Snare.bwdevice");
        putDrum(d, "v0 Tom", "v0 Tom.bwdevice");
        putDrum(d, "v0 Zap Kick", "v0 Zap Kick.bwdevice");

        putDrum(d, "v1 Clap", "v1 Clap.bwdevice");
        putDrum(d, "v1 Cowbell", "v1 Cowbell.bwdevice");
        putDrum(d, "v1 Hat", "v1 Hat.bwdevice");
        putDrum(d, "v1 Kick", "v1 Kick.bwdevice");
        putDrum(d, "v1 Snare", "v1 Snare.bwdevice");
        putDrum(d, "v1 Tom", "v1 Tom.bwdevice");

        putDrum(d, "v8 Clap", "v8 Clap.bwdevice");
        putDrum(d, "v8 Claves", "v8 Claves.bwdevice");
        putDrum(d, "v8 Cowbell", "v8 Cowbell.bwdevice");
        putDrum(d, "v8 Cymbal", "v8 Cymbal.bwdevice");
        putDrum(d, "v8 Hat", "v8 Hat.bwdevice");
        putDrum(d, "v8 Kick", "v8 Kick.bwdevice");
        putDrum(d, "v8 Maracas", "v8 Maracas.bwdevice");
        putDrum(d, "v8 Rimshot", "v8 Rimshot.bwdevice");
        putDrum(d, "v8 Snare", "v8 Snare.bwdevice");
        putDrum(d, "v8 Tom", "v8 Tom.bwdevice");

        putDrum(d, "v9 Clap", "v9 Clap.bwdevice");
        putDrum(d, "v9 Crash", "v9 Crash.bwdevice");
        putDrum(d, "v9 Hat Closed", "v9 Hat Closed.bwdevice");
        putDrum(d, "v9 Hat Open", "v9 Hat Open.bwdevice");
        putDrum(d, "v9 Kick", "v9 Kick.bwdevice");
        putDrum(d, "v9 Ride", "v9 Ride.bwdevice");
        putDrum(d, "v9 Rimshot", "v9 Rimshot.bwdevice");
        putDrum(d, "v9 Snare", "v9 Snare.bwdevice");
        putDrum(d, "v9 Tom", "v9 Tom.bwdevice");

        // WIGSCRIPT aliases → same files
        aliasDrum(d, "v0cymbal", "v0 Cymbal.bwdevice");
        aliasDrum(d, "v0hat", "v0 Hat.bwdevice");
        aliasDrum(d, "v0kick", "v0 Kick.bwdevice");
        aliasDrum(d, "v0snare", "v0 Snare.bwdevice");
        aliasDrum(d, "v0tom", "v0 Tom.bwdevice");
        aliasDrum(d, "v0zapkick", "v0 Zap Kick.bwdevice");
        aliasDrum(d, "kick", "v0 Kick.bwdevice");
        aliasDrum(d, "hh", "v0 Hat.bwdevice");
        aliasDrum(d, "hat", "v0 Hat.bwdevice");
        aliasDrum(d, "cymb", "v0 Cymbal.bwdevice");
        aliasDrum(d, "cymbal", "v0 Cymbal.bwdevice");
        aliasDrum(d, "cy", "v0 Cymbal.bwdevice");
        aliasDrum(d, "tom", "v0 Tom.bwdevice");
        aliasDrum(d, "zap", "v0 Zap Kick.bwdevice");
        aliasDrum(d, "zapkick", "v0 Zap Kick.bwdevice");
        aliasDrum(d, "v0zap", "v0 Zap Kick.bwdevice");

        aliasDrum(d, "v1clap", "v1 Clap.bwdevice");
        aliasDrum(d, "v1cowbell", "v1 Cowbell.bwdevice");
        aliasDrum(d, "v1hat", "v1 Hat.bwdevice");
        aliasDrum(d, "v1kick", "v1 Kick.bwdevice");
        aliasDrum(d, "v1snare", "v1 Snare.bwdevice");
        aliasDrum(d, "v1tom", "v1 Tom.bwdevice");
        aliasDrum(d, "v1perc", "v1 Cowbell.bwdevice"); // legacy alias

        aliasDrum(d, "v8clap", "v8 Clap.bwdevice");
        aliasDrum(d, "v8claves", "v8 Claves.bwdevice");
        aliasDrum(d, "v8cowbell", "v8 Cowbell.bwdevice");
        aliasDrum(d, "v8cymbal", "v8 Cymbal.bwdevice");
        aliasDrum(d, "v8hat", "v8 Hat.bwdevice");
        aliasDrum(d, "v8kick", "v8 Kick.bwdevice");
        aliasDrum(d, "v8maracas", "v8 Maracas.bwdevice");
        aliasDrum(d, "v8rimshot", "v8 Rimshot.bwdevice");
        aliasDrum(d, "v8rim", "v8 Rimshot.bwdevice");
        aliasDrum(d, "v8snare", "v8 Snare.bwdevice");
        aliasDrum(d, "v8tom", "v8 Tom.bwdevice");
        aliasDrum(d, "v8perc", "v8 Cowbell.bwdevice");
        aliasDrum(d, "v8cp", "v8 Clap.bwdevice");
        aliasDrum(d, "v8sn", "v8 Snare.bwdevice");
        aliasDrum(d, "v8hh", "v8 Hat.bwdevice");
        // family-only short names default to v8 (Rust device.rs parity)
        aliasDrum(d, "snare", "v8 Snare.bwdevice");
        aliasDrum(d, "sd", "v8 Snare.bwdevice");
        aliasDrum(d, "clap", "v8 Clap.bwdevice");
        aliasDrum(d, "cp", "v8 Clap.bwdevice");

        aliasDrum(d, "v9clap", "v9 Clap.bwdevice");
        aliasDrum(d, "v9crash", "v9 Crash.bwdevice");
        aliasDrum(d, "v9hatclosed", "v9 Hat Closed.bwdevice");
        aliasDrum(d, "v9hatopen", "v9 Hat Open.bwdevice");
        aliasDrum(d, "v9hat", "v9 Hat Closed.bwdevice"); // default closed
        aliasDrum(d, "v9hh", "v9 Hat Closed.bwdevice");
        aliasDrum(d, "v9kick", "v9 Kick.bwdevice");
        aliasDrum(d, "v9ride", "v9 Ride.bwdevice");
        aliasDrum(d, "v9rimshot", "v9 Rimshot.bwdevice");
        aliasDrum(d, "v9rim", "v9 Rimshot.bwdevice");
        aliasDrum(d, "v9snare", "v9 Snare.bwdevice");
        aliasDrum(d, "v9sn", "v9 Snare.bwdevice");
        aliasDrum(d, "v9tom", "v9 Tom.bwdevice");
        aliasDrum(d, "v9cp", "v9 Clap.bwdevice");
        // family-only short names (Rust device.rs parity)
        aliasDrum(d, "ride", "v9 Ride.bwdevice");
        aliasDrum(d, "rim", "v9 Rimshot.bwdevice");
        aliasDrum(d, "crash", "v9 Crash.bwdevice");
        aliasDrum(d, "hatopen", "v9 Hat Open.bwdevice");

        // type.variant (kick.v9, hat.v8, …)
        aliasDrum(d, "kick.v0", "v0 Kick.bwdevice");
        aliasDrum(d, "kick.v1", "v1 Kick.bwdevice");
        aliasDrum(d, "kick.v8", "v8 Kick.bwdevice");
        aliasDrum(d, "kick.v9", "v9 Kick.bwdevice");
        aliasDrum(d, "kick.808", "v8 Kick.bwdevice");
        aliasDrum(d, "kick.909", "v9 Kick.bwdevice");
        aliasDrum(d, "hat.v0", "v0 Hat.bwdevice");
        aliasDrum(d, "hat.v1", "v1 Hat.bwdevice");
        aliasDrum(d, "hat.v8", "v8 Hat.bwdevice");
        aliasDrum(d, "hat.v9", "v9 Hat Closed.bwdevice");
        aliasDrum(d, "hat.808", "v8 Hat.bwdevice");
        aliasDrum(d, "hat.909", "v9 Hat Closed.bwdevice");
        aliasDrum(d, "snare.v1", "v1 Snare.bwdevice");
        aliasDrum(d, "snare.v8", "v8 Snare.bwdevice");
        aliasDrum(d, "snare.v9", "v9 Snare.bwdevice");
        aliasDrum(d, "snare.808", "v8 Snare.bwdevice");
        aliasDrum(d, "snare.909", "v9 Snare.bwdevice");
        aliasDrum(d, "clap.v1", "v1 Clap.bwdevice");
        aliasDrum(d, "clap.v8", "v8 Clap.bwdevice");
        aliasDrum(d, "clap.v9", "v9 Clap.bwdevice");
        aliasDrum(d, "tom.v0", "v0 Tom.bwdevice");
        aliasDrum(d, "tom.v1", "v1 Tom.bwdevice");
        aliasDrum(d, "tom.v8", "v8 Tom.bwdevice");
        aliasDrum(d, "tom.v9", "v9 Tom.bwdevice");
        aliasDrum(d, "cymb.v0", "v0 Cymbal.bwdevice");
        aliasDrum(d, "cymb.v8", "v8 Cymbal.bwdevice");
        aliasDrum(d, "ride.v9", "v9 Ride.bwdevice");
        aliasDrum(d, "rim.v8", "v8 Rimshot.bwdevice");
        aliasDrum(d, "rim.v9", "v9 Rimshot.bwdevice");
        aliasDrum(d, "snare.v0", "v0 Snare.bwdevice");
        aliasDrum(d, "clap.808", "v8 Clap.bwdevice");
        aliasDrum(d, "clap.909", "v9 Clap.bwdevice");
        aliasDrum(d, "crash.v9", "v9 Crash.bwdevice");
        aliasDrum(d, "cymbal.v0", "v0 Cymbal.bwdevice");

        DRUM_FILES = Collections.unmodifiableMap(d);
    }

    private DeviceCatalog() {
    }

    private static void putUuid(final Map<String, UUID> m, final String name, final String uuid) {
        m.put(key(name), UUID.fromString(uuid));
    }

    private static void putDrum(final Map<String, String> m, final String bitwigName, final String file) {
        m.put(key(bitwigName), file);
    }

    private static void aliasDrum(final Map<String, String> m, final String aliasKey, final String file) {
        m.put(key(aliasKey), file);
    }

    /**
     * The one Java-side name normalization — keep in sync with Rust
     * {@code device::norm}: lowercase; strip space / {@code -} / {@code _} /
     * {@code .} ({@code kick.v9} legacy → {@code kickv9}); {@code +} → {@code plus}.
     */
    static String key(final String name) {
        return name.toLowerCase(Locale.ROOT)
                .replace(" ", "")
                .replace("-", "")
                .replace("_", "")
                .replace(".", "")
                .replace("+", "plus");
    }

    /**
     * Sampler / Drum Machine ban — the <b>single</b> Java-side definition,
     * enforced server-side (authoritative). Rust re-checks client-side
     * ({@code device::is_banned}) for early errors. Deliberate double guard,
     * same rule both sides.
     */
    public static boolean isBanned(final String name) {
        if (name == null) {
            return false;
        }
        final String k = key(name.trim());
        return k.contains("sampler") || k.contains("drummachine") || "dm".equals(k);
    }

    /** UUID devices (synths / layer / FX). {@code null} if not UUID-curated. */
    public static UUID resolveUuid(final String name) {
        if (name == null || name.isBlank()) {
            return null;
        }
        final String trimmed = name.trim();
        try {
            return UUID.fromString(trimmed);
        } catch (final IllegalArgumentException ignored) {
            // fall through
        }
        final String k = key(trimmed);
        if ("delay2".equals(k) || "delay1".equals(k) || "dly2".equals(k) || "dly1".equals(k)
                || "delay".equals(k)) {
            return BY_UUID.get(key("Delay+"));
        }
        if ("chorus".equals(k) || "chor".equals(k)) {
            return BY_UUID.get(key("Chorus+"));
        }
        if ("dist".equals(k) || "distortion".equals(k)) {
            return BY_UUID.get(key("Saturator"));
        }
        if ("layer".equals(k) || "instrumentlayer".equals(k)) {
            return BY_UUID.get(key("Instrument Layer"));
        }
        if ("poly".equals(k)) {
            return BY_UUID.get(key("Polymer"));
        }
        if ("psynth".equals(k)) {
            return BY_UUID.get(key("Polysynth"));
        }
        if ("filt".equals(k)) {
            return BY_UUID.get(key("Filter"));
        }
        if ("rev".equals(k)) {
            return BY_UUID.get(key("Reverb"));
        }
        return BY_UUID.get(k);
    }

    /**
     * Absolute path to a stock drum {@code .bwdevice}, or {@code null}.
     * Never resolves Sampler / Drum Machine.
     */
    public static String resolveDrumFile(final String name) {
        if (name == null || name.isBlank()) {
            return null;
        }
        if (isBanned(name)) {
            return null;
        }
        final String k = key(name.trim());
        final String file = DRUM_FILES.get(k);
        if (file == null) {
            return null;
        }
        final Path dir = devicesLibraryDir();
        if (dir == null) {
            return null;
        }
        final Path full = dir.resolve(file);
        return Files.isRegularFile(full) ? full.toAbsolutePath().toString() : null;
    }

    /**
     * Resolve any library {@code .bwdevice} by display name / file stem
     * (e.g. {@code "EQ+"} → {@code EQ+.bwdevice} under Library/devices).
     */
    public static String resolveLibraryDeviceFile(final String name) {
        if (name == null || name.isBlank()) {
            return null;
        }
        final String trimmed = name.trim();
        if (isBanned(trimmed)) {
            return null;
        }
        final Path dir = devicesLibraryDir();
        if (dir == null) {
            return null;
        }
        // Absolute / relative path already pointing at a file
        final Path asPath = Paths.get(trimmed);
        if (Files.isRegularFile(asPath)) {
            return asPath.toAbsolutePath().toString();
        }
        final String stem = trimmed.endsWith(".bwdevice")
                ? trimmed.substring(0, trimmed.length() - ".bwdevice".length())
                : trimmed;
        final Path exact = dir.resolve(stem + ".bwdevice");
        if (Files.isRegularFile(exact)) {
            return exact.toAbsolutePath().toString();
        }
        // Case-insensitive scan of top-level library devices
        try (var stream = Files.list(dir)) {
            final String want = key(stem);
            final var match = stream
                    .filter(Files::isRegularFile)
                    .filter(p -> {
                        final String fn = p.getFileName().toString();
                        if (!fn.toLowerCase(Locale.ROOT).endsWith(".bwdevice")) {
                            return false;
                        }
                        final String base = fn.substring(0, fn.length() - ".bwdevice".length());
                        return key(base).equals(want);
                    })
                    .findFirst();
            return match.map(p -> p.toAbsolutePath().toString()).orElse(null);
        } catch (final Exception e) {
            logScanFailureOnce(dir, e);
            return null;
        }
    }

    /** Optional host for one-time scan error logging (set from extension init). */
    private static volatile ControllerHost host;
    private static volatile boolean scanFailureLogged;

    public static void setHost(final ControllerHost h) {
        host = h;
    }

    /** Library scan errors were silent — log once per extension lifetime. */
    private static void logScanFailureOnce(final Path dir, final Exception e) {
        if (scanFailureLogged) {
            return;
        }
        scanFailureLogged = true;
        final ControllerHost h = host;
        if (h != null) {
            h.errorln("Codewig: library device scan failed for " + dir + ": " + e);
        }
    }

    /** UUID, known drum, or library .bwdevice. */
    public static boolean isAllowed(final String name) {
        return resolveUuid(name) != null
                || resolveDrumFile(name) != null
                || resolveLibraryDeviceFile(name) != null;
    }

    /**
     * Bitwig stock devices folder. Override with env {@code BITWIG_DEVICES}.
     * Resolved once per extension lifetime (result incl. {@code null} cached —
     * scanning the filesystem on every resolve was wasteful).
     */
    private static Path cachedDevicesDir;
    private static boolean devicesDirResolved;

    static synchronized Path devicesLibraryDir() {
        if (devicesDirResolved) {
            return cachedDevicesDir;
        }
        devicesDirResolved = true;
        cachedDevicesDir = findDevicesLibraryDir();
        return cachedDevicesDir;
    }

    private static Path findDevicesLibraryDir() {
        final String env = System.getenv("BITWIG_DEVICES");
        if (env != null && !env.isBlank()) {
            final Path p = Paths.get(env.trim());
            if (Files.isDirectory(p)) {
                return p;
            }
        }
        final String[] candidates = {
                "C:\\Program Files\\Bitwig Studio\\Library\\devices",
                "/Applications/Bitwig Studio.app/Contents/Resources/Library/devices",
                System.getProperty("user.home") + "/Bitwig Studio/Library/devices",
        };
        for (final String c : candidates) {
            final Path p = Paths.get(c);
            if (Files.isDirectory(p)) {
                return p;
            }
        }
        // Versioned install dirs: "Bitwig Studio 5.3" etc.
        final File pf = new File("C:\\Program Files");
        if (pf.isDirectory()) {
            final File[] kids = pf.listFiles((dir, n) -> n.startsWith("Bitwig Studio"));
            if (kids != null) {
                for (final File k : kids) {
                    final Path dev = k.toPath().resolve("Library").resolve("devices");
                    if (Files.isDirectory(dev)) {
                        return dev;
                    }
                }
            }
        }
        return null;
    }
}
