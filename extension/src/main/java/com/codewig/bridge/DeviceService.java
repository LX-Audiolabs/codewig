package com.codewig.bridge;

import java.util.UUID;

import com.bitwig.extension.controller.api.ControllerHost;
import com.bitwig.extension.controller.api.CursorDevice;
import com.bitwig.extension.controller.api.CursorTrack;
import com.bitwig.extension.controller.api.Device;
import com.bitwig.extension.controller.api.DeviceBank;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;

/**
 * Devices on the cursor track (chain end insert by default).
 */
public final class DeviceService {
    public static final int BANK_SIZE = 32;

    private final CursorTrack cursorTrack;
    private final CursorDevice cursorDevice;
    private final DeviceBank deviceBank;

    public DeviceService(final ControllerHost host, final CursorTrack cursorTrack) {
        this.cursorTrack = cursorTrack;
        this.cursorDevice = cursorTrack.createCursorDevice("CODEWIG_DEVICE", "Codewig Device", 0,
                com.bitwig.extension.controller.api.CursorDeviceFollowMode.FOLLOW_SELECTION);
        this.deviceBank = cursorTrack.createDeviceBank(BANK_SIZE);

        cursorDevice.exists().markInterested();
        cursorDevice.name().markInterested();
        cursorDevice.position().markInterested();

        for (int i = 0; i < BANK_SIZE; i++) {
            final Device d = deviceBank.getItemAt(i);
            d.exists().markInterested();
            d.name().markInterested();
            d.position().markInterested();
        }
    }

    public JsonObject list() {
        requireTrack();
        deviceBank.scrollPosition().set(0);
        final JsonArray devices = new JsonArray();
        for (int i = 0; i < BANK_SIZE; i++) {
            final Device d = deviceBank.getItemAt(i);
            if (!d.exists().get()) {
                continue;
            }
            final JsonObject o = new JsonObject();
            o.addProperty("index", d.position().get());
            o.addProperty("name", d.name().get());
            devices.add(o);
        }
        final JsonObject result = new JsonObject();
        result.add("devices", devices);
        result.addProperty("count", devices.size());
        result.addProperty("track", cursorTrack.name().get());
        if (cursorDevice.exists().get()) {
            result.addProperty("selected", cursorDevice.name().get());
            result.addProperty("selectedIndex", cursorDevice.position().get());
        }
        return result;
    }

    public JsonObject add(final String deviceName) {
        requireTrack();
        if (deviceName == null || deviceName.isBlank()) {
            throw new IllegalArgumentException("device name empty");
        }
        final String name = deviceName.trim();
        // Server-side authoritative guard (single definition in DeviceCatalog);
        // the Rust client checks the same rule earlier (device::is_banned).
        if (DeviceCatalog.isBanned(name)) {
            throw new IllegalArgumentException(
                    "device '" + name + "' not insertable (Sampler / Drum Machine out of scope)");
        }

        final var insert = cursorTrack.endOfDeviceChainInsertionPoint();
        final JsonObject result = new JsonObject();
        result.addProperty("added", name);
        result.addProperty("track", cursorTrack.name().get());

        // 1) Known UUID map + raw UUID string
        final UUID uuid = DeviceCatalog.resolveUuid(name);
        if (uuid != null) {
            insert.insertBitwigDevice(uuid);
            result.addProperty("uuid", uuid.toString());
            result.addProperty("via", "uuid");
            return result;
        }

        // 2) Known drum / library alias map
        final String file = DeviceCatalog.resolveDrumFile(name);
        if (file != null) {
            insert.insertFile(file);
            result.addProperty("file", file);
            result.addProperty("via", "file");
            return result;
        }

        // 3) Open insert: any Bitwig library .bwdevice matching the display name
        final String libraryFile = DeviceCatalog.resolveLibraryDeviceFile(name);
        if (libraryFile != null) {
            insert.insertFile(libraryFile);
            result.addProperty("file", libraryFile);
            result.addProperty("via", "library");
            return result;
        }

        throw new IllegalArgumentException(
                "cannot insert '" + name
                        + "' — no UUID mapping and no matching .bwdevice in Bitwig Library/devices. "
                        + "Pass a Bitwig device name, library file stem, or raw UUID. "
                        + "Not allowed: Sampler, Drum Machine.");
    }

    public JsonObject select(final int index) {
        requireTrack();
        deviceBank.scrollPosition().set(0);
        final Device d = findByIndex(index);
        d.selectInEditor();
        final JsonObject result = new JsonObject();
        result.addProperty("index", d.position().get());
        result.addProperty("name", d.name().get());
        return result;
    }

    public JsonObject delete(final int index) {
        requireTrack();
        final Device d = findByIndex(index);
        final String name = d.name().get();
        d.deleteObject();
        final JsonObject result = new JsonObject();
        result.addProperty("deleted", name);
        result.addProperty("index", index);
        return result;
    }

    public CursorDevice getCursorDevice() {
        return cursorDevice;
    }

    private Device findByIndex(final int index) {
        deviceBank.scrollPosition().set(0);
        for (int i = 0; i < BANK_SIZE; i++) {
            final Device d = deviceBank.getItemAt(i);
            if (d.exists().get() && d.position().get() == index) {
                return d;
            }
        }
        // fallback: bank slot index
        if (index >= 0 && index < BANK_SIZE) {
            final Device d = deviceBank.getItemAt(index);
            if (d.exists().get()) {
                return d;
            }
        }
        throw new IllegalArgumentException("no device at index " + index);
    }

    private void requireTrack() {
        if (!cursorTrack.exists().get()) {
            throw new IllegalArgumentException("no track selected");
        }
    }
}
