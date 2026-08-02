package com.codewig.bridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.UUID;
import org.junit.jupiter.api.Test;

/**
 * Host-free parts of {@link DeviceCatalog}: key normalization, the Sampler /
 * Drum Machine ban, and UUID/alias resolution. Filesystem-backed resolution
 * ({@code resolveDrumFile} / {@code resolveLibraryDeviceFile}) is only
 * exercised where it must return {@code null} without touching the disk.
 */
class DeviceCatalogTest {

    // --- key() normalization ----------------------------------------------------

    @Test
    void keyLowercasesAndStripsSeparators() {
        assertEquals("v9kick", DeviceCatalog.key("v9 Kick"));
        assertEquals("v9kick", DeviceCatalog.key("V9-KICK"));
        assertEquals("v9kick", DeviceCatalog.key("v9_kick"));
        assertEquals("kickv9", DeviceCatalog.key("kick.v9")); // legacy dotted form
        assertEquals("v9hatopen", DeviceCatalog.key("v9 Hat Open"));
    }

    @Test
    void keyMapsPlusToPlus() {
        assertEquals("delayplus", DeviceCatalog.key("Delay+"));
        assertEquals("chorusplus", DeviceCatalog.key("Chorus+"));
    }

    // --- isBanned() --------------------------------------------------------------

    @Test
    void bannedDevices() {
        assertTrue(DeviceCatalog.isBanned("Sampler"));
        assertTrue(DeviceCatalog.isBanned("sampler"));
        assertTrue(DeviceCatalog.isBanned("The Sampler"));
        assertTrue(DeviceCatalog.isBanned("Drum Machine"));
        assertTrue(DeviceCatalog.isBanned("drummachine"));
        assertTrue(DeviceCatalog.isBanned("Drum-Machine"));
        assertTrue(DeviceCatalog.isBanned("DM"));
        assertTrue(DeviceCatalog.isBanned("dm"));
    }

    @Test
    void allowedDevicesAreNotBanned() {
        assertFalse(DeviceCatalog.isBanned(null));
        assertFalse(DeviceCatalog.isBanned("Polymer"));
        assertFalse(DeviceCatalog.isBanned("v9 Kick"));
        assertFalse(DeviceCatalog.isBanned("Delay+"));
    }

    // --- resolveUuid() -------------------------------------------------------------

    @Test
    void rawUuidPassesThrough() {
        final UUID id = UUID.fromString("8f58138b-03aa-4e9d-83bd-a038c99a4ed5");
        assertEquals(id, DeviceCatalog.resolveUuid("8f58138b-03aa-4e9d-83bd-a038c99a4ed5"));
    }

    @Test
    void curatedNamesAndAliasesResolve() {
        final UUID polymer = UUID.fromString("8f58138b-03aa-4e9d-83bd-a038c99a4ed5");
        assertEquals(polymer, DeviceCatalog.resolveUuid("Polymer"));
        assertEquals(polymer, DeviceCatalog.resolveUuid("poly"));
        assertEquals(polymer, DeviceCatalog.resolveUuid("poly mer")); // normalized

        final UUID delayPlus = UUID.fromString("f2baa2a8-36c5-4a79-b1d9-a4e461c45ee9");
        assertEquals(delayPlus, DeviceCatalog.resolveUuid("Delay+"));
        assertEquals(delayPlus, DeviceCatalog.resolveUuid("delay"));
        assertEquals(delayPlus, DeviceCatalog.resolveUuid("delay2"));

        final UUID saturator = UUID.fromString("93d11348-86ae-4ead-9fe7-84ac03b9369c");
        assertEquals(saturator, DeviceCatalog.resolveUuid("dist"));
    }

    @Test
    void unknownAndEmptyNamesResolveToNull() {
        assertNull(DeviceCatalog.resolveUuid(null));
        assertNull(DeviceCatalog.resolveUuid("   "));
        assertNull(DeviceCatalog.resolveUuid("no-such-device-xyz"));
    }

    // --- drum file resolution: only disk-independent outcomes ----------------------

    @Test
    void drumFileResolutionRejectsBannedAndUnknownWithoutDisk() {
        assertNull(DeviceCatalog.resolveDrumFile(null));
        assertNull(DeviceCatalog.resolveDrumFile("Sampler"));
        assertNull(DeviceCatalog.resolveDrumFile("Drum Machine"));
        assertNull(DeviceCatalog.resolveDrumFile("no-such-drum-xyz"));
    }
}
